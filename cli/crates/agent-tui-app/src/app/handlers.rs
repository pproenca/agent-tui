#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI command handlers.

use std::collections::HashMap;
use std::io;
use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

use crate::adapters::RpcValue;
use crate::adapters::RpcValueRef;
use crate::adapters::rpc::params;
use crate::common::Colors;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;
use crate::infra::ipc::ProcessController;
use crate::infra::ipc::Signal;
use crate::infra::ipc::UnixProcessController;
use crate::infra::ipc::socket_path;

use crate::adapters::presenter::ClientErrorView;
use crate::adapters::presenter::Presenter;
use crate::adapters::presenter::create_presenter;
use crate::app::attach::DetachKeys;
use crate::app::commands::LiveStartArgs;
use crate::app::commands::OutputFormat;
use crate::app::commands::WaitParams;
use crate::app::error::AttachError;
use crate::app::error::CliError;
use crate::app::rpc_client::call_no_params;
use crate::app::rpc_client::call_with_params;

pub(crate) type HandlerResult = Result<()>;

fn client_error_view(error: &ClientError) -> ClientErrorView {
    ClientErrorView {
        message: error.to_string(),
        suggestion: error.suggestion().map(str::to_string),
        retryable: error.is_retryable(),
        json: Some(error.to_json_string()),
    }
}

macro_rules! key_handler {
    ($name:ident, $method:literal, $success:expr) => {
        pub(crate) fn $name<C: DaemonClient>(
            ctx: &mut HandlerContext<C>,
            key: String,
        ) -> HandlerResult {
            let success_message = match ctx.format {
                OutputFormat::Text => Some($success(&key)),
                OutputFormat::Json => None,
            };
            let params = params::KeyParams {
                key,
                session: ctx.session.clone(),
            };
            let result = call_with_params(ctx.client, $method, params)?;
            if let Some(success_message) = success_message {
                ctx.output_success_and_ok(&result, &success_message, concat!($method, " failed"))
            } else {
                ctx.output_success_and_ok(&result, "", concat!($method, " failed"))
            }
        }
    };
}

pub(crate) fn resolve_wait_condition(params: &WaitParams) -> Option<String> {
    if params.stable {
        return Some("stable".to_string());
    }

    if params.text.is_some() && params.gone {
        return Some("text_gone".to_string());
    }

    None
}

pub(crate) struct HandlerContext<'a, C: DaemonClient> {
    pub client: &'a mut C,
    pub session: Option<String>,
    pub format: OutputFormat,
    presenter: Box<dyn Presenter>,
}

impl<'a, C: DaemonClient> HandlerContext<'a, C> {
    pub fn new(client: &'a mut C, session: Option<String>, format: OutputFormat) -> Self {
        let presenter = create_presenter(&format);
        Self {
            client,
            session,
            format,
            presenter,
        }
    }

    pub fn presenter(&self) -> &dyn Presenter {
        self.presenter.as_ref()
    }

