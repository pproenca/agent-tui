#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI command handlers.

use std::collections::HashMap;
use std::io;
use std::io::IsTerminal;
use std::io::Read;
use std::net::SocketAddr;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

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
use crate::infra::ipc::DaemonProcessLookupResult;
use crate::infra::ipc::ProcessController;
use crate::infra::ipc::ProcessIdentity;
use crate::infra::ipc::Signal;
use crate::infra::ipc::UnixProcessController;
use crate::infra::ipc::UnixSocketClient;
use crate::infra::ipc::check_expected_process;
use crate::infra::ipc::daemon_uses_client_working_directory;
use crate::infra::ipc::get_daemon_process_identity;
use crate::infra::ipc::socket_path;
use crate::infra::ipc::start_daemon_background;

use crate::adapters::presenter::ClientErrorView;
use crate::adapters::presenter::Presenter;
use crate::adapters::presenter::create_presenter;
use crate::app::attach::DetachKeys;
use crate::app::commands::LiveStartArgs;
use crate::app::commands::OutputFormat;
use crate::app::commands::ScrollDirection;
use crate::app::commands::WaitParams;
use crate::app::error::AttachError;
use crate::app::error::CliError;
use crate::app::error::DaemonNotRunningError;
use crate::app::rpc_client::call_no_params;
use crate::app::rpc_client::call_with_params;
use crate::domain::session_types::TerminalSize;

pub(crate) type HandlerResult = Result<()>;

fn format_elapsed_secs(elapsed_secs: u64) -> String {
    let mins = elapsed_secs / 60;
    let hours = mins / 60;
    if hours > 0 {
        format!("{hours}h {}m {}s", mins % 60, elapsed_secs % 60)
    } else if mins > 0 {
        format!("{mins}m {}s", elapsed_secs % 60)
    } else {
        format!("{elapsed_secs}s")
    }
}

fn daemon_pid(ws_state: Option<&WsState>) -> Option<u32> {
    match get_daemon_process_identity() {
        DaemonProcessLookupResult::Found(identity) => Some(identity.pid),
        DaemonProcessLookupResult::NotRunning | DaemonProcessLookupResult::InvalidState { .. } => {
            ws_state.map(|state| state.pid)
        }
    }
}

fn client_error_view(error: &ClientError) -> ClientErrorView {
    ClientErrorView {
        message: error.to_string(),
        suggestion: error.suggestion().map(str::to_string),
        retryable: error.is_retryable(),
        json: Some(error.to_json_string()),
    }
}

fn can_autostart_after_local_connect_error(error: &ClientError) -> bool {
    match error {
        ClientError::DaemonNotRunning => true,
        ClientError::ConnectionFailed(io_err) => matches!(
            io_err.kind(),
            io::ErrorKind::ConnectionRefused
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::NotFound
        ),
        _ => false,
    }
}

fn structured_cli_error(
    format: OutputFormat,
    exit_code: i32,
    message: impl Into<String>,
    category: &'static str,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    let message = message.into();
    let mut output = serde_json::json!({
        "code": exit_code,
        "message": message.as_str(),
        "category": category,
        "retryable": false,
    });
    if let Some(context) = context {
        output["context"] = context;
    }
    if let Some(suggestion) = suggestion.as_deref() {
        output["suggestion"] = serde_json::json!(suggestion);
    }
    CliError::new(
        format,
        message,
        Some(serde_json::to_string_pretty(&output).unwrap_or_default()),
        exit_code,
    )
}

fn usage_cli_error(
    format: OutputFormat,
    message: impl Into<String>,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    structured_cli_error(
        format,
        super::exit_codes::USAGE,
        message,
        "invalid_input",
        suggestion,
        context,
    )
}

fn unavailable_cli_error(
    format: OutputFormat,
    message: impl Into<String>,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    structured_cli_error(
        format,
        super::exit_codes::UNAVAILABLE,
        message,
        "not_found",
        suggestion,
        context,
    )
}

fn timeout_cli_error(
    format: OutputFormat,
    message: impl Into<String>,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    structured_cli_error(
        format,
        super::exit_codes::TEMPFAIL,
        message,
        "timeout",
        suggestion,
        context,
    )
}

fn confirmation_required_cli_error(format: OutputFormat) -> CliError {
    usage_cli_error(
        format,
        "Confirmation required. Re-run with --yes to perform the action or --dry-run to preview it.",
        Some("Add --yes to proceed or --dry-run to preview the change.".to_string()),
        None,
    )
}

fn confirmation_cancelled_cli_error(format: OutputFormat) -> CliError {
    structured_cli_error(
        format,
        super::exit_codes::GENERAL_ERROR,
        "Cancelled at confirmation prompt.",
        "external",
        None,
        None,
    )
}

fn confirm_destructive_action(
    format: OutputFormat,
    no_input: bool,
    yes: bool,
    prompt: &str,
) -> Result<()> {
    if yes {
        return Ok(());
    }

    let stdin_tty = io::stdin().is_terminal();
    let stdout_tty = io::stdout().is_terminal();
    if no_input || !stdin_tty || !stdout_tty {
        return Err(confirmation_required_cli_error(format).into());
    }

    if crate::app::prompt_yes_no(prompt, false)? {
        Ok(())
    } else {
        Err(confirmation_cancelled_cli_error(format).into())
    }
}

fn selected_session_label(session: Option<&str>) -> String {
    session
        .map(|id| format!("session {}", Colors::session_id(id)))
        .unwrap_or_else(|| "the current session".to_string())
}

