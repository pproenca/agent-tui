use std::collections::HashMap;
use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use serde_json::json;

use crate::common::Colors;
use crate::common::ValueExt;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;
use crate::infra::ipc::ProcessController;
use crate::infra::ipc::UnixProcessController;
use crate::infra::ipc::params;
use crate::infra::ipc::socket_path;

use crate::adapters::presenter::{ElementView, Presenter, create_presenter};
use crate::app::attach::DetachKeys;
use crate::app::commands::FindParams;
use crate::app::commands::LiveStartArgs;
use crate::app::commands::OutputFormat;
use crate::app::commands::ScrollDirection;
use crate::app::commands::WaitParams;
use crate::app::error::{AttachError, CliError};

pub type HandlerResult = Result<(), Box<dyn std::error::Error>>;

fn format_uptime_ms(uptime_ms: u64) -> String {
    let secs = uptime_ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins % 60, secs % 60)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

macro_rules! key_handler {
    ($name:ident, $method:literal, $success:expr) => {
        pub fn $name<C: DaemonClient>(ctx: &mut HandlerContext<C>, key: String) -> HandlerResult {
            let params = ctx.params_with(json!({ "key": key }));
            let result = ctx.client.call($method, Some(params))?;
            ctx.output_success_and_ok(&result, &$success(&key), concat!($method, " failed"))
        }
    };
}

macro_rules! ref_action_handler {
    ($name:ident, $method:literal, $success:expr, $failure:literal) => {
        pub fn $name<C: DaemonClient>(
            ctx: &mut HandlerContext<C>,
            element_ref: String,
        ) -> HandlerResult {
            ctx.call_ref_action($method, &element_ref, &$success(&element_ref), $failure)
        }
    };
}

pub fn resolve_wait_condition(params: &WaitParams) -> (Option<String>, Option<String>) {
    if params.stable {
        return (Some("stable".to_string()), None);
    }

    if let Some(ref elem) = params.element {
        let condition = if params.gone {
            "not_visible"
        } else {
            "element"
        };
        return (Some(condition.to_string()), Some(elem.clone()));
    }

    if let Some(ref elem) = params.focused {
        return (Some("focused".to_string()), Some(elem.clone()));
    }

    if let Some(ref val) = params.value {
        return (Some("value".to_string()), Some(val.clone()));
    }

    if let Some(ref txt) = params.text {
        if params.gone {
            return (Some("text_gone".to_string()), Some(txt.clone()));
        }
    }

    (None, None)
}

pub struct HandlerContext<'a, C: DaemonClient> {
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
        result: &Value,
        success_msg: &str,
        failure_prefix: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
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
                        Some(result.clone()),
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
                        Some(result.clone()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
        }
        Ok(true)
    }

    fn ref_params(&self, element_ref: &str) -> Value {
        json!({ "ref": element_ref, "session": self.session })
    }

    fn params_with(&self, extra: Value) -> Value {
        let mut p = extra;
        p["session"] = json!(self.session);
        p
    }

    fn call_ref_action(
        &mut self,
        method: &str,
        element_ref: &str,
        success_msg: &str,
        failure_prefix: &str,
    ) -> HandlerResult {
        let params = self.ref_params(element_ref);
        let result = self.client.call(method, Some(params))?;
        self.output_success_result(&result, success_msg, failure_prefix)?;
        Ok(())
    }

    fn session_params(&self) -> Value {
        json!({ "session": self.session })
    }

    fn output_json_or<F>(&self, result: &Value, text_fn: F) -> HandlerResult
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
        result: &Value,
        success_msg: &str,
        failure_prefix: &str,
    ) -> HandlerResult {
        self.output_success_result(result, success_msg, failure_prefix)?;
        Ok(())
    }

    pub fn display_error(&self, error: &ClientError) {
        self.presenter.present_client_error(error);
    }
}