    pub fn output_success_result(
        &self,
        result: &RpcValue,
        success_msg: &str,
        failure_prefix: &str,
    ) -> Result<bool> {
        let success = result.bool_or("success", false);

        match self.format {
            OutputFormat::Json => {
                if success {
                    self.presenter.present_value(result);
                } else {
                    let msg = result.str_or("message", "Unknown error");
                    let message = format!("{}: {}", failure_prefix, msg);
                    return Err(CliError::new(
                        self.format,
                        message,
                        Some(result.to_pretty_json()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
            OutputFormat::Text => {
                if success {
                    let warning = result.get("warning").and_then(|w| w.as_str());
                    self.presenter.present_success(success_msg, warning);
                } else {
                    let msg = result.str_or("message", "Unknown error");
                    let message = format!("{}: {}", failure_prefix, msg);
                    return Err(CliError::new(
                        self.format,
                        message,
                        Some(result.to_pretty_json()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
        }
        Ok(true)
    }

    fn output_json_or<F>(&self, result: &RpcValue, text_fn: F) -> HandlerResult
    where
        F: FnOnce(),
    {
        match self.format {
            OutputFormat::Json => {
                self.presenter.present_value(result);
            }
            OutputFormat::Text => {
                text_fn();
            }
        }
        Ok(())
    }

    pub fn output_success_and_ok(
        &self,
        result: &RpcValue,
        success_msg: &str,
        failure_prefix: &str,
    ) -> HandlerResult {
        self.output_success_result(result, success_msg, failure_prefix)?;
        Ok(())
    }

    pub fn display_error(&self, error: &ClientError) {
        let view = client_error_view(error);
        self.presenter.present_client_error(&view);
    }
}

pub(crate) fn handle_spawn<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
    let cwd = cwd.map(|path| path.to_string_lossy().into_owned());
    let rpc_params = params::SpawnParams {
        command,
        args,
        cwd,
        session: ctx.session.clone(),
        cols,
        rows,
    };
    let result = call_with_params(ctx.client, "spawn", rpc_params)?;

    ctx.output_json_or(&result, || {
        let session_id = result.str_or("session_id", "unknown");
        let pid = result.u64_or("pid", 0);
        println!(
            "{} {}",
            Colors::success("Session started:"),
            Colors::session_id(session_id)
        );
        println!("  PID: {}", pid);
    })
}

pub(crate) fn handle_snapshot<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    region: Option<String>,
    strip_ansi: bool,
    include_cursor: bool,
) -> HandlerResult {
    let rpc_params = params::SnapshotParams {
        session: ctx.session.clone(),
        region,
        strip_ansi,
        include_cursor,
        include_render: false,
    };
    let result = call_with_params(ctx.client, "snapshot", rpc_params)?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", result.to_pretty_json());
        }
        OutputFormat::Text => {
            println!("{}", Colors::bold("Screenshot:"));
            if let Some(screenshot) = result.get("screenshot").and_then(|v| v.as_str()) {
                println!("{}", screenshot);
            }
            if include_cursor {
                if let Some(cursor) = result.get("cursor") {
                    let row = cursor.u64_or("row", 0);
                    let col = cursor.u64_or("col", 0);
                    let visible = cursor.bool_or("visible", false);
                    let vis_str = if visible { "visible" } else { "hidden" };
                    println!("\nCursor: row={}, col={} ({})", row, col, vis_str);
                } else {
                    eprintln!("Warning: Cursor position requested but not available from session");
                }
            }
        }
    }
    Ok(())
}

key_handler!(handle_press, "keystroke", |_: &String| "Key pressed"
    .to_string());
key_handler!(handle_keydown, "keydown", |k: &String| format!(
    "Key held: {}",
    k
));
key_handler!(handle_keyup, "keyup", |k: &String| format!(
    "Key released: {}",
    k
));

pub(crate) fn handle_type<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    text: String,
) -> HandlerResult {
    let params = params::TypeParams {
        text,
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "type", params)?;
    ctx.output_success_and_ok(&result, "Text typed", "Type failed")
}

pub(crate) fn handle_wait<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    wait_params: WaitParams,
) -> HandlerResult {
    use crate::adapters::presenter::WaitResult;

    let cond = resolve_wait_condition(&wait_params);
    let WaitParams {
        text,
        timeout,
        assert,
        ..
    } = wait_params;
    let rpc_params = params::WaitParams {
        session: ctx.session.clone(),
        text,
        timeout_ms: timeout,
        condition: cond,
    };
    let result = call_with_params(ctx.client, "wait", rpc_params)?;

    let wait_result = WaitResult::from_json(&result);

    if assert && !wait_result.found {
        return Err(CliError::new(
            ctx.format,
            "Wait condition not met within timeout",
            Some(result.to_pretty_json()),
            super::exit_codes::GENERAL_ERROR,
        )
        .into());
    }

    match ctx.format {
        OutputFormat::Json => ctx.presenter().present_value(&result),
        OutputFormat::Text => ctx.presenter().present_wait_result(&wait_result),
    }
    Ok(())
}

pub(crate) fn handle_kill<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let params = params::SessionParams {
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "kill", params)?;

    ctx.output_json_or(&result, || {
        println!("Session {} killed", result.str_or("session_id", "unknown"));
    })
}

pub(crate) fn handle_restart<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let params = params::SessionParams {
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "restart", params)?;

    ctx.output_json_or(&result, || {
        println!(
            "Restarted '{}': {} -> {}",
            result.str_or("command", "unknown"),
            result.str_or("old_session_id", "unknown"),
            result.str_or("new_session_id", "unknown")
        );
    })
}

pub(crate) fn handle_sessions<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = call_no_params(ctx.client, "sessions")?;

    ctx.output_json_or(&result, || {
        let active_id = result.get("active_session").and_then(|v| v.as_str());

        match result.get("sessions").and_then(|v| v.as_array()) {
            Some(sessions) if !sessions.is_empty() => {
                println!("{}", Colors::bold("Active sessions:"));
                for session in sessions.iter() {
                    let id = session.str_or("id", "?");
                    let command = session.str_or("command", "?");
                    let pid = session.u64_or("pid", 0);
                    let running = session.bool_or("running", false);
                    let cols = session
                        .get("size")
                        .and_then(|s| s.get("cols"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let rows = session
                        .get("size")
                        .and_then(|s| s.get("rows"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    let is_active = active_id == Some(id);
                    let active = if is_active {
                        Colors::success(" (active)")
                    } else {
                        String::new()
                    };
                    let status = if running {
                        Colors::success("running")
                    } else {
                        Colors::error("stopped")
                    };

                    println!(
                        "  {} - {} [{}] {}x{} pid:{}{}",
                        Colors::session_id(id),
                        command,
                        status,
                        cols,
                        rows,
                        pid,
                        active
                    );
                }
            }
            _ => {
                println!("{}", Colors::dim("No active sessions"));
            }
        }
    })
}

pub(crate) fn handle_session_show<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
) -> HandlerResult {
    let result = call_no_params(ctx.client, "sessions")?;
    let active_id = result.get("active_session").and_then(|v| v.as_str());
    let sessions = result
        .get("sessions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid sessions response"))?;

    let session = sessions
        .iter()
        .find(|session| session.str_or("id", "") == session_id.as_str())
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    match ctx.format {
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct SessionShow<'a> {
                session: RpcValueRef<'a>,
                active_session: Option<&'a str>,
            }
            let payload = SessionShow {
                session,
                active_session: active_id,
            };
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
        OutputFormat::Text => {
            let id = session.str_or("id", "?");
            let command = session.str_or("command", "?");
            let pid = session.u64_or("pid", 0);
            let running = session.bool_or("running", false);
            let cols = session
                .get("size")
                .and_then(|s| s.get("cols"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let rows = session
                .get("size")
                .and_then(|s| s.get("rows"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let created_at = session.get("created_at").and_then(|v| v.as_str());

            let is_active = active_id == Some(id);
            let active = if is_active {
                Colors::success(" (active)")
            } else {
                String::new()
            };
            let status = if running {
                Colors::success("running")
            } else {
                Colors::error("stopped")
            };

            println!(
                "{} {}{}",
                Colors::bold("Session:"),
                Colors::session_id(id),
                active
            );
            println!("  Command: {}", command);
            println!("  Status: {}", status);
            println!("  Size: {}x{}", cols, rows);
            println!("  PID: {}", pid);
            if let Some(created) = created_at {
                println!("  Created: {}", created);
            }
        }
    }

    Ok(())
}

pub(crate) fn resolve_attach_session_id<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
) -> Result<String> {
    if let Some(id) = ctx.session.clone() {
        return Ok(id);
    }

    let result = call_no_params(ctx.client, "sessions")?;
    if let Some(active) = result.get("active_session").and_then(|v| v.as_str()) {
        return Ok(active.to_string());
    }

    Err(anyhow::anyhow!(
        "No active session to attach. Use 'agent-tui sessions list' then 'agent-tui sessions switch <id>' or pass --session."
    ))
}

pub(crate) fn handle_session_switch<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
) -> HandlerResult {
    let params = params::SessionParams {
        session: Some(session_id.clone()),
    };
    let result = call_with_params(ctx.client, "attach", params)?;
    let success_message = format!("Active session set to {}", Colors::session_id(&session_id));
    ctx.output_success_and_ok(&result, &success_message, "Switch failed")
}

pub(crate) fn handle_live_start<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    args: LiveStartArgs,
) -> HandlerResult {
    if args.listen.is_some() || args.allow_remote || args.max_viewers.is_some() {
        eprintln!(
            "{} Live preview is now served by the daemon WebSocket server. Configure it via:",
            Colors::info("Note:")
        );
        eprintln!(
            "  AGENT_TUI_WS_LISTEN / AGENT_TUI_WS_ALLOW_REMOTE / AGENT_TUI_WS_MAX_CONNECTIONS"
        );
    }

    let state_path = ws_state_path();
    let state = wait_for_api_state(&state_path, Duration::from_secs(3)).ok_or_else(|| {
        CliError::new(
            ctx.format,
            "WebSocket live preview is not available. Restart the daemon and try again."
                .to_string(),
            None,
            super::exit_codes::GENERAL_ERROR,
        )
    })?;
    let daemon_ui_url = state.resolved_ui_url();
    let open_base_url = std::env::var("AGENT_TUI_UI_URL")
        .ok()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| daemon_ui_url.clone());

    match ctx.format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct LiveStartOutput {
                running: bool,
                pid: u32,
                listen: String,
                ws_url: String,
                ui_url: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                started_at: Option<u64>,
            }

            let output = LiveStartOutput {
                running: true,
                pid: state.pid,
                listen: state.listen.clone(),
                ws_url: state.ws_url.clone(),
                ui_url: daemon_ui_url,
                started_at: state.started_at,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("WS: {}", state.ws_url);
            println!("UI: {}", daemon_ui_url);
        }
    }

    if args.open {
        let target = build_ui_url(&open_base_url, &state);
        if let Err(err) = open_in_browser(&target, args.browser.as_deref()) {
            eprintln!("Warning: failed to open browser: {}", err);
        }
    }

    Ok(())
}

pub(crate) fn handle_live_stop<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    handle_live_stop_standalone(ctx.format)
}