fn selected_session_plain_label(session: Option<&str>) -> String {
    session
        .map(|id| format!("session {id}"))
        .unwrap_or_else(|| "the current session".to_string())
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

    if (params.text.is_some() || params.legacy_element.is_some()) && params.gone {
        return Some("text_gone".to_string());
    }

    if params.legacy_element.is_some() {
        return Some("text".to_string());
    }

    None
}

pub(crate) struct HandlerContext<'a, C: DaemonClient> {
    pub client: &'a mut C,
    pub session: Option<String>,
    pub format: OutputFormat,
    pub no_input: bool,
    presenter: Box<dyn Presenter>,
    current_dir_override: Option<PathBuf>,
}

impl<'a, C: DaemonClient> HandlerContext<'a, C> {
    pub fn new(
        client: &'a mut C,
        session: Option<String>,
        format: OutputFormat,
        no_input: bool,
    ) -> Self {
        let presenter = create_presenter(&format);
        Self {
            client,
            session,
            format,
            no_input,
            presenter,
            current_dir_override: None,
        }
    }

    pub fn presenter(&self) -> &dyn Presenter {
        self.presenter.as_ref()
    }

    fn effective_current_dir(&self) -> std::io::Result<PathBuf> {
        if let Some(path) = &self.current_dir_override {
            Ok(path.clone())
        } else {
            std::env::current_dir()
        }
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
                    let message = format!("{failure_prefix}: {msg}");
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
                    let message = format!("{failure_prefix}: {msg}");
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
    env: Option<HashMap<String, String>>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
    let size = TerminalSize::try_new(cols, rows).map_err(|err| {
        usage_cli_error(
            ctx.format,
            format!("Invalid terminal size: {err}"),
            Some("Use a terminal size within the supported range.".to_string()),
            Some(serde_json::json!({ "cols": cols, "rows": rows })),
        )
    })?;
    let cwd = match cwd {
        Some(path) => Some(path.to_string_lossy().into_owned()),
        None if daemon_uses_client_working_directory() => Some(
            ctx.effective_current_dir()
                .context("failed to resolve current working directory for run command")?
                .to_string_lossy()
                .into_owned(),
        ),
        None => None,
    };
    let rpc_params = params::SpawnParams {
        command,
        args,
        cwd,
        env,
        session: ctx.session.clone(),
        size,
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
        println!("  PID: {pid}");
    })
}

pub(crate) fn handle_snapshot<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    region: Option<String>,
    strip_ansi: bool,
    retain_ansi: bool,
    include_cursor: bool,
) -> HandlerResult {
    let preserve_ansi = retain_ansi || !strip_ansi;
    let rpc_params = params::SnapshotParams {
        session: ctx.session.clone(),
        region,
        strip_ansi,
        retain_ansi: preserve_ansi,
        include_cursor,
        include_render: preserve_ansi,
    };
    let result = call_with_params(ctx.client, "snapshot", rpc_params)?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", result.to_pretty_json());
        }
        OutputFormat::Text => {
            println!("{}", Colors::bold("Screenshot:"));
            let screen = if preserve_ansi {
                result
                    .get("compact_rendered")
                    .and_then(|v| v.as_str())
                    .or_else(|| result.get("rendered").and_then(|v| v.as_str()))
                    .or_else(|| result.get("screenshot").and_then(|v| v.as_str()))
            } else {
                result.get("screenshot").and_then(|v| v.as_str())
            };
            if let Some(screenshot) = screen {
                println!("{screenshot}");
            }
            if include_cursor {
                if let Some(cursor) = result.get("cursor") {
                    let row = cursor.u64_or("row", 0);
                    let col = cursor.u64_or("col", 0);
                    let visible = cursor.bool_or("visible", false);
                    let vis_str = if visible { "visible" } else { "hidden" };
                    println!("\nCursor: row={row}, col={col} ({vis_str})");
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
    "Key held: {k}"
));
key_handler!(handle_keyup, "keyup", |k: &String| format!(
    "Key released: {k}"
));

pub(crate) fn handle_type<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    text: String,
) -> HandlerResult {
    let text = if text == "-" {
        if io::stdin().is_terminal() {
            return Err(usage_cli_error(
                ctx.format,
                "Refusing to read `type -` from a TTY stdin.",
                Some(
                    "Pipe text into stdin or pass the literal text argument directly.".to_string(),
                ),
                None,
            )
            .into());
        }

        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("failed to read text payload from stdin for `type -`")?;
        buffer
    } else {
        text
    };

    let params = params::TypeParams {
        text,
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "type", params)?;
    ctx.output_success_and_ok(&result, "Text typed", "Type failed")
}

pub(crate) fn warn_legacy_deprecation(command: &str, replacement: &str) {
    let replacement_display = format!("`agent-tui {replacement}`");
    warn_legacy_deprecation_with_replacement(command, &replacement_display);
}

pub(crate) fn warn_legacy_deprecation_with_replacement(command: &str, replacement_display: &str) {
    eprintln!(
        "Warning: agent-tui {command} is deprecated; use {replacement_display} instead. It will be deprecated in the next major release."
    );
}