pub fn handle_spawn<C: DaemonClient>(
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
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("spawn", Some(params))?;

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

pub fn handle_snapshot<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    elements: bool,
    region: Option<String>,
    strip_ansi: bool,
    include_cursor: bool,
) -> HandlerResult {
    let rpc_params = params::SnapshotParams {
        session: ctx.session.clone(),
        include_elements: elements,
        region,
        strip_ansi,
        include_cursor,
        include_render: false,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("snapshot", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text => {
            if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
                if !elements.is_empty() {
                    println!("{}", Colors::bold("Elements:"));
                    for el in elements {
                        let ev = ElementView(el);
                        let (row, col) = ev.position();
                        let value = ev
                            .value()
                            .map(|v| format!(" \"{}\"", v))
                            .unwrap_or_default();

                        println!(
                            "{} [{}{}]{} {}{}{}",
                            Colors::element_ref(ev.ref_str()),
                            ev.el_type(),
                            ev.label_suffix(),
                            value,
                            Colors::dim(&format!("({},{})", row, col)),
                            ev.focused_indicator(),
                            ev.selected_indicator()
                        );
                    }
                    println!();
                }
            }
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

pub fn handle_accessibility_snapshot<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    interactive_only: bool,
) -> HandlerResult {
    let rpc_params = params::AccessibilitySnapshotParams {
        session: ctx.session.clone(),
        interactive: interactive_only,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("accessibility_snapshot", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text => {
            if let Some(tree) = result.get("tree").and_then(|v| v.as_str()) {
                println!("{}", tree);
            }
        }
    }
    Ok(())
}

ref_action_handler!(
    handle_click,
    "click",
    |_: &String| "Clicked successfully".to_string(),
    "Click failed"
);
ref_action_handler!(
    handle_dbl_click,
    "dbl_click",
    |_: &String| "Double-clicked successfully".to_string(),
    "Double-click failed"
);

pub fn handle_fill<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
    value: String,
) -> HandlerResult {
    let params = ctx.params_with(json!({ "ref": element_ref, "value": value }));
    let result = ctx.client.call("fill", Some(params))?;
    ctx.output_success_and_ok(&result, "Filled successfully", "Fill failed")
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

pub fn handle_type<C: DaemonClient>(ctx: &mut HandlerContext<C>, text: String) -> HandlerResult {
    let params = ctx.params_with(json!({ "text": text }));
    let result = ctx.client.call("type", Some(params))?;
    ctx.output_success_and_ok(&result, "Text typed", "Type failed")
}

pub fn handle_wait<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    wait_params: WaitParams,
) -> HandlerResult {
    use crate::adapters::presenter::WaitResult;

    let (cond, tgt) = resolve_wait_condition(&wait_params);
    let rpc_params = params::WaitParams {
        session: ctx.session.clone(),
        text: wait_params.text.clone(),
        timeout_ms: wait_params.timeout,
        condition: cond,
        target: tgt,
    };
    let params_json = serde_json::to_value(rpc_params)?;
    let result = ctx.client.call("wait", Some(params_json))?;

    let wait_result = WaitResult::from_json(&result);

    if wait_params.assert && !wait_result.found {
        return Err(CliError::new(
            ctx.format,
            "Wait condition not met within timeout",
            Some(result.clone()),
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

pub fn handle_kill<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = ctx.client.call("kill", Some(ctx.session_params()))?;

    ctx.output_json_or(&result, || {
        println!("Session {} killed", result.str_or("session_id", "unknown"));
    })
}

pub fn handle_restart<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = ctx.client.call("restart", Some(ctx.session_params()))?;

    ctx.output_json_or(&result, || {
        println!(
            "Restarted '{}': {} -> {}",
            result.str_or("command", "unknown"),
            result.str_or("old_session_id", "unknown"),
            result.str_or("new_session_id", "unknown")
        );
    })
}

pub fn handle_sessions<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = ctx.client.call("sessions", None)?;

    ctx.output_json_or(&result, || {
        let active_id = result.get("active_session").and_then(|v| v.as_str());

        match result.get("sessions").and_then(|v| v.as_array()) {
            Some(sessions) if !sessions.is_empty() => {
                println!("{}", Colors::bold("Active sessions:"));
                for session in sessions {
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

pub fn handle_session_show<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
) -> HandlerResult {
    let result = ctx.client.call("sessions", None)?;
    let active_id = result.get("active_session").and_then(|v| v.as_str());
    let sessions = result
        .get("sessions")
        .and_then(|v| v.as_array())
        .ok_or("Invalid sessions response")?;

    let session = sessions
        .iter()
        .find(|session| session.str_or("id", "") == session_id.as_str())
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    match ctx.format {
        OutputFormat::Json => {
            let payload = json!({
                "session": session,
                "active_session": active_id
            });
            ctx.presenter().present_value(&payload);
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

pub fn resolve_attach_session_id<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(id) = session_id {
        return Ok(id);
    }

    if let Some(id) = ctx.session.clone() {
        return Ok(id);
    }

    let result = ctx.client.call("sessions", None)?;
    if let Some(active) = result.get("active_session").and_then(|v| v.as_str()) {
        return Ok(active.to_string());
    }

    Err("No active session to attach. Use 'agent-tui sessions list' or pass --session.".into())
}

pub fn handle_health<C: DaemonClient>(ctx: &mut HandlerContext<C>, verbose: bool) -> HandlerResult {
    use crate::adapters::presenter::HealthResult;

    let result = ctx.client.call("health", None)?;

    match ctx.format {
        OutputFormat::Json => ctx.presenter().present_value(&result),
        OutputFormat::Text => {
            let health = HealthResult::from_json(&result, verbose);
            ctx.presenter().present_health(&health);
        }
    }
    Ok(())
}

pub fn handle_live_start<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    args: LiveStartArgs,
) -> HandlerResult {
    if args.listen.is_some() || args.allow_remote || args.max_viewers.is_some() {
        eprintln!(
            "{} Live preview is now served by the daemon API. Configure it via:",
            Colors::info("Note:")
        );
        eprintln!("  AGENT_TUI_API_LISTEN / AGENT_TUI_API_ALLOW_REMOTE / AGENT_TUI_API_MAX_CONNECTIONS");
    }

    let state_path = api_state_path();
    let state = wait_for_api_state(&state_path, Duration::from_secs(3)).ok_or_else(|| {
        CliError::new(
            ctx.format,
            "API server is not available. Restart the daemon and try again.".to_string(),
            None,
            super::exit_codes::GENERAL_ERROR,
        )
    })?;

    match ctx.format {
        OutputFormat::Json => {
            let output = json!({
                "running": true,
                "pid": state.pid,
                "listen": state.listen,
                "http_url": state.http_url,
                "ws_url": state.ws_url,
                "token": state.token,
                "api_version": state.api_version,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("API: {}", state.http_url);
            println!("WS: {}", state.ws_url);
            if let Some(api_version) = state.api_version.as_deref() {
                println!("API version: {}", api_version);
            }
            if let Some(token) = state.token.as_deref() {
                println!("Token: {}", token);
            } else {
                println!("Token: (disabled)");
            }
        }
    }

    if args.open {
        let ui_url = std::env::var("AGENT_TUI_UI_URL").ok();
        let target = ui_url.as_deref().unwrap_or(&state.http_url);
        if ui_url.is_none() {
            eprintln!(
                "{} AGENT_TUI_UI_URL not set; opening API URL instead.",
                Colors::warning("Warning:")
            );
        }
        if let Err(err) = open_in_browser(target, args.browser.as_deref()) {
            eprintln!("Warning: failed to open browser: {}", err);
        }
    }

    Ok(())
}

pub fn handle_live_stop<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    match ctx.format {
        OutputFormat::Json => {
            let output = json!({
                "stopped": false,
                "reason": "live preview is served by the daemon; stop the daemon to stop"
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("Live preview is served by the daemon; run 'agent-tui daemon stop' to stop.");
        }
    }
    Ok(())
}

pub fn handle_live_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let status = read_api_state_running(&api_state_path());

    match ctx.format {
        OutputFormat::Json => {
            let output = match status {
                Some(state) => json!({
                    "running": true,
                    "pid": state.pid,
                    "listen": state.listen,
                    "http_url": state.http_url,
                    "ws_url": state.ws_url,
                    "token": state.token,
                    "api_version": state.api_version
                }),
                None => json!({ "running": false }),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            if let Some(state) = status {
                println!("Live preview API: {}", state.http_url);
            } else {
                println!("Live preview: not running");
            }
        }
    }

    Ok(())
}

pub fn handle_resize<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
    let rpc_params = params::ResizeParams {
        cols,
        rows,
        session: ctx.session.clone(),
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("resize", Some(params))?;

    ctx.output_json_or(&result, || {
        println!(
            "Session {} resized to {}x{}",
            Colors::session_id(result.str_or("session_id", "?")),
            cols,
            rows
        );
    })
}

pub fn handle_version<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");

    let (daemon_version, daemon_commit, daemon_error) = match ctx.client.call("health", None) {
        Ok(result) => (
            result
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            result
                .get("commit")
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
    };

    match ctx.format {
        OutputFormat::Json => {
            let mut output = json!({
                "cli_version": cli_version,
                "cli_commit": cli_commit,
                "daemon_version": daemon_version,
                "daemon_commit": daemon_commit,
                "mode": "daemon"
            });
            if let Some(err) = &daemon_error {
                output["daemon_error"] = json!(err);
            }
            println!("{}", output);
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

#[derive(Debug, Clone)]
struct ApiState {
    pid: u32,
    http_url: String,
    ws_url: String,
    listen: String,
    token: Option<String>,
    api_version: Option<String>,
}

fn api_state_path() -> PathBuf {
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
    let value: Value = serde_json::from_str(&contents).ok()?;
    Some(ApiState {
        pid: value.get("pid")?.as_u64()? as u32,
        http_url: value.get("http_url")?.as_str()?.to_string(),
        ws_url: value.get("ws_url")?.as_str()?.to_string(),
        listen: value.get("listen")?.as_str()?.to_string(),
        token: value.get("token").and_then(|v| v.as_str()).map(|s| s.to_string()),
        api_version: value
            .get("api_version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

fn is_api_process_running(pid: u32) -> bool {
    let controller = UnixProcessController;
    matches!(
        controller.check_process(pid),
        Ok(crate::infra::ipc::ProcessStatus::Running)
            | Ok(crate::infra::ipc::ProcessStatus::NoPermission)
    )
}

fn read_api_state_running(path: &PathBuf) -> Option<ApiState> {
    let state = read_api_state(path)?;
    if is_api_process_running(state.pid) {
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
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn open_in_browser(url: &str, browser_override: Option<&str>) -> Result<(), String> {
    use std::process::Command;

    let browser = browser_override
        .map(String::from)
        .or_else(|| std::env::var("BROWSER").ok());

    let mut cmd = if let Some(browser) = browser {
        let mut parts = browser.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| "Browser command is empty".to_string())?;
        let mut cmd = Command::new(program);
        cmd.args(parts);
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

    let status = cmd.arg(url).status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Browser command exited with status {}", status))
    }
}

pub fn handle_cleanup<C: DaemonClient>(ctx: &mut HandlerContext<C>, all: bool) -> HandlerResult {
    use crate::adapters::presenter::{CleanupFailure, CleanupResult};

    let sessions_result = ctx.client.call("sessions", None)?;
    let sessions = sessions_result.get("sessions").and_then(|v| v.as_array());

    let mut cleaned = 0;
    let mut failures: Vec<CleanupFailure> = Vec::new();

    if let Some(sessions) = sessions {
        for session in sessions {
            let id = session.get("id").and_then(|v| v.as_str());
            let should_cleanup = all || !session.bool_or("running", false);
            if should_cleanup {
                if let Some(id) = id {
                    let params = json!({ "session": id });
                    match ctx.client.call("kill", Some(params)) {
                        Ok(_) => cleaned += 1,
                        Err(e) => failures.push(CleanupFailure {
                            session_id: id.to_string(),
                            error: e.to_string(),
                        }),
                    }
                }
            }
        }
    }

    let result = CleanupResult { cleaned, failures };

    let failures_json: Vec<_> = result
        .failures
        .iter()
        .map(|f| json!({"session": f.session_id, "error": f.error}))
        .collect();
    let output = json!({
        "sessions_cleaned": result.cleaned,
        "sessions_failed": result.failures.len(),
        "failures": failures_json
    });

    if result.failures.is_empty() {
        match ctx.format {
            OutputFormat::Json => ctx.presenter().present_value(&output),
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
            Some(output),
            super::exit_codes::GENERAL_ERROR,
        )
        .into());
    }
    Ok(())
}

pub fn handle_find<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    find_params: FindParams,
) -> HandlerResult {
    let focused_opt = if find_params.focused {
        Some(true)
    } else {
        None
    };

    let rpc_params = params::FindParams {
        session: ctx.session.clone(),
        role: find_params.role,
        name: find_params.name,
        text: find_params.text,
        placeholder: find_params.placeholder,
        focused: focused_opt,
        nth: find_params.nth,
        exact: find_params.exact,
    };
    let params_json = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("find", Some(params_json))?;

    match ctx.format {
        OutputFormat::Json => ctx.presenter().present_value(&result),
        OutputFormat::Text => {
            let find_result = crate::adapters::presenter::FindResult::from_json(&result);
            ctx.presenter().present_find(&find_result);
        }
    }
    Ok(())
}

pub fn handle_select<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
    options: Vec<String>,
) -> HandlerResult {
    if options.len() == 1 {
        handle_select_single(ctx, element_ref, options.into_iter().next().unwrap())
    } else {
        handle_select_multiple(ctx, element_ref, options)
    }
}

fn handle_select_single<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
    option: String,
) -> HandlerResult {
    let params = ctx.params_with(json!({ "ref": element_ref, "option": option }));
    let result = ctx.client.call("select", Some(params))?;
    ctx.output_success_and_ok(&result, &format!("Selected: {}", option), "Select failed")
}

fn handle_select_multiple<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
    options: Vec<String>,
) -> HandlerResult {
    let params = ctx.params_with(json!({ "ref": element_ref, "options": options }));

    let result = ctx.client.call("multiselect", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            if result.bool_or("success", false) {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let msg = result.str_or("message", "Unknown error");
                let message = format!("Multiselect failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
        OutputFormat::Text => {
            if result.bool_or("success", false) {
                println!(
                    "Selected: {}",
                    result.str_array_join("selected_options", ", ")
                );
            } else {
                let msg = result.str_or("message", "Unknown error");
                let message = format!("Multiselect failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
    }
    Ok(())
}

pub fn handle_scroll<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    direction: ScrollDirection,
    amount: u16,
) -> HandlerResult {
    let dir_str = direction.as_str();
    let params = ctx.params_with(json!({ "direction": dir_str, "amount": amount }));
    let result = ctx.client.call("scroll", Some(params))?;
    ctx.output_success_and_ok(
        &result,
        &format!("Scrolled {} {} times", dir_str, amount),
        "Scroll failed",
    )
}

pub fn handle_scroll_into_view<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
) -> HandlerResult {
    let params = ctx.params_with(json!({ "ref": element_ref }));
    let result = ctx.client.call("scroll_into_view", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            if result.bool_or("success", false) {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let msg = result.str_or("message", "Element not found");
                let message = format!("Scroll into view failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
        OutputFormat::Text => {
            if result.bool_or("success", false) {
                println!(
                    "Scrolled to {} ({} scrolls)",
                    element_ref,
                    result.u64_or("scrolls_needed", 0)
                );
            } else {
                let msg = result.str_or("message", "Element not found");
                let message = format!("Scroll into view failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
    }
    Ok(())
}

ref_action_handler!(
    handle_focus,
    "focus",
    |r: &String| format!("Focused: {}", r),
    "Focus failed"
);
ref_action_handler!(
    handle_clear,
    "clear",
    |r: &String| format!("Cleared: {}", r),
    "Clear failed"
);
ref_action_handler!(
    handle_select_all,
    "select_all",
    |r: &String| format!("Selected all in: {}", r),
    "Select all failed"
);

pub fn handle_count<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    role: Option<String>,
    name: Option<String>,
    text: Option<String>,
) -> HandlerResult {
    let rpc_params = params::CountParams {
        session: ctx.session.clone(),
        role,
        name,
        text,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("count", Some(params))?;

    ctx.output_json_or(&result, || {
        println!("{}", result.u64_or("count", 0));
    })
}

pub fn handle_toggle<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    element_ref: String,
    state: Option<bool>,
) -> HandlerResult {
    let params = ctx.params_with(json!({ "ref": element_ref, "state": state }));
    let result = ctx.client.call("toggle", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            if result.bool_or("success", false) {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let msg = result.str_or("message", "Unknown error");
                let message = format!("Toggle failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
        OutputFormat::Text => {
            if result.bool_or("success", false) {
                let state = if result.bool_or("checked", true) {
                    "checked"
                } else {
                    "unchecked"
                };
                println!("{} is now {}", element_ref, state);
            } else {
                let msg = result.str_or("message", "Unknown error");
                let message = format!("Toggle failed: {}", msg);
                return Err(CliError::new(
                    ctx.format,
                    message,
                    Some(result.clone()),
                    super::exit_codes::GENERAL_ERROR,
                )
                .into());
            }
        }
    }
    Ok(())
}

pub fn handle_attach<C: DaemonClient>(
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

    let params = json!({ "session": session_id });
    let result = ctx.client.call("attach", Some(params))?;

    if interactive {
        if !result.bool_or("success", false) {
            return Err(CliError::new(
                ctx.format,
                format!("Failed to attach to session: {}", session_id),
                Some(result.clone()),
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
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    return Err(CliError::new(
                        ctx.format,
                        format!("Failed to attach to session: {}", session_id),
                        Some(result.clone()),
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
                        Some(result.clone()),
                        super::exit_codes::GENERAL_ERROR,
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

pub fn handle_env<C: DaemonClient>(ctx: &HandlerContext<C>) -> HandlerResult {
    let vars = [
        (
            "AGENT_TUI_TRANSPORT",
            std::env::var("AGENT_TUI_TRANSPORT").ok(),
        ),
        (
            "AGENT_TUI_TCP_PORT",
            std::env::var("AGENT_TUI_TCP_PORT").ok(),
        ),
        (
            "AGENT_TUI_DETACH_KEYS",
            std::env::var("AGENT_TUI_DETACH_KEYS").ok(),
        ),
        (
            "AGENT_TUI_API_LISTEN",
            std::env::var("AGENT_TUI_API_LISTEN").ok(),
        ),
        (
            "AGENT_TUI_API_ALLOW_REMOTE",
            std::env::var("AGENT_TUI_API_ALLOW_REMOTE").ok(),
        ),
        (
            "AGENT_TUI_API_TOKEN",
            std::env::var("AGENT_TUI_API_TOKEN").ok(),
        ),
        (
            "AGENT_TUI_API_STATE",
            std::env::var("AGENT_TUI_API_STATE").ok(),
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
        ("AGENT_TUI_UI_URL", std::env::var("AGENT_TUI_UI_URL").ok()),
        ("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR").ok()),
        ("NO_COLOR", std::env::var("NO_COLOR").ok()),
    ];

    match ctx.format {
        OutputFormat::Json => {
            let env_map: HashMap<&str, Option<String>> = vars.iter().cloned().collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "environment": env_map,
                    "socket_path": socket_path().display().to_string()
                }))?
            );
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

pub fn handle_assert<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    condition: String,
) -> HandlerResult {
    let parts: Vec<&str> = condition.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(CliError::new(
            ctx.format,
            "Invalid condition format. Use: text:pattern, element:ref, or session:id",
            None,
            super::exit_codes::USAGE,
        )
        .into());
    }

    let (cond_type, cond_value) = (parts[0], parts[1]);

    let passed = match cond_type {
        "text" => {
            let params = json!({
                "session": ctx.session,
                "strip_ansi": true
            });
            let result = ctx.client.call("snapshot", Some(params))?;
            result.str_or("screenshot", "").contains(cond_value)
        }
        "element" => {
            let params = json!({ "ref": cond_value, "session": ctx.session });
            let result = ctx.client.call("is_visible", Some(params))?;
            result.bool_or("visible", false)
        }
        "session" => {
            let result = ctx.client.call("sessions", None)?;
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
                    "Unknown condition type: {}. Use: text, element, or session",
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
                let output = json!({
                    "condition": condition,
                    "passed": passed
                });
                ctx.presenter().present_value(&output);
            }
            OutputFormat::Text => {
                ctx.presenter().present_assert_result(&assert_result);
            }
        }
    } else {
        let output = json!({
            "condition": condition,
            "passed": passed
        });
        return Err(CliError::new(
            ctx.format,
            format!("Assertion failed: {}", assert_result.condition),
            Some(output),
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

/// Core daemon stop logic that doesn't require an active client connection.
/// Returns `Ok(StopResult)` on success, including when daemon is already stopped (idempotent).
pub fn stop_daemon_core(force: bool) -> Result<StopResult, Box<dyn std::error::Error>> {
    use crate::infra::ipc::{PidLookupResult, UnixSocketClient, daemon_lifecycle, get_daemon_pid};

    let pid = match get_daemon_pid() {
        PidLookupResult::Found(pid) => pid,
        PidLookupResult::NotRunning => {
            return Ok(StopResult::AlreadyStopped);
        }
        PidLookupResult::Error(msg) => {
            return Err(Box::new(ClientError::SignalFailed {
                pid: 0,
                message: msg,
            }));
        }
    };

    let socket = socket_path();

    if !force {
        // Try graceful RPC shutdown first (needs connection but doesn't auto-start)
        if let Ok(mut client) = UnixSocketClient::connect() {
            if let Ok(result) = daemon_lifecycle::stop_daemon_via_rpc(&mut client, &socket) {
                return Ok(StopResult::Stopped {
                    pid,
                    warnings: result.warnings,
                });
            }
        }
    }

    // Fall back to signal-based stop
    let controller = UnixProcessController;
    let result = daemon_lifecycle::stop_daemon(&controller, pid, &socket, force)?;
    Ok(StopResult::Stopped {
        pid,
        warnings: result.warnings,
    })
}

pub fn handle_daemon_stop<C: DaemonClient>(
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

pub fn print_daemon_status_from_result(result: &serde_json::Value, format: OutputFormat) {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");
    let daemon_version = result.str_or("version", "unknown");
    let daemon_commit = result.str_or("commit", "unknown");
    let status = result.str_or("status", "unknown");
    let pid = result.u64_or("pid", 0);
    let uptime_ms = result.u64_or("uptime_ms", 0);
    let session_count = result.u64_or("session_count", 0);

    let version_mismatch = cli_version != daemon_version;
    let commit_mismatch =
        cli_commit != "unknown" && daemon_commit != "unknown" && cli_commit != daemon_commit;
    let api_state = read_api_state_running(&api_state_path());

    match format {
        OutputFormat::Json => {
            let api_json = match api_state {
                Some(state) => json!({
                    "running": true,
                    "pid": state.pid,
                    "listen": state.listen,
                    "http_url": state.http_url,
                    "ws_url": state.ws_url,
                    "token": state.token,
                    "api_version": state.api_version
                }),
                None => json!({ "running": false }),
            };
            println!(
                "{}",
                serde_json::json!({
                    "running": true,
                    "status": status,
                    "pid": pid,
                    "uptime_ms": uptime_ms,
                    "session_count": session_count,
                    "daemon_version": daemon_version,
                    "daemon_commit": daemon_commit,
                    "cli_version": cli_version,
                    "cli_commit": cli_commit,
                    "version_mismatch": version_mismatch,
                    "commit_mismatch": commit_mismatch,
                    "api": api_json
                })
            );
        }
        OutputFormat::Text => {
            println!(
                "{} {}",
                Colors::bold("Daemon status:"),
                Colors::success(status)
            );
            println!("  PID: {}", pid);
            println!("  Uptime: {}", format_uptime_ms(uptime_ms));
            println!("  Sessions: {}", session_count);
            println!("  Daemon version: {}", daemon_version);
            println!("  Daemon commit: {}", daemon_commit);
            println!("  CLI version: {}", cli_version);
            println!("  CLI commit: {}", cli_commit);
            if let Some(state) = api_state {
                println!("  API: {}", state.http_url);
                println!("  WS: {}", state.ws_url);
                if let Some(token) = state.token.as_deref() {
                    println!("  API token: {}", token);
                } else {
                    println!("  API token: (disabled)");
                }
            } else {
                println!("  API: not running");
            }

            if version_mismatch {
                eprintln!();
                eprintln!("{} Version mismatch detected!", Colors::warning("⚠"));
                eprintln!(
                    "  Run '{}' to update the daemon.",
                    Colors::info("agent-tui daemon restart")
                );
            } else if commit_mismatch {
                eprintln!();
                eprintln!(
                    "{} Build mismatch detected (commit differs).",
                    Colors::warning("⚠")
                );
                eprintln!(
                    "  Run '{}' to update the daemon.",
                    Colors::info("agent-tui daemon restart")
                );
            }
        }
    }
}

pub fn handle_daemon_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let cli_version = env!("AGENT_TUI_VERSION");
    let cli_commit = env!("AGENT_TUI_GIT_SHA");
    match ctx.client.call("health", None) {
        Ok(result) => print_daemon_status_from_result(&result, ctx.format),
        Err(e) => match ctx.format {
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "running": false,
                        "cli_version": cli_version,
                        "cli_commit": cli_commit,
                        "error": e.to_string()
                    })
                );
            }
            OutputFormat::Text => {
                println!(
                    "{} {} ({})",
                    Colors::bold("Daemon status:"),
                    Colors::error("not running"),
                    Colors::dim(&e.to_string())
                );
                println!("  CLI version: {}", cli_version);
                println!("  CLI commit: {}", cli_commit);
            }
        },
    }
    Ok(())
}

pub fn handle_daemon_restart<C: DaemonClient>(ctx: &HandlerContext<C>) -> HandlerResult {
    use crate::infra::ipc::{
        PidLookupResult, daemon_lifecycle, get_daemon_pid, start_daemon_background,
    };

    if let OutputFormat::Text = ctx.format {
        ctx.presenter().present_info("Restarting daemon...");
    }

    let controller = UnixProcessController;

    let get_pid = || -> Option<u32> {
        match get_daemon_pid() {
            PidLookupResult::Found(pid) => Some(pid),
            PidLookupResult::NotRunning => None,
            PidLookupResult::Error(msg) => {
                eprintln!(
                    "{} Could not read daemon PID: {}",
                    Colors::warning("Warning:"),
                    msg
                );
                None
            }
        }
    };

    let warnings = daemon_lifecycle::restart_daemon(
        &controller,
        get_pid,
        &socket_path(),
        start_daemon_background,
    )?;

    for warning in &warnings {
        eprintln!("{}", Colors::warning(warning));
    }

    ctx.presenter().present_success("Daemon restarted", None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::presenter::{Presenter, TextPresenter};
    use std::cell::RefCell;
    use std::rc::Rc;

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

        fn present_value(&self, value: &Value) {
            self.output.borrow_mut().push(format!("value: {}", value));
        }

        fn present_client_error(&self, error: &crate::infra::ipc::ClientError) {
            self.output
                .borrow_mut()
                .push(format!("client_error: {}", error));
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

        fn present_element_ref(&self, element_ref: &str, info: Option<&str>) {
            self.output
                .borrow_mut()
                .push(format!("ref: {} {:?}", element_ref, info));
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

        fn present_health(&self, health: &crate::adapters::presenter::HealthResult) {
            self.output.borrow_mut().push(format!(
                "health: status={}, pid={}, sessions={}",
                health.status, health.pid, health.session_count
            ));
        }

        fn present_cleanup(&self, result: &crate::adapters::presenter::CleanupResult) {
            self.output.borrow_mut().push(format!(
                "cleanup: cleaned={}, failed={}",
                result.cleaned,
                result.failures.len()
            ));
        }

        fn present_find(&self, result: &crate::adapters::presenter::FindResult) {
            self.output.borrow_mut().push(format!(
                "find: count={}, elements={}",
                result.count,
                result.elements.len()
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

    fn make_element(json: Value) -> Value {
        json
    }

    #[test]
    fn test_element_view_ref_str() {
        let el = make_element(json!({"ref": "@btn1"}));
        let view = ElementView(&el);
        assert_eq!(view.ref_str(), "@btn1");
    }

    #[test]
    fn test_element_view_ref_str_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert_eq!(view.ref_str(), "");
    }

    #[test]
    fn test_element_view_el_type() {
        let el = make_element(json!({"type": "button"}));
        let view = ElementView(&el);
        assert_eq!(view.el_type(), "button");
    }

    #[test]
    fn test_element_view_el_type_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert_eq!(view.el_type(), "");
    }

    #[test]
    fn test_element_view_label() {
        let el = make_element(json!({"label": "Submit"}));
        let view = ElementView(&el);
        assert_eq!(view.label(), "Submit");
    }

    #[test]
    fn test_element_view_label_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert_eq!(view.label(), "");
    }

    #[test]
    fn test_element_view_focused_true() {
        let el = make_element(json!({"focused": true}));
        let view = ElementView(&el);
        assert!(view.focused());
    }

    #[test]
    fn test_element_view_focused_false() {
        let el = make_element(json!({"focused": false}));
        let view = ElementView(&el);
        assert!(!view.focused());
    }

    #[test]
    fn test_element_view_focused_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert!(!view.focused());
    }

    #[test]
    fn test_element_view_selected() {
        let el = make_element(json!({"selected": true}));
        let view = ElementView(&el);
        assert!(view.selected());
    }

    #[test]
    fn test_element_view_selected_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert!(!view.selected());
    }

    #[test]
    fn test_element_view_value_present() {
        let el = make_element(json!({"value": "test input"}));
        let view = ElementView(&el);
        assert_eq!(view.value(), Some("test input"));
    }

    #[test]
    fn test_element_view_value_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert_eq!(view.value(), None);
    }

    #[test]
    fn test_element_view_position() {
        let el = make_element(json!({"position": {"row": 5, "col": 10}}));
        let view = ElementView(&el);
        assert_eq!(view.position(), (5, 10));
    }

    #[test]
    fn test_element_view_position_partial() {
        let el = make_element(json!({"position": {"row": 5}}));
        let view = ElementView(&el);
        assert_eq!(view.position(), (5, 0));
    }

    #[test]
    fn test_element_view_position_missing() {
        let el = make_element(json!({}));
        let view = ElementView(&el);
        assert_eq!(view.position(), (0, 0));
    }

    #[test]
    fn test_element_view_full_element() {
        let el = make_element(json!({
            "ref": "@inp1",
            "type": "input",
            "label": "Email",
            "value": "test@example.com",
            "focused": true,
            "selected": false,
            "checked": null,
            "position": {"row": 3, "col": 15}
        }));
        let view = ElementView(&el);
        assert_eq!(view.ref_str(), "@inp1");
        assert_eq!(view.el_type(), "input");
        assert_eq!(view.label(), "Email");
        assert_eq!(view.value(), Some("test@example.com"));
        assert!(view.focused());
        assert!(!view.selected());
        assert_eq!(view.position(), (3, 15));
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
    fn test_assert_condition_parsing_element() {
        let condition = "element:@btn1";
        let parts: Vec<&str> = condition.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "element");
        assert_eq!(parts[1], "@btn1");
    }

    #[test]
    fn test_assert_condition_parsing_session() {
        let condition = "session:my-session";
        let parts: Vec<&str> = condition.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "session");
        assert_eq!(parts[1], "my-session");
    }

    #[test]
    fn test_assert_condition_parsing_with_colon_in_value() {
        let condition = "text:URL: https://example.com";
        let parts: Vec<&str> = condition.splitn(2, ':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "text");
        assert_eq!(parts[1], "URL: https://example.com");
    }

    #[test]
    fn test_assert_condition_parsing_invalid() {
        let condition = "invalid_format";
        let parts: Vec<&str> = condition.splitn(2, ':').collect();
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_wait_condition_stable() {
        let params = WaitParams {
            stable: true,
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("stable".to_string()));
        assert_eq!(tgt, None);
    }

    #[test]
    fn test_wait_condition_element() {
        let params = WaitParams {
            element: Some("@btn1".to_string()),
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("element".to_string()));
        assert_eq!(tgt, Some("@btn1".to_string()));
    }

    #[test]
    fn test_wait_condition_focused() {
        let params = WaitParams {
            focused: Some("@inp1".to_string()),
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("focused".to_string()));
        assert_eq!(tgt, Some("@inp1".to_string()));
    }

    #[test]
    fn test_wait_condition_element_gone() {
        let params = WaitParams {
            element: Some("@spinner".to_string()),
            gone: true,
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("not_visible".to_string()));
        assert_eq!(tgt, Some("@spinner".to_string()));
    }

    #[test]
    fn test_wait_condition_text_gone() {
        let params = WaitParams {
            text: Some("Loading...".to_string()),
            gone: true,
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("text_gone".to_string()));
        assert_eq!(tgt, Some("Loading...".to_string()));
    }

    #[test]
    fn test_wait_condition_value() {
        let params = WaitParams {
            value: Some("@inp1=hello".to_string()),
            ..Default::default()
        };
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, Some("value".to_string()));
        assert_eq!(tgt, Some("@inp1=hello".to_string()));
    }

    #[test]
    fn test_wait_condition_none() {
        let params = WaitParams::default();
        let (cond, tgt) = resolve_wait_condition(&params);
        assert_eq!(cond, None);
        assert_eq!(tgt, None);
    }
}