pub(crate) fn handle_live_stop_standalone(format: OutputFormat) -> HandlerResult {
    let ui_result = stop_ui_server();
    let ui_error = ui_result.as_ref().err().map(|err| err.to_string());
    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct UiStopPayload {
                stopped: bool,
                #[serde(skip_serializing_if = "Option::is_none")]
                reason: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                error: Option<String>,
            }

            #[derive(Serialize)]
            struct LiveStopOutput {
                stopped: bool,
                reason: String,
                ui: UiStopPayload,
            }

            let ui_payload = match ui_result {
                Ok(StopUiResult::Stopped) => UiStopPayload {
                    stopped: true,
                    reason: None,
                    error: None,
                },
                Ok(StopUiResult::AlreadyStopped) => UiStopPayload {
                    stopped: false,
                    reason: Some("ui not running".to_string()),
                    error: None,
                },
                Ok(StopUiResult::External) => UiStopPayload {
                    stopped: false,
                    reason: Some("ui managed externally".to_string()),
                    error: None,
                },
                Err(err) => UiStopPayload {
                    stopped: false,
                    reason: None,
                    error: Some(err.to_string()),
                },
            };

            let output = LiveStopOutput {
                stopped: false,
                reason: "live preview is served by the daemon; stop the daemon to stop".to_string(),
                ui: ui_payload,
            };
            if let Some(err) = ui_error {
                return Err(CliError::new(
                    format,
                    format!("Failed to stop UI server: {}", err),
                    Some(serde_json::to_string_pretty(&output)?),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            match ui_result {
                Ok(StopUiResult::Stopped) => {
                    println!("UI server stopped.");
                }
                Ok(StopUiResult::AlreadyStopped) => {
                    println!("UI server is not running.");
                }
                Ok(StopUiResult::External) => {
                    println!("UI server is managed externally (AGENT_TUI_UI_URL).");
                }
                Err(err) => {
                    eprintln!(
                        "{} Failed to stop UI server: {}",
                        Colors::warning("Warning:"),
                        err
                    );
                }
            }
            println!("Live preview is served by the daemon; run 'agent-tui daemon stop' to stop.");
        }
    }
    if let Some(err) = ui_error {
        return Err(CliError::new(
            format,
            format!("Failed to stop UI server: {}", err),
            None,
            super::exit_codes::GENERAL_ERROR,
        )
        .into());
    }
    Ok(())
}

pub(crate) fn handle_live_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    handle_live_status_standalone(ctx.format)
}