pub(crate) fn handle_scroll<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    direction: ScrollDirection,
    amount: NonZeroU16,
) -> HandlerResult {
    const SCROLL_INTER_STEP_DELAY_MS: u64 = 50;
    let amount = amount.get();

    let key = match direction {
        ScrollDirection::Up => "ArrowUp",
        ScrollDirection::Down => "ArrowDown",
        ScrollDirection::Left => "ArrowLeft",
        ScrollDirection::Right => "ArrowRight",
    };

    for step in 0..amount {
        let params = params::KeyParams {
            key: key.to_string(),
            session: ctx.session.clone(),
        };
        let result = call_with_params(ctx.client, "keystroke", params)?;
        let success = result.bool_or("success", false);
        if !success {
            let message = result.str_or("message", "Unknown error");
            return Err(CliError::new(
                ctx.format,
                format!("scroll failed: {message}"),
                Some(result.to_pretty_json()),
                super::exit_codes::GENERAL_ERROR,
            )
            .into());
        }

        if step + 1 < amount {
            std::thread::park_timeout(Duration::from_millis(SCROLL_INTER_STEP_DELAY_MS));
        }
    }

    match ctx.format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct ScrollOutput<'a> {
                success: bool,
                direction: &'a str,
                amount: u16,
            }

            let output = ScrollOutput {
                success: true,
                direction: direction.as_str(),
                amount,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            if amount == 1 {
                println!("Scrolled {}", direction.as_str());
            } else {
                println!("Scrolled {} {} steps", direction.as_str(), amount);
            }
        }
    }

    Ok(())
}

pub(crate) fn handle_wait<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    wait_params: WaitParams,
) -> HandlerResult {
    use crate::adapters::presenter::WaitResult;

    let cond = resolve_wait_condition(&wait_params);
    let WaitParams {
        text,
        legacy_element,
        timeout,
        assert,
        ..
    } = wait_params;
    let text = text.or(legacy_element);
    let rpc_params = params::WaitParams {
        session: ctx.session.clone(),
        text,
        timeout_ms: timeout,
        condition: cond,
    };
    let result = call_with_params(ctx.client, "wait", rpc_params)?;

    let wait_result = WaitResult::from_json(&result);

    if assert && !wait_result.found {
        return Err(timeout_cli_error(
            ctx.format,
            "Wait condition not met within timeout.",
            Some("Increase --timeout or verify that the expected UI state can appear before asserting.".to_string()),
            Some(serde_json::to_value(result.as_ref())?),
        )
        .into());
    }

    match ctx.format {
        OutputFormat::Json => ctx.presenter().present_value(&result),
        OutputFormat::Text => ctx.presenter().present_wait_result(&wait_result),
    }
    Ok(())
}

pub(crate) fn handle_kill<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    dry_run: bool,
    yes: bool,
) -> HandlerResult {
    let target = selected_session_plain_label(ctx.session.as_deref());
    if dry_run {
        #[derive(Serialize)]
        struct KillPreview<'a> {
            action: &'static str,
            dry_run: bool,
            target: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            session_id: Option<&'a str>,
        }

        let output = KillPreview {
            action: "kill",
            dry_run: true,
            target: &target,
            session_id: ctx.session.as_deref(),
        };

        return ctx.output_json_or(&RpcValue::new(serde_json::to_value(output)?), || {
            println!(
                "Would kill {}.",
                selected_session_label(ctx.session.as_deref())
            );
        });
    }

    confirm_destructive_action(ctx.format, ctx.no_input, yes, &format!("Kill {target}?"))?;

    let params = params::SessionParams {
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "kill", params)?;

    ctx.output_json_or(&result, || {
        println!("Session {} killed", result.str_or("session_id", "unknown"));
    })
}

pub(crate) fn handle_restart<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    dry_run: bool,
    yes: bool,
) -> HandlerResult {
    let target = selected_session_plain_label(ctx.session.as_deref());
    if dry_run {
        #[derive(Serialize)]
        struct RestartPreview<'a> {
            action: &'static str,
            dry_run: bool,
            target: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            session_id: Option<&'a str>,
        }

        let output = RestartPreview {
            action: "restart",
            dry_run: true,
            target: &target,
            session_id: ctx.session.as_deref(),
        };

        return ctx.output_json_or(&RpcValue::new(serde_json::to_value(output)?), || {
            println!(
                "Would restart {}.",
                selected_session_label(ctx.session.as_deref())
            );
        });
    }

    confirm_destructive_action(ctx.format, ctx.no_input, yes, &format!("Restart {target}?"))?;

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
        .ok_or_else(|| {
            structured_cli_error(
                ctx.format,
                super::exit_codes::GENERAL_ERROR,
                "Invalid sessions response.",
                "internal",
                Some(
                    "Run `agent-tui sessions --json` to inspect the raw session payload."
                        .to_string(),
                ),
                None,
            )
        })?;

    let session = sessions
        .iter()
        .find(|session| session.str_or("id", "") == session_id.as_str())
        .ok_or_else(|| {
            unavailable_cli_error(
                ctx.format,
                format!("Session not found: {session_id}"),
                Some("Run `agent-tui sessions list` to discover valid session ids.".to_string()),
                Some(serde_json::json!({ "session_id": session_id })),
            )
        })?;

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
            println!("  Command: {command}");
            println!("  Status: {status}");
            println!("  Size: {cols}x{rows}");
            println!("  PID: {pid}");
            if let Some(created) = created_at {
                println!("  Created: {created}");
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

    Err(unavailable_cli_error(
        ctx.format,
        "No active session to attach.",
        Some(
            "Run `agent-tui sessions list`, then `agent-tui sessions switch <id>`, or pass --session."
                .to_string(),
        ),
        None,
    )
    .into())
}

pub(crate) fn handle_session_switch<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
) -> HandlerResult {
    let success_message = format!("Active session set to {}", Colors::session_id(&session_id));
    let params = params::SessionParams {
        session: Some(session_id),
    };
    let result = call_with_params(ctx.client, "attach", params)?;
    let success = result.bool_or("success", false);
    let result_session_id = result.str_or("session_id", "unknown");
    let message = if success {
        format!("Switched active session to {result_session_id}")
    } else {
        result.str_or("message", "Unknown error").to_string()
    };
    let switch_result = RpcValue::new(serde_json::json!({
        "success": success,
        "session_id": result_session_id,
        "message": message
    }));
    ctx.output_success_and_ok(&switch_result, &success_message, "Switch failed")
}

pub(crate) fn handle_live_start<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    args: LiveStartArgs,
) -> HandlerResult {
    handle_live_start_standalone(ctx.format, args)
}

pub(crate) fn handle_live_start_standalone(
    format: OutputFormat,
    args: LiveStartArgs,
) -> HandlerResult {
    let state_path = ws_state_path();
    if read_ws_state_running(&state_path).is_none() {
        match UnixSocketClient::connect_local() {
            Ok(_) => {}
            Err(ClientError::DaemonNotRunning) => start_daemon_background()?,
            Err(err) => return Err(err.into()),
        }
    }

    let state = wait_for_ws_state(&state_path, Duration::from_secs(3)).ok_or_else(|| {
        CliError::new(
            format,
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
        .filter(|url| !url.is_empty());

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct LiveStartOutput<'a> {
                running: bool,
                pid: u32,
                listen: &'a str,
                ws_url: &'a str,
                ui_url: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                started_at: Option<u64>,
            }

            let output = LiveStartOutput {
                running: true,
                pid: state.pid,
                listen: &state.listen,
                ws_url: &state.ws_url,
                ui_url: &daemon_ui_url,
                started_at: state.started_at,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("WS: {}", state.ws_url);
            println!("UI: {daemon_ui_url}");
        }
    }

    if args.open {
        let open_base_url = open_base_url.as_deref().unwrap_or(&daemon_ui_url);
        match build_ui_url(open_base_url, &state) {
            Ok(target) => {
                if let Err(err) = open_in_browser(&target, args.browser.as_deref()) {
                    eprintln!("Warning: failed to open browser: {err}");
                }
            }
            Err(err) => {
                eprintln!("Warning: failed to build live preview URL: {err}");
            }
        }
    }

    Ok(())
}

pub(crate) fn handle_live_stop<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    handle_live_stop_standalone(ctx.format)
}