pub(crate) fn handle_live_status_standalone(format: OutputFormat) -> HandlerResult {
    let status = read_api_state_running(&ws_state_path());
    let ui_status = resolve_ui_status();

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct UiStatusPayload {
                #[serde(skip_serializing_if = "Option::is_none")]
                running: Option<bool>,
                #[serde(skip_serializing_if = "Option::is_none")]
                url: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                managed: Option<bool>,
                #[serde(skip_serializing_if = "Option::is_none")]
                pid: Option<u32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                port: Option<u16>,
            }

            #[derive(Serialize)]
            struct LiveStatusOutput {
                running: bool,
                #[serde(skip_serializing_if = "Option::is_none")]
                pid: Option<u32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                listen: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                ws_url: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                ui_url: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                started_at: Option<u64>,
                ui: UiStatusPayload,
            }

            let output = match status {
                Some(state) => {
                    let daemon_ui_url = state.resolved_ui_url();
                    let daemon_ui_port = parse_port_from_url(&daemon_ui_url);
                    let ui = match ui_status {
                        UiStatus::External(url) => UiStatusPayload {
                            running: Some(true),
                            url: Some(url),
                            managed: Some(false),
                            pid: None,
                            port: None,
                        },
                        UiStatus::Running(state) => UiStatusPayload {
                            running: Some(true),
                            url: Some(state.url),
                            managed: Some(true),
                            pid: Some(state.pid),
                            port: Some(state.port),
                        },
                        UiStatus::NotRunning => UiStatusPayload {
                            running: Some(true),
                            url: Some(daemon_ui_url.clone()),
                            managed: Some(true),
                            pid: Some(state.pid),
                            port: daemon_ui_port,
                        },
                    };

                    LiveStatusOutput {
                        ui_url: Some(daemon_ui_url),
                        running: true,
                        pid: Some(state.pid),
                        listen: Some(state.listen),
                        ws_url: Some(state.ws_url),
                        started_at: state.started_at,
                        ui,
                    }
                }
                None => LiveStatusOutput {
                    running: false,
                    pid: None,
                    listen: None,
                    ws_url: None,
                    ui_url: None,
                    started_at: None,
                    ui: match ui_status {
                        UiStatus::External(url) => UiStatusPayload {
                            running: None,
                            url: Some(url),
                            managed: Some(false),
                            pid: None,
                            port: None,
                        },
                        UiStatus::Running(state) => UiStatusPayload {
                            running: None,
                            url: Some(state.url),
                            managed: Some(true),
                            pid: Some(state.pid),
                            port: Some(state.port),
                        },
                        UiStatus::NotRunning => UiStatusPayload {
                            running: Some(false),
                            url: None,
                            managed: None,
                            pid: None,
                            port: None,
                        },
                    },
                },
            };

            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            if let Some(state) = status {
                println!("Live preview WS: {}", state.ws_url);
                println!("Live preview UI: {}", state.resolved_ui_url());
                if let UiStatus::External(url) = ui_status {
                    println!("UI override: {} (external)", url);
                }
            } else {
                println!("Live preview: not running");
                match ui_status {
                    UiStatus::External(url) => {
                        println!("UI: {} (external)", url);
                    }
                    UiStatus::Running(state) => {
                        println!("UI: {}", state.url);
                    }
                    UiStatus::NotRunning => {
                        println!("UI: not running");
                    }
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn handle_resize<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
    let rpc_params = params::ResizeParams {
        cols,
        rows,
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "resize", rpc_params)?;

    ctx.output_json_or(&result, || {
        println!(
            "Session {} resized to {}x{}",
            Colors::session_id(result.str_or("session_id", "?")),
            cols,
            rows
        );
    })
}

pub(crate) fn handle_version_standalone(format: OutputFormat) -> HandlerResult {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");

    let (daemon_version, daemon_commit, daemon_error) =
        match crate::infra::ipc::UnixSocketClient::connect() {
            Ok(mut client) => match call_no_params(&mut client, "version") {
                Ok(result) => (
                    result
                        .get("daemon_version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    result
                        .get("daemon_commit")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    None,
                ),
                Err(e) => (
                    "unavailable".to_string(),
                    "unknown".to_string(),
                    Some(e.to_string()),
                ),
            },
            Err(e) => (
                "unavailable".to_string(),
                "unknown".to_string(),
                Some(e.to_string()),
            ),
        };

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct VersionOutput {
                cli_version: &'static str,
                cli_commit: &'static str,
                daemon_version: String,
                daemon_commit: String,
                mode: &'static str,
                #[serde(skip_serializing_if = "Option::is_none")]
                daemon_error: Option<String>,
            }

            let output = VersionOutput {
                cli_version,
                cli_commit,
                daemon_version,
                daemon_commit,
                mode: "daemon",
                daemon_error,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("{}", Colors::bold("agent-tui"));
            println!("  CLI version: {}", cli_version);
            println!("  CLI commit: {}", cli_commit);
            if let Some(err) = &daemon_error {
                println!(
                    "  Daemon version: {} ({})",
                    Colors::dim(&daemon_version),
                    Colors::error(err)
                );
            } else {
                println!("  Daemon version: {}", daemon_version);
                println!("  Daemon commit: {}", daemon_commit);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct ApiState {
    pid: u32,
    ws_url: String,
    #[serde(default)]
    ui_url: Option<String>,
    listen: String,
    #[serde(default)]
    started_at: Option<u64>,
    #[serde(default)]
    http_url: Option<String>,
}

impl ApiState {
    fn resolved_ui_url(&self) -> String {
        if let Some(url) = self.ui_url.as_ref()
            && !url.trim().is_empty()
        {
            return url.trim().to_string();
        }
        if let Some(http_url) = self.http_url.as_ref()
            && !http_url.trim().is_empty()
        {
            if http_url.ends_with('/') {
                return format!("{http_url}ui");
            }
            return format!("{http_url}/ui");
        }
        if let Ok(ws_url) = url::Url::parse(&self.ws_url) {
            let scheme = if ws_url.scheme() == "wss" {
                "https"
            } else {
                "http"
            };
            if let Some(host) = ws_url.host_str() {
                if let Some(port) = ws_url.port() {
                    return format!("{scheme}://{host}:{port}/ui");
                }
                return format!("{scheme}://{host}/ui");
            }
        }
        "/ui".to_string()
    }
}

#[derive(Debug, Clone)]
struct UiState {
    pid: u32,
    url: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct UiStateFile {
    pid: u32,
    url: String,
    #[serde(default)]
    port: Option<u16>,
}

#[derive(Debug, Clone)]
enum UiStatus {
    External(String),
    Running(UiState),
    NotRunning,
}

#[derive(Debug, Clone)]
enum StopUiResult {
    Stopped,
    AlreadyStopped,
    External,
}

fn ws_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("AGENT_TUI_WS_STATE") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("AGENT_TUI_API_STATE") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".agent-tui").join("api.json")
}

fn read_api_state(path: &PathBuf) -> Option<ApiState> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn is_process_running(pid: u32) -> bool {
    let controller = UnixProcessController;
    matches!(
        controller.check_process(pid),
        Ok(crate::infra::ipc::ProcessStatus::Running)
            | Ok(crate::infra::ipc::ProcessStatus::NoPermission)
    )
}

fn read_api_state_running(path: &PathBuf) -> Option<ApiState> {
    let state = read_api_state(path)?;
    if is_process_running(state.pid) {
        Some(state)
    } else {
        let _ = std::fs::remove_file(path);
        None
    }
}

fn wait_for_api_state(path: &PathBuf, timeout: Duration) -> Option<ApiState> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(state) = read_api_state_running(path) {
            return Some(state);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::park_timeout(Duration::from_millis(50));
    }
}

fn ui_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("AGENT_TUI_UI_STATE") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".agent-tui").join("ui.json")
}

fn read_ui_state(path: &PathBuf) -> Option<UiState> {
    let contents = std::fs::read_to_string(path).ok()?;
    let file: UiStateFile = serde_json::from_str(&contents).ok()?;
    let port = file.port.or_else(|| parse_port_from_url(&file.url))?;
    Some(UiState {
        pid: file.pid,
        url: file.url,
        port,
    })
}

fn read_ui_state_running(path: &PathBuf) -> Option<UiState> {
    let state = read_ui_state(path)?;
    if is_process_running(state.pid) {
        Some(state)
    } else {
        let _ = std::fs::remove_file(path);
        None
    }
}

fn parse_port_from_url(url: &str) -> Option<u16> {
    let host = url.split("://").nth(1)?;
    let host = host.split('/').next()?;
    let addr: SocketAddr = host.parse().ok()?;
    Some(addr.port())
}

fn resolve_ui_status() -> UiStatus {
    if let Ok(url) = std::env::var("AGENT_TUI_UI_URL")
        && !url.trim().is_empty()
    {
        return UiStatus::External(url);
    }
    match read_ui_state_running(&ui_state_path()) {
        Some(state) => UiStatus::Running(state),
        None => UiStatus::NotRunning,
    }
}

fn remove_api_state_file() {
    let state_path = ws_state_path();
    if let Err(err) = std::fs::remove_file(&state_path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            path = %state_path.display(),
            error = %err,
            "Failed to remove WS state file"
        );
    }
}

fn wait_for_process_exit_with_controller<C: ProcessController>(
    controller: &C,
    pid: u32,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match controller.check_process(pid) {
            Ok(crate::infra::ipc::ProcessStatus::NotFound) => return true,
            Ok(_) => {}
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::park_timeout(Duration::from_millis(50));
    }
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let controller = UnixProcessController;
    wait_for_process_exit_with_controller(&controller, pid, timeout)
}

fn process_running_with_controller<C: ProcessController>(controller: &C, pid: u32) -> bool {
    matches!(
        controller.check_process(pid),
        Ok(crate::infra::ipc::ProcessStatus::Running)
            | Ok(crate::infra::ipc::ProcessStatus::NoPermission)
    )
}

const UI_STOP_TERM_TIMEOUT: Duration = Duration::from_secs(2);
const UI_STOP_KILL_TIMEOUT: Duration = Duration::from_secs(2);

fn stop_ui_server() -> Result<StopUiResult> {
    let controller = UnixProcessController;
    stop_ui_server_with_controller_and_timeouts(
        &controller,
        UI_STOP_TERM_TIMEOUT,
        UI_STOP_KILL_TIMEOUT,
    )
}

fn stop_ui_server_with_controller_and_timeouts<C: ProcessController>(
    controller: &C,
    term_timeout: Duration,
    kill_timeout: Duration,
) -> Result<StopUiResult> {
    if let Ok(url) = std::env::var("AGENT_TUI_UI_URL")
        && !url.trim().is_empty()
    {
        return Ok(StopUiResult::External);
    }

    let state_path = ui_state_path();
    let Some(state) = read_ui_state(&state_path) else {
        return Ok(StopUiResult::AlreadyStopped);
    };

    if !process_running_with_controller(controller, state.pid) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::AlreadyStopped);
    }

    controller
        .send_signal(state.pid, Signal::Term)
        .with_context(|| format!("Failed to stop UI server (pid {})", state.pid))?;

    if wait_for_process_exit_with_controller(controller, state.pid, term_timeout) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::Stopped);
    }

    controller
        .send_signal(state.pid, Signal::Kill)
        .with_context(|| format!("Failed to force-stop UI server (pid {})", state.pid))?;

    if wait_for_process_exit_with_controller(controller, state.pid, kill_timeout) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::Stopped);
    }

    Err(anyhow::anyhow!(
        "UI server did not stop after SIGTERM/SIGKILL"
    ))
}