pub(crate) fn handle_live_stop_standalone(format: OutputFormat) -> HandlerResult {
    let ui_result = stop_ui_server();
    let ui_error = ui_result
        .as_ref()
        .err()
        .map(std::string::ToString::to_string);
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
                reason:
                    "live preview is served by the daemon; run `agent-tui daemon stop --yes` to stop it."
                        .to_string(),
                ui: ui_payload,
            };
            if let Some(err) = ui_error {
                return Err(CliError::new(
                    format,
                    err,
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
                    return Err(CliError::new(
                        format,
                        err.to_string(),
                        None,
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
            println!(
                "Live preview is served by the daemon; run 'agent-tui daemon stop --yes' to stop."
            );
        }
    }
    Ok(())
}

pub(crate) fn handle_live_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    handle_live_status_standalone(ctx.format)
}

pub(crate) fn handle_live_status_standalone(format: OutputFormat) -> HandlerResult {
    let status = read_ws_state_running(&ws_state_path());
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
                    println!("UI override: {url} (external)");
                }
            } else {
                println!("Live preview: not running");
                match ui_status {
                    UiStatus::External(url) => {
                        println!("UI: {url} (external)");
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
    let size = TerminalSize::try_new(cols, rows).map_err(|err| {
        usage_cli_error(
            ctx.format,
            format!("Invalid terminal size: {err}"),
            Some("Use a terminal size within the supported range.".to_string()),
            Some(serde_json::json!({ "cols": cols, "rows": rows })),
        )
    })?;
    let rpc_params = params::ResizeParams {
        size,
        session: ctx.session.clone(),
    };
    let result = call_with_params(ctx.client, "resize", rpc_params)?;

    ctx.output_json_or(&result, || {
        println!(
            "Session {} resized to {}x{}",
            Colors::session_id(result.str_or("session_id", "?")),
            size.cols(),
            size.rows()
        );
    })
}

pub(crate) fn handle_version_standalone(format: OutputFormat) -> HandlerResult {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");

    let (daemon_version, daemon_commit, daemon_error) =
        match crate::infra::ipc::UnixSocketClient::connect_local() {
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
            println!("  CLI version: {cli_version}");
            println!("  CLI commit: {cli_commit}");
            if let Some(err) = &daemon_error {
                println!(
                    "  Daemon version: {} ({})",
                    Colors::dim(&daemon_version),
                    Colors::error(err)
                );
            } else {
                println!("  Daemon version: {daemon_version}");
                println!("  Daemon commit: {daemon_commit}");
            }
        }
    }
    Ok(())
}

pub(crate) fn handle_daemon_start_standalone(format: OutputFormat) -> HandlerResult {
    let changed = match UnixSocketClient::connect_local() {
        Ok(_) => false,
        Err(err) if can_autostart_after_local_connect_error(&err) => {
            match start_daemon_background() {
                Ok(()) => true,
                Err(ClientError::DaemonNotRunning) => {
                    return Err(structured_cli_error(
                        format,
                        super::exit_codes::GENERAL_ERROR,
                        "Daemon failed to start in background.",
                        "external",
                        Some(
                            "Inspect the daemon log or rerun `agent-tui daemon run` for foreground diagnostics."
                                .to_string(),
                        ),
                        None,
                    )
                    .into());
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(err) => return Err(err.into()),
    };

    let ws_state = read_ws_state_running(&ws_state_path());
    let pid = daemon_pid(ws_state.as_ref());

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct DaemonStartOutput {
                action: &'static str,
                running: bool,
                changed: bool,
                socket_path: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                pid: Option<u32>,
                #[serde(skip_serializing_if = "Option::is_none")]
                listen: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                ws_url: Option<String>,
                #[serde(skip_serializing_if = "Option::is_none")]
                ui_url: Option<String>,
            }

            let output = DaemonStartOutput {
                action: "daemon_start",
                running: true,
                changed,
                socket_path: socket_path().display().to_string(),
                pid,
                listen: ws_state.as_ref().map(|state| state.listen.clone()),
                ws_url: ws_state.as_ref().map(|state| state.ws_url.clone()),
                ui_url: ws_state.as_ref().map(WsState::resolved_ui_url),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            let message = if changed {
                "Daemon started"
            } else {
                "Daemon already running"
            };
            create_presenter(&format).present_success(message, None);
        }
    }

    Ok(())
}

pub(crate) fn handle_daemon_stop_standalone(
    format: OutputFormat,
    force: bool,
    dry_run: bool,
    yes: bool,
    no_input: bool,
) -> HandlerResult {
    let ws_state = read_ws_state_running(&ws_state_path());
    let pid = daemon_pid(ws_state.as_ref());

    if dry_run {
        #[derive(Serialize)]
        struct DaemonStopPreview {
            action: &'static str,
            dry_run: bool,
            force: bool,
            running: bool,
            would_change: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            pid: Option<u32>,
        }

        let output = DaemonStopPreview {
            action: "daemon_stop",
            dry_run: true,
            force,
            running: pid.is_some(),
            would_change: pid.is_some(),
            pid,
        };
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&output)?),
            OutputFormat::Text => {
                if let Some(pid) = pid {
                    println!("Would stop daemon (pid {pid}).");
                } else {
                    println!("Daemon is not running; nothing would change.");
                }
            }
        }
        return Ok(());
    }

    if let Some(pid) = pid {
        let prompt = if force {
            format!("Force-stop daemon (pid {pid})?")
        } else {
            format!("Stop daemon (pid {pid})?")
        };
        confirm_destructive_action(format, no_input, yes, &prompt)?;
    }

    match stop_daemon_core(force)? {
        StopResult::Stopped { pid, warnings } => match format {
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct DaemonStopOutput {
                    action: &'static str,
                    running: bool,
                    changed: bool,
                    force: bool,
                    pid: u32,
                    warnings: Vec<String>,
                }

                let output = DaemonStopOutput {
                    action: "daemon_stop",
                    running: false,
                    changed: true,
                    force,
                    pid,
                    warnings,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Text => {
                for warning in &warnings {
                    eprintln!("{}", Colors::warning(warning));
                }
                create_presenter(&format).present_success("Daemon stopped", None);
            }
        },
        StopResult::AlreadyStopped => match format {
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct DaemonStopOutput {
                    action: &'static str,
                    running: bool,
                    changed: bool,
                    force: bool,
                    warnings: Vec<String>,
                }

                let output = DaemonStopOutput {
                    action: "daemon_stop",
                    running: false,
                    changed: false,
                    force,
                    warnings: Vec::new(),
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            OutputFormat::Text => println!("Daemon is not running (already stopped)"),
        },
    }
    Ok(())
}

pub(crate) fn handle_daemon_restart_standalone(
    format: OutputFormat,
    dry_run: bool,
    yes: bool,
    no_input: bool,
) -> HandlerResult {
    let ws_state = read_ws_state_running(&ws_state_path());
    let pid = daemon_pid(ws_state.as_ref());
    let was_running = pid.is_some();

    if dry_run {
        #[derive(Serialize)]
        struct DaemonRestartPreview {
            action: &'static str,
            dry_run: bool,
            running: bool,
            would_change: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            pid: Option<u32>,
        }

        let output = DaemonRestartPreview {
            action: "daemon_restart",
            dry_run: true,
            running: was_running,
            would_change: true,
            pid,
        };
        match format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&output)?),
            OutputFormat::Text => {
                if let Some(pid) = pid {
                    println!("Would restart daemon (pid {pid}).");
                } else {
                    println!("Would start the daemon.");
                }
            }
        }
        return Ok(());
    }

    if was_running {
        confirm_destructive_action(
            format,
            no_input,
            yes,
            "Restart daemon and terminate active sessions?",
        )?;
    }

    if let OutputFormat::Text = format {
        create_presenter(&format).present_info("Restarting daemon...");
    }

    let warnings = restart_daemon_core()?;
    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct DaemonRestartOutput {
                action: &'static str,
                running: bool,
                changed: bool,
                was_running: bool,
                warnings: Vec<String>,
            }

            let output = DaemonRestartOutput {
                action: "daemon_restart",
                running: true,
                changed: true,
                was_running,
                warnings,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            for warning in &warnings {
                eprintln!("{}", Colors::warning(warning));
            }
            create_presenter(&format).present_success("Daemon restarted", None);
        }
    }
    Ok(())
}