fn build_ui_url(base: &str, state: &ApiState) -> String {
    let (base, fragment) = base.split_once('#').unwrap_or((base, ""));
    if let Ok(mut url) = url::Url::parse(base) {
        let existing: Vec<(String, String)> = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.clear();
            for (key, value) in existing {
                match key.as_ref() {
                    "ws" | "session" | "auto" => {}
                    _ => {
                        pairs.append_pair(&key, &value);
                    }
                }
            }
            pairs.append_pair("ws", &state.ws_url);
            pairs.append_pair("session", "active");
            pairs.append_pair("auto", "1");
        }
        if !fragment.is_empty() {
            url.set_fragment(Some(fragment));
        }
        return url.to_string();
    }

    warn!(base = %base, "Failed to parse UI URL; falling back to string concatenation");

    let separator = if base.contains('?') { "&" } else { "?" };
    let mut url = String::with_capacity(base.len() + 96);
    url.push_str(base);
    url.push_str(separator);
    url.push_str("ws=");
    url.push_str(&state.ws_url);
    url.push_str("&session=active&auto=1");
    if !fragment.is_empty() {
        url.push('#');
        url.push_str(fragment);
    }
    url
}

fn open_in_browser(url: &str, browser_override: Option<&str>) -> Result<()> {
    use std::process::Command;

    let browser = browser_override
        .map(String::from)
        .or_else(|| std::env::var("BROWSER").ok());

    let mut cmd = if let Some(browser) = browser {
        let parts = shell_words::split(&browser)
            .with_context(|| format!("Failed to parse browser command: {browser}"))?;
        let (program, args) = parts
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("Browser command is empty"))?;
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start"]);
        cmd
    } else {
        Command::new("xdg-open")
    };

    let program = cmd.get_program().to_string_lossy().into_owned();
    let status = cmd
        .arg(url)
        .status()
        .with_context(|| format!("Failed to launch browser ({program})"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Browser command exited with status {}",
            status
        ))
    }
}

pub(crate) fn handle_cleanup<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    all: bool,
) -> HandlerResult {
    use crate::adapters::presenter::CleanupFailure;
    use crate::adapters::presenter::CleanupResult;

    let sessions_result = call_no_params(ctx.client, "sessions")?;
    let sessions = sessions_result.get("sessions").and_then(|v| v.as_array());

    let mut cleaned = 0;
    let mut failures: Vec<CleanupFailure> = Vec::new();

    if let Some(sessions) = sessions {
        for session in sessions.iter() {
            let id = session.get("id").and_then(|v| v.as_str());
            let should_cleanup = all || !session.bool_or("running", false);
            if should_cleanup && let Some(id) = id {
                let params = params::SessionParams {
                    session: Some(id.to_string()),
                };
                match call_with_params(ctx.client, "kill", params) {
                    Ok(_) => cleaned += 1,
                    Err(e) => failures.push(CleanupFailure {
                        session_id: id.to_string(),
                        error: e.to_string(),
                    }),
                }
            }
        }
    }

    let result = CleanupResult { cleaned, failures };

    #[derive(Serialize)]
    struct CleanupFailureJson {
        session: String,
        error: String,
    }

    #[derive(Serialize)]
    struct CleanupOutputJson {
        sessions_cleaned: usize,
        sessions_failed: usize,
        failures: Vec<CleanupFailureJson>,
    }

    let output = CleanupOutputJson {
        sessions_cleaned: result.cleaned,
        sessions_failed: result.failures.len(),
        failures: result
            .failures
            .iter()
            .map(|f| CleanupFailureJson {
                session: f.session_id.clone(),
                error: f.error.clone(),
            })
            .collect(),
    };

    if result.failures.is_empty() {
        match ctx.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Text => ctx.presenter().present_cleanup(&result),
        }
    } else {
        let mut message = format!("Failed to clean up {} session(s)", result.failures.len());
        for failure in &result.failures {
            message.push_str(&format!("\n  {}: {}", failure.session_id, failure.error));
        }
        return Err(CliError::new(
            ctx.format,
            message,
            Some(serde_json::to_string_pretty(&output)?),
            super::exit_codes::GENERAL_ERROR,
        )
        .into());
    }
    Ok(())
}