pub(crate) fn handle_daemon_status_standalone(format: OutputFormat) -> HandlerResult {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");
    let ws_state = read_ws_state_running(&ws_state_path());

    let mut client = match UnixSocketClient::connect_local() {
        Ok(client) => client,
        Err(ClientError::DaemonNotRunning) => {
            match format {
                OutputFormat::Json => {
                    #[derive(Serialize)]
                    struct DaemonStatusOutput {
                        running: bool,
                        socket_path: String,
                        cli_version: &'static str,
                        cli_commit: &'static str,
                    }

                    let output = DaemonStatusOutput {
                        running: false,
                        socket_path: socket_path().display().to_string(),
                        cli_version,
                        cli_commit,
                    };
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                OutputFormat::Text => {
                    println!("Daemon is not running");
                    println!("  Socket: {}", socket_path().display());
                    println!("  CLI version: {cli_version}");
                    println!("  CLI commit: {cli_commit}");
                }
            }
            return Err(DaemonNotRunningError.into());
        }
        Err(err) => return Err(err.into()),
    };

    let version = call_no_params(&mut client, "version")?;
    let session_count = call_no_params(&mut client, "sessions")
        .ok()
        .and_then(|result| {
            result
                .get("sessions")
                .and_then(|sessions| sessions.as_array())
                .map(|sessions| sessions.iter().count() as u64)
        });

    let daemon_version = version.str_or("daemon_version", "unknown").to_string();
    let daemon_commit = version.str_or("daemon_commit", "unknown").to_string();
    let pid = daemon_pid(ws_state.as_ref());
    let uptime = ws_state.as_ref().and_then(|state| {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        Some(format_elapsed_secs(
            now.saturating_sub(state.started_at.unwrap_or(now)),
        ))
    });
    let version_mismatch = daemon_version != cli_version;

    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct WsStatusOutput {
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
            }

            #[derive(Serialize)]
            struct DaemonStatusOutput {
                running: bool,
                #[serde(skip_serializing_if = "Option::is_none")]
                pid: Option<u32>,
                socket_path: String,
                cli_version: &'static str,
                cli_commit: &'static str,
                daemon_version: String,
                daemon_commit: String,
                version_mismatch: bool,
                #[serde(skip_serializing_if = "Option::is_none")]
                session_count: Option<u64>,
                ws: WsStatusOutput,
            }

            let output = DaemonStatusOutput {
                running: true,
                pid,
                socket_path: socket_path().display().to_string(),
                cli_version,
                cli_commit,
                daemon_version,
                daemon_commit,
                version_mismatch,
                session_count,
                ws: match ws_state {
                    Some(state) => {
                        let ui_url = state.resolved_ui_url();
                        WsStatusOutput {
                            running: true,
                            pid: Some(state.pid),
                            listen: Some(state.listen),
                            ws_url: Some(state.ws_url),
                            ui_url: Some(ui_url),
                            started_at: state.started_at,
                        }
                    }
                    None => WsStatusOutput {
                        running: false,
                        pid: None,
                        listen: None,
                        ws_url: None,
                        ui_url: None,
                        started_at: None,
                    },
                },
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            if let Some(pid) = pid {
                println!("Daemon is running (pid {pid})");
            } else {
                println!("Daemon is running");
            }
            println!("  Socket: {}", socket_path().display());
            println!("  CLI version: {cli_version}");
            println!("  CLI commit: {cli_commit}");
            println!("  Daemon version: {daemon_version}");
            println!("  Daemon commit: {daemon_commit}");
            if version_mismatch {
                println!(
                    "  Version status: {}",
                    Colors::warning("CLI/daemon mismatch")
                );
            }
            if let Some(session_count) = session_count {
                println!("  Sessions: {session_count}");
            }
            if let Some(uptime) = uptime {
                println!("  Uptime: {uptime}");
            }
            if let Some(state) = ws_state {
                println!("  Listen: {}", state.listen);
                println!("  WS: {}", state.ws_url);
                println!("  UI: {}", state.resolved_ui_url());
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct WsState {
    pid: u32,
    ws_url: String,
    #[serde(default)]
    ui_url: Option<String>,
    listen: String,
    #[serde(default)]
    started_at: Option<u64>,
    #[serde(default)]
    process_started_at: Option<u64>,
    #[serde(default)]
    http_url: Option<String>,
}

impl WsState {
    fn process_identity(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: self.pid,
            started_at: self.process_started_at,
        }
    }

    fn resolved_ui_url(&self) -> String {
        if let Some(url) = self
            .ui_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
        {
            return url.to_string();
        }
        if let Some(http_url) = self
            .http_url
            .as_deref()
            .map(str::trim)
            .filter(|http_url| !http_url.is_empty())
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
    process_started_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct UiStateFile {
    pid: u32,
    url: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    process_started_at: Option<u64>,
}

impl UiState {
    fn process_identity(&self) -> Option<ProcessIdentity> {
        self.process_started_at.map(|started_at| ProcessIdentity {
            pid: self.pid,
            started_at: Some(started_at),
        })
    }
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
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    home.join(".agent-tui").join("api.json")
}

fn read_ws_state(path: &PathBuf) -> Option<WsState> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn process_running(expected: ProcessIdentity) -> bool {
    let controller = UnixProcessController;
    process_running_with_controller(&controller, expected)
}

fn expected_ws_process_identity(state: &WsState) -> Option<ProcessIdentity> {
    match get_daemon_process_identity() {
        DaemonProcessLookupResult::Found(identity) if identity.pid == state.pid => Some(identity),
        DaemonProcessLookupResult::Found(_) | DaemonProcessLookupResult::InvalidState { .. } => {
            None
        }
        DaemonProcessLookupResult::NotRunning => Some(state.process_identity()),
    }
}

fn read_ws_state_running(path: &PathBuf) -> Option<WsState> {
    let state = read_ws_state(path)?;
    let Some(expected) = expected_ws_process_identity(&state) else {
        let _ = std::fs::remove_file(path);
        return None;
    };
    if process_running(expected) {
        Some(state)
    } else {
        let _ = std::fs::remove_file(path);
        None
    }
}

fn wait_for_ws_state(path: &PathBuf, timeout: Duration) -> Option<WsState> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(state) = read_ws_state_running(path) {
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
        .unwrap_or_else(|_| std::env::temp_dir());
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
        process_started_at: file.process_started_at,
    })
}

fn read_ui_state_running(path: &PathBuf) -> Option<UiState> {
    let state = read_ui_state(path)?;
    let Some(expected) = state.process_identity() else {
        let _ = std::fs::remove_file(path);
        return None;
    };
    if process_running(expected) {
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
    match std::env::var("AGENT_TUI_UI_URL") {
        Ok(url) if !url.trim().is_empty() => return UiStatus::External(url),
        _ => {}
    }
    match read_ui_state_running(&ui_state_path()) {
        Some(state) => UiStatus::Running(state),
        None => UiStatus::NotRunning,
    }
}

fn remove_ws_state_file() {
    let state_path = ws_state_path();
    match std::fs::remove_file(&state_path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            warn!(
                path = %state_path.display(),
                error = %err,
                "Failed to remove WS state file"
            );
        }
        _ => {}
    }
}

fn wait_for_process_exit_with_controller<C: ProcessController>(
    controller: &C,
    expected: ProcessIdentity,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match check_expected_process(controller, expected) {
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

fn process_running_with_controller<C: ProcessController>(
    controller: &C,
    expected: ProcessIdentity,
) -> bool {
    matches!(
        check_expected_process(controller, expected),
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
    match std::env::var("AGENT_TUI_UI_URL") {
        Ok(url) if !url.trim().is_empty() => return Ok(StopUiResult::External),
        _ => {}
    }

    let state_path = ui_state_path();
    let Some(state) = read_ui_state(&state_path) else {
        return Ok(StopUiResult::AlreadyStopped);
    };
    let Some(expected) = state.process_identity() else {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::AlreadyStopped);
    };

    if !process_running_with_controller(controller, expected) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::AlreadyStopped);
    }

    controller
        .send_signal(state.pid, Signal::Term)
        .with_context(|| format!("Failed to stop UI server (pid {})", state.pid))?;

    if wait_for_process_exit_with_controller(controller, expected, term_timeout) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::Stopped);
    }

    controller
        .send_signal(state.pid, Signal::Kill)
        .with_context(|| format!("Failed to force-stop UI server (pid {})", state.pid))?;

    if wait_for_process_exit_with_controller(controller, expected, kill_timeout) {
        let _ = std::fs::remove_file(&state_path);
        return Ok(StopUiResult::Stopped);
    }

    Err(anyhow::anyhow!(
        "UI server did not stop after SIGTERM/SIGKILL"
    ))
}

fn ui_urls_share_origin(left: &url::Url, right: &url::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn build_ui_url(base: &str, state: &WsState) -> Result<String> {
    let (base, fragment) = base.split_once('#').unwrap_or((base, ""));
    let daemon_ui_url = state.resolved_ui_url();
    let daemon_url = url::Url::parse(&daemon_ui_url)
        .with_context(|| format!("Invalid daemon UI URL: {daemon_ui_url}"))?;

    let mut url = if let Ok(parsed) = url::Url::parse(base) {
        if !ui_urls_share_origin(&parsed, &daemon_url) {
            anyhow::bail!("AGENT_TUI_UI_URL must use the same origin as the local live preview UI");
        }
        parsed
    } else {
        daemon_url
            .join(base)
            .with_context(|| format!("Invalid AGENT_TUI_UI_URL override: {base}"))?
    };

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
    Ok(url.to_string())
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
            "Browser command exited with status {status}"
        ))
    }
}