pub(crate) fn handle_attach<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
    interactive: bool,
    detach_keys: Option<DetachKeys>,
) -> HandlerResult {
    use crate::app::attach;

    if interactive {
        let stdin_tty = io::stdin().is_terminal();
        let stdout_tty = io::stdout().is_terminal();
        if !stdin_tty || !stdout_tty {
            let err = io::Error::other(
                "interactive attach requires a TTY on stdin and stdout (use -T to disable TTY)",
            );
            return Err(AttachError::Terminal(err).into());
        }
    }

    let params = params::SessionParams {
        session: Some(session_id.clone()),
    };
    let result = call_with_params(ctx.client, "attach", params)?;

    if interactive {
        if !result.bool_or("success", false) {
            return Err(CliError::new(
                ctx.format,
                format!("Failed to attach to session: {}", session_id),
                Some(result.to_pretty_json()),
                super::exit_codes::GENERAL_ERROR,
            )
            .into());
        }

        let mode = if interactive {
            attach::AttachMode::Tty
        } else {
            attach::AttachMode::Stream
        };
        let detach_keys = detach_keys.unwrap_or_default();
        attach::attach_ipc(ctx.client, &session_id, mode, detach_keys)?;
    } else {
        match ctx.format {
            OutputFormat::Json => {
                if result.bool_or("success", false) {
                    println!("{}", result.to_pretty_json());
                } else {
                    return Err(CliError::new(
                        ctx.format,
                        format!("Failed to attach to session: {}", session_id),
                        Some(result.to_pretty_json()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
            OutputFormat::Text => {
                if result.bool_or("success", false) {
                    println!("Attached to session {}", Colors::session_id(&session_id));
                } else {
                    return Err(CliError::new(
                        ctx.format,
                        format!("Failed to attach to session: {}", session_id),
                        Some(result.to_pretty_json()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn handle_env(format: OutputFormat) -> HandlerResult {
    let vars = [
        (
            "AGENT_TUI_TRANSPORT",
            std::env::var("AGENT_TUI_TRANSPORT").ok(),
        ),
        ("AGENT_TUI_WS_ADDR", std::env::var("AGENT_TUI_WS_ADDR").ok()),
        (
            "AGENT_TUI_DETACH_KEYS",
            std::env::var("AGENT_TUI_DETACH_KEYS").ok(),
        ),
        (
            "AGENT_TUI_DAEMON_FOREGROUND",
            std::env::var("AGENT_TUI_DAEMON_FOREGROUND").ok(),
        ),
        (
            "AGENT_TUI_WS_LISTEN",
            std::env::var("AGENT_TUI_WS_LISTEN").ok(),
        ),
        (
            "AGENT_TUI_WS_ALLOW_REMOTE",
            std::env::var("AGENT_TUI_WS_ALLOW_REMOTE").ok(),
        ),
        (
            "AGENT_TUI_WS_STATE",
            std::env::var("AGENT_TUI_WS_STATE").ok(),
        ),
        (
            "AGENT_TUI_WS_DISABLED",
            std::env::var("AGENT_TUI_WS_DISABLED").ok(),
        ),
        (
            "AGENT_TUI_WS_MAX_CONNECTIONS",
            std::env::var("AGENT_TUI_WS_MAX_CONNECTIONS").ok(),
        ),
        (
            "AGENT_TUI_WS_QUEUE",
            std::env::var("AGENT_TUI_WS_QUEUE").ok(),
        ),
        // Deprecated aliases (kept for compatibility).
        (
            "AGENT_TUI_API_LISTEN",
            std::env::var("AGENT_TUI_API_LISTEN").ok(),
        ),
        (
            "AGENT_TUI_API_ALLOW_REMOTE",
            std::env::var("AGENT_TUI_API_ALLOW_REMOTE").ok(),
        ),
        (
            "AGENT_TUI_API_STATE",
            std::env::var("AGENT_TUI_API_STATE").ok(),
        ),
        (
            "AGENT_TUI_API_TOKEN",
            std::env::var("AGENT_TUI_API_TOKEN").ok(),
        ),
        (
            "AGENT_TUI_API_DISABLED",
            std::env::var("AGENT_TUI_API_DISABLED").ok(),
        ),
        (
            "AGENT_TUI_API_MAX_CONNECTIONS",
            std::env::var("AGENT_TUI_API_MAX_CONNECTIONS").ok(),
        ),
        (
            "AGENT_TUI_API_WS_QUEUE",
            std::env::var("AGENT_TUI_API_WS_QUEUE").ok(),
        ),
        (
            "AGENT_TUI_SESSION_STORE",
            std::env::var("AGENT_TUI_SESSION_STORE").ok(),
        ),
        ("AGENT_TUI_LOG", std::env::var("AGENT_TUI_LOG").ok()),
        (
            "AGENT_TUI_LOG_FORMAT",
            std::env::var("AGENT_TUI_LOG_FORMAT").ok(),
        ),
        (
            "AGENT_TUI_LOG_STREAM",
            std::env::var("AGENT_TUI_LOG_STREAM").ok(),
        ),
        ("AGENT_TUI_UI_URL", std::env::var("AGENT_TUI_UI_URL").ok()),
        ("AGENT_TUI_UI_MODE", std::env::var("AGENT_TUI_UI_MODE").ok()),
        ("AGENT_TUI_UI_PORT", std::env::var("AGENT_TUI_UI_PORT").ok()),
        ("AGENT_TUI_UI_ROOT", std::env::var("AGENT_TUI_UI_ROOT").ok()),
        (
            "AGENT_TUI_UI_STATE",
            std::env::var("AGENT_TUI_UI_STATE").ok(),
        ),
        ("PORT", std::env::var("PORT").ok()),
        ("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR").ok()),
        ("NO_COLOR", std::env::var("NO_COLOR").ok()),
    ];

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct EnvOutput {
                environment: HashMap<&'static str, Option<String>>,
                socket_path: String,
            }

            let env_map: HashMap<&'static str, Option<String>> = vars.iter().cloned().collect();
            let output = EnvOutput {
                environment: env_map,
                socket_path: socket_path().display().to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("{}", Colors::bold("Environment Configuration:"));
            let transport = vars
                .iter()
                .find(|(n, _)| *n == "AGENT_TUI_TRANSPORT")
                .and_then(|(_, v)| v.as_ref());
            println!(
                "  Transport: {}",
                transport.map(|v| v.as_str()).unwrap_or("unix")
            );
            println!("  Socket: {}", socket_path().display());
            println!();
            println!("{}", Colors::bold("Environment Variables:"));
            for (name, value) in &vars {
                let val_str = value.as_ref().map(|v| v.as_str()).unwrap_or("(not set)");
                println!(
                    "  {}: {}",
                    name,
                    if value.is_some() {
                        Colors::info(val_str)
                    } else {
                        Colors::dim(val_str)
                    }
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn handle_assert<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    condition: String,
) -> HandlerResult {
    let parts: Vec<&str> = condition.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(CliError::new(
            ctx.format,
            "Invalid condition format. Use: text:pattern or session:id",
            None,
            super::exit_codes::USAGE,
        )
        .into());
    }

    let (cond_type, cond_value) = (parts[0], parts[1]);

    let passed = match cond_type {
        "text" => {
            let params = params::SnapshotParams {
                session: ctx.session.clone(),
                region: None,
                strip_ansi: true,
                include_cursor: false,
                include_render: false,
            };
            let result = call_with_params(ctx.client, "snapshot", params)?;
            result.str_or("screenshot", "").contains(cond_value)
        }
        "session" => {
            let result = call_no_params(ctx.client, "sessions")?;
            if let Some(sessions) = result.get("sessions").and_then(|v| v.as_array()) {
                sessions
                    .iter()
                    .any(|s| s.str_or("id", "") == cond_value && s.bool_or("running", false))
            } else {
                false
            }
        }
        _ => {
            return Err(CliError::new(
                ctx.format,
                format!(
                    "Unknown condition type: {}. Use: text or session",
                    cond_type
                ),
                None,
                super::exit_codes::USAGE,
            )
            .into());
        }
    };

    let assert_result = crate::adapters::presenter::AssertResult {
        passed,
        condition: condition.clone(),
    };

    if assert_result.passed {
        match ctx.format {
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct AssertOutput<'a> {
                    condition: &'a str,
                    passed: bool,
                }
                let output = AssertOutput {
                    condition: &condition,
                    passed,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Text => {
                ctx.presenter().present_assert_result(&assert_result);
            }
        }
    } else {
        #[derive(Serialize)]
        struct AssertOutput<'a> {
            condition: &'a str,
            passed: bool,
        }
        let output = AssertOutput {
            condition: &condition,
            passed,
        };
        return Err(CliError::new(
            ctx.format,
            format!("Assertion failed: {}", assert_result.condition),
            Some(serde_json::to_string_pretty(&output)?),
            super::exit_codes::GENERAL_ERROR,
        )
        .into());
    }
    Ok(())
}

/// Result of the daemon stop operation.
pub enum StopResult {
    /// Daemon was stopped successfully.
    Stopped { pid: u32, warnings: Vec<String> },
    /// Daemon was already stopped (idempotent success).
    AlreadyStopped,
}

/// Core daemon restart logic that doesn't require an active client connection.
pub(crate) fn restart_daemon_core() -> Result<Vec<String>> {
    use crate::infra::ipc::PidLookupResult;
    use crate::infra::ipc::daemon_lifecycle;
    use crate::infra::ipc::get_daemon_pid;
    use crate::infra::ipc::start_daemon_background;

    let mut warnings = Vec::new();
    let pid = match get_daemon_pid() {
        PidLookupResult::Found(pid) => Some(pid),
        PidLookupResult::NotRunning => None,
        PidLookupResult::Error(msg) => {
            warnings.push(format!("Could not read daemon PID: {}", msg));
            None
        }
    };

    let controller = UnixProcessController;
    let get_pid = move || pid;
    let mut restart_warnings = daemon_lifecycle::restart_daemon(
        &controller,
        get_pid,
        &socket_path(),
        start_daemon_background,
    )?;
    warnings.append(&mut restart_warnings);
    Ok(warnings)
}

/// Core daemon stop logic that doesn't require an active client connection.
/// Returns `Ok(StopResult)` on success, including when daemon is already stopped (idempotent).
pub(crate) fn stop_daemon_core(force: bool) -> Result<StopResult> {
    use crate::infra::ipc::PidLookupResult;
    use crate::infra::ipc::UnixSocketClient;
    use crate::infra::ipc::daemon_lifecycle;
    use crate::infra::ipc::get_daemon_pid;

    let pid = match get_daemon_pid() {
        PidLookupResult::Found(pid) => pid,
        PidLookupResult::NotRunning => {
            remove_api_state_file();
            return Ok(StopResult::AlreadyStopped);
        }
        PidLookupResult::Error(msg) => {
            return Err(anyhow::Error::new(ClientError::SignalFailed {
                pid: 0,
                message: msg,
                source: None,
            }));
        }
    };

    let socket = socket_path();

    if !force {
        // Try graceful RPC shutdown first (needs connection but doesn't auto-start)
        if let Ok(mut client) = UnixSocketClient::connect()
            && let Ok(result) = daemon_lifecycle::stop_daemon_via_rpc(&mut client, &socket)
        {
            remove_api_state_file();
            return Ok(StopResult::Stopped {
                pid,
                warnings: result.warnings,
            });
        }
    }

    // Fall back to signal-based stop
    let controller = UnixProcessController;
    let stop_result = match daemon_lifecycle::stop_daemon(&controller, pid, &socket, force) {
        Ok(result) => StopResult::Stopped {
            pid,
            warnings: result.warnings,
        },
        Err(ClientError::DaemonNotRunning) => StopResult::AlreadyStopped,
        Err(err) => return Err(err.into()),
    };

    remove_api_state_file();
    Ok(stop_result)
}

pub(crate) fn handle_daemon_stop<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    force: bool,
) -> HandlerResult {
    match stop_daemon_core(force)? {
        StopResult::Stopped { warnings, .. } => {
            for warning in &warnings {
                eprintln!("{}", Colors::warning(warning));
            }
            ctx.presenter().present_success("Daemon stopped", None);
        }
        StopResult::AlreadyStopped => {
            ctx.presenter()
                .present_success("Daemon is not running (already stopped)", None);
        }
    }
    Ok(())
}

pub(crate) fn handle_daemon_restart<C: DaemonClient>(ctx: &HandlerContext<C>) -> HandlerResult {
    if let OutputFormat::Text = ctx.format {
        ctx.presenter().present_info("Restarting daemon...");
    }
    let warnings = restart_daemon_core()?;

    for warning in &warnings {
        eprintln!("{}", Colors::warning(warning));
    }

    ctx.presenter().present_success("Daemon restarted", None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::RpcValue;
    use crate::adapters::presenter::ClientErrorView;
    use crate::adapters::presenter::Presenter;
    use crate::adapters::presenter::TextPresenter;
    use crate::infra::ipc::ProcessStatus;
    use crate::test_support::env_lock;
    use std::cell::RefCell;
    use std::fs;
    use std::rc::Rc;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl Into<String>) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: test-only environment mutation under env_lock.
            unsafe {
                std::env::set_var(key, value.into());
            }
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: test-only environment restoration under env_lock.
            unsafe {
                if let Some(prev) = self.prev.take() {
                    std::env::set_var(self.key, prev);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    struct StopUiController {
        status: Mutex<ProcessStatus>,
        signals: Mutex<Vec<Signal>>,
        kill_on_term: bool,
        kill_on_kill: bool,
    }

    impl StopUiController {
        fn new(status: ProcessStatus, kill_on_term: bool, kill_on_kill: bool) -> Self {
            Self {
                status: Mutex::new(status),
                signals: Mutex::new(Vec::new()),
                kill_on_term,
                kill_on_kill,
            }
        }

        fn signals(&self) -> Vec<Signal> {
            self.signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }

    impl ProcessController for StopUiController {
        fn check_process(&self, _pid: u32) -> std::io::Result<crate::infra::ipc::ProcessStatus> {
            Ok(*self.status.lock().unwrap_or_else(|e| e.into_inner()))
        }

        fn send_signal(&self, _pid: u32, signal: Signal) -> std::io::Result<()> {
            self.signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(signal);
            let should_stop = match signal {
                Signal::Term => self.kill_on_term,
                Signal::Kill => self.kill_on_kill,
            };
            if should_stop {
                *self.status.lock().unwrap_or_else(|e| e.into_inner()) = ProcessStatus::NotFound;
            }
            Ok(())
        }
    }

    struct MockPresenter {
        output: Rc<RefCell<Vec<String>>>,
    }

    impl MockPresenter {
        fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
            let output = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    output: output.clone(),
                },
                output,
            )
        }
    }

    impl Presenter for MockPresenter {
        fn present_success(&self, message: &str, warning: Option<&str>) {
            self.output
                .borrow_mut()
                .push(format!("success: {}", message));
            if let Some(w) = warning {
                self.output.borrow_mut().push(format!("warning: {}", w));
            }
        }

        fn present_error(&self, message: &str) {
            self.output.borrow_mut().push(format!("error: {}", message));
        }

        fn present_value(&self, value: &RpcValue) {
            self.output
                .borrow_mut()
                .push(format!("value: {}", value.to_pretty_json()));
        }

        fn present_client_error(&self, error: &ClientErrorView) {
            self.output
                .borrow_mut()
                .push(format!("client_error: {}", error.message));
        }

        fn present_kv(&self, key: &str, value: &str) {
            self.output
                .borrow_mut()
                .push(format!("kv: {}={}", key, value));
        }

        fn present_session_id(&self, session_id: &str, label: Option<&str>) {
            self.output
                .borrow_mut()
                .push(format!("session: {} {:?}", session_id, label));
        }

        fn present_list_header(&self, title: &str) {
            self.output.borrow_mut().push(format!("header: {}", title));
        }

        fn present_list_item(&self, item: &str) {
            self.output.borrow_mut().push(format!("item: {}", item));
        }

        fn present_info(&self, message: &str) {
            self.output.borrow_mut().push(format!("info: {}", message));
        }

        fn present_header(&self, text: &str) {
            self.output.borrow_mut().push(format!("bold: {}", text));
        }

        fn present_raw(&self, text: &str) {
            self.output.borrow_mut().push(format!("raw: {}", text));
        }

        fn present_wait_result(&self, result: &crate::adapters::presenter::WaitResult) {
            self.output.borrow_mut().push(format!(
                "wait: found={}, elapsed={}ms",
                result.found, result.elapsed_ms
            ));
        }

        fn present_assert_result(&self, result: &crate::adapters::presenter::AssertResult) {
            self.output.borrow_mut().push(format!(
                "assert: passed={}, condition={}",
                result.passed, result.condition
            ));
        }

        fn present_cleanup(&self, result: &crate::adapters::presenter::CleanupResult) {
            self.output.borrow_mut().push(format!(
                "cleanup: cleaned={}, failed={}",
                result.cleaned,
                result.failures.len()
            ));
        }
    }

    #[test]
    fn test_handler_context_has_presenter() {
        let presenter = TextPresenter;

        let _: &dyn Presenter = &presenter;
    }

    #[test]
    fn test_mock_presenter_captures_output() {
        let (presenter, output) = MockPresenter::new();

        presenter.present_success("Operation completed", None);
        presenter.present_error("Something failed");
        presenter.present_kv("key", "value");

        let captured = output.borrow();
        assert!(captured.iter().any(|s| s.contains("success:")));
        assert!(captured.iter().any(|s| s.contains("error:")));
        assert!(captured.iter().any(|s| s.contains("kv:")));
    }

    #[test]
    fn test_assert_condition_parsing_text() {
        let condition = "text:Submit";
        let parts: Vec<&str> = condition.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "text");
        assert_eq!(parts[1], "Submit");
    }

    #[test]
    fn test_assert_condition_parsing_session() {
        let condition = "session:my-session";
        let (kind, value) = condition.split_once(':').expect("expected separator");
        assert_eq!(kind, "session");
        assert_eq!(value, "my-session");
    }

    #[test]
    fn test_assert_condition_parsing_with_colon_in_value() {
        let condition = "text:URL: https://example.com";
        let (kind, value) = condition.split_once(':').expect("expected separator");
        assert_eq!(kind, "text");
        assert_eq!(value, "URL: https://example.com");
    }

    #[test]
    fn test_assert_condition_parsing_invalid() {
        let condition = "invalid_format";
        assert!(condition.split_once(':').is_none());
    }

    #[test]
    fn test_wait_condition_stable() {
        let params = WaitParams {
            stable: true,
            ..Default::default()
        };
        let cond = resolve_wait_condition(&params);
        assert_eq!(cond, Some("stable".to_string()));
    }

    #[test]
    fn test_wait_condition_text_gone() {
        let params = WaitParams {
            text: Some("Loading...".to_string()),
            gone: true,
            ..Default::default()
        };
        let cond = resolve_wait_condition(&params);
        assert_eq!(cond, Some("text_gone".to_string()));
    }

    #[test]
    fn test_wait_condition_none() {
        let params = WaitParams::default();
        let cond = resolve_wait_condition(&params);
        assert_eq!(cond, None);
    }

    #[test]
    fn stop_ui_server_escalates_to_sigkill_and_cleans_state() {
        let _env = env_lock();
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let state_path = temp.path().join("ui.json");
        fs::write(
            &state_path,
            r#"{"pid":42,"url":"http://127.0.0.1:7777/ui","port":7777}"#,
        )
        .expect("write ui state");
        let _state_guard = EnvVarGuard::set("AGENT_TUI_UI_STATE", state_path.display().to_string());
        let _external_guard = EnvVarGuard::set("AGENT_TUI_UI_URL", "");

        let controller = StopUiController::new(ProcessStatus::Running, false, true);
        let result = stop_ui_server_with_controller_and_timeouts(
            &controller,
            Duration::from_millis(5),
            Duration::from_millis(5),
        )
        .expect("ui stop should escalate and succeed");

        assert!(matches!(result, StopUiResult::Stopped));
        assert_eq!(controller.signals(), vec![Signal::Term, Signal::Kill]);
        assert!(
            !state_path.exists(),
            "state file should be removed when stop succeeds"
        );
    }
}