pub(crate) fn handle_cleanup<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    all: bool,
    dry_run: bool,
    yes: bool,
) -> HandlerResult {
    use crate::adapters::presenter::CleanupFailure;
    use crate::adapters::presenter::CleanupResult;

    let sessions_result = call_no_params(ctx.client, "sessions")?;
    let sessions = sessions_result.get("sessions").and_then(|v| v.as_array());

    let target_ids: Vec<String> = if let Some(sessions) = sessions {
        sessions
            .iter()
            .filter(|session| all || !session.bool_or("running", false))
            .filter_map(|session| session.get("id").and_then(|v| v.as_str()))
            .map(ToOwned::to_owned)
            .collect()
    } else {
        Vec::new()
    };

    if dry_run {
        #[derive(Serialize)]
        struct CleanupPreview<'a> {
            action: &'static str,
            dry_run: bool,
            all: bool,
            sessions_cleaned: usize,
            session_ids: &'a [String],
        }

        let output = CleanupPreview {
            action: "sessions_cleanup",
            dry_run: true,
            all,
            sessions_cleaned: target_ids.len(),
            session_ids: &target_ids,
        };

        return ctx.output_json_or(&RpcValue::new(serde_json::to_value(output)?), || {
            if target_ids.is_empty() {
                println!("No sessions would be cleaned.");
            } else {
                println!("Would clean up {} session(s):", target_ids.len());
                for id in &target_ids {
                    println!("  {}", Colors::session_id(id));
                }
            }
        });
    }

    if !target_ids.is_empty() {
        let prompt = if all {
            format!(
                "Clean up {} session(s), including running sessions?",
                target_ids.len()
            )
        } else {
            format!("Clean up {} stopped/orphaned session(s)?", target_ids.len())
        };
        confirm_destructive_action(ctx.format, ctx.no_input, yes, &prompt)?;
    }

    let mut cleaned = 0;
    let mut failures: Vec<CleanupFailure> = Vec::new();

    if let Some(sessions) = sessions {
        for session in sessions.iter() {
            if let Some(id) = session
                .get("id")
                .and_then(|v| v.as_str())
                .filter(|id| target_ids.iter().any(|candidate| candidate == id))
            {
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
    struct CleanupFailureJson<'a> {
        session: &'a str,
        error: &'a str,
    }

    #[derive(Serialize)]
    struct CleanupOutputJson<'a> {
        sessions_cleaned: usize,
        sessions_failed: usize,
        failures: Vec<CleanupFailureJson<'a>>,
    }

    let output = CleanupOutputJson {
        sessions_cleaned: result.cleaned,
        sessions_failed: result.failures.len(),
        failures: result
            .failures
            .iter()
            .map(|f| CleanupFailureJson {
                session: &f.session_id,
                error: &f.error,
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
                format!("Failed to attach to session: {session_id}"),
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
                        format!("Failed to attach to session: {session_id}"),
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
                        format!("Failed to attach to session: {session_id}"),
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
            "AGENT_TUI_NO_INPUT",
            std::env::var("AGENT_TUI_NO_INPUT").ok(),
        ),
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
                transport.map(std::string::String::as_str).unwrap_or("unix")
            );
            println!("  Socket: {}", socket_path().display());
            println!();
            println!("{}", Colors::bold("Environment Variables:"));
            for (name, value) in &vars {
                let val_str = value
                    .as_ref()
                    .map(std::string::String::as_str)
                    .unwrap_or("(not set)");
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
    let (cond_type, cond_value) = condition.split_once(':').ok_or_else(|| {
        CliError::new(
            ctx.format,
            "Invalid condition format. Use: text:pattern or session:id",
            None,
            super::exit_codes::USAGE,
        )
    })?;

    let passed = match cond_type {
        "text" => {
            let params = params::SnapshotParams {
                session: ctx.session.clone(),
                region: None,
                strip_ansi: true,
                retain_ansi: false,
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
                format!("Unknown condition type: {cond_type}. Use: text or session"),
                None,
                super::exit_codes::USAGE,
            )
            .into());
        }
    };

    if passed {
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
                let assert_result = crate::adapters::presenter::AssertResult { passed, condition };
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
            format!("Assertion failed: {condition}"),
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
    use crate::infra::ipc::DaemonProcessLookupResult;
    use crate::infra::ipc::daemon_lifecycle;
    use crate::infra::ipc::get_daemon_process_identity;
    use crate::infra::ipc::start_daemon_background;

    let daemon = match get_daemon_process_identity() {
        DaemonProcessLookupResult::Found(identity) => Some(identity),
        DaemonProcessLookupResult::NotRunning => None,
        DaemonProcessLookupResult::InvalidState { path, message } => {
            return Err(anyhow::Error::new(ClientError::DaemonStateInvalid {
                path: path.display().to_string(),
                message,
            }));
        }
    };

    let controller = UnixProcessController;
    let get_pid = move || daemon;
    let restart_warnings = daemon_lifecycle::restart_daemon(
        &controller,
        get_pid,
        &socket_path(),
        start_daemon_background,
    )?;
    Ok(restart_warnings)
}

/// Core daemon stop logic that doesn't require an active client connection.
/// Returns `Ok(StopResult)` on success, including when daemon is already stopped (idempotent).
pub(crate) fn stop_daemon_core(force: bool) -> Result<StopResult> {
    use crate::infra::ipc::DaemonProcessLookupResult;
    use crate::infra::ipc::UnixSocketClient;
    use crate::infra::ipc::daemon_lifecycle;
    use crate::infra::ipc::get_daemon_process_identity;

    let daemon = match get_daemon_process_identity() {
        DaemonProcessLookupResult::Found(identity) => identity,
        DaemonProcessLookupResult::NotRunning => {
            remove_ws_state_file();
            return Ok(StopResult::AlreadyStopped);
        }
        DaemonProcessLookupResult::InvalidState { path, message } => {
            return Err(anyhow::Error::new(ClientError::DaemonStateInvalid {
                path: path.display().to_string(),
                message,
            }));
        }
    };
    let pid = daemon.pid;

    let socket = socket_path();

    let controller = UnixProcessController;
    let stop_result = match daemon_lifecycle::stop_daemon_graceful(
        UnixSocketClient::connect_local,
        &controller,
        daemon,
        &socket,
        force,
    ) {
        Ok(result) => StopResult::Stopped {
            pid,
            warnings: result.warnings,
        },
        Err(ClientError::DaemonNotRunning) => StopResult::AlreadyStopped,
        Err(err) => return Err(err.into()),
    };

    remove_ws_state_file();
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
#[path = "handlers_tests.rs"]
mod tests;
