use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use crate::common::Colors;
use crate::common::ValueExt;
use crate::ipc::ClientError;
use crate::ipc::DaemonClient;
use crate::ipc::UnixProcessController;
use crate::ipc::params;
use crate::ipc::socket_path;

use crate::commands::FindParams;
use crate::commands::OutputFormat;
use crate::commands::RecordFormat;
use crate::commands::ScrollDirection;
use crate::commands::WaitParams;
use crate::presenter::{ElementView, Presenter, create_presenter};

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
                self.presenter.present_value(result);
            }
            OutputFormat::Text => {
                if success {
                    let warning = result.get("warning").and_then(|w| w.as_str());
                    self.presenter.present_success(success_msg, warning);
                } else {
                    let msg = result.str_or("message", "Unknown error");
                    self.presenter
                        .present_error(&format!("{}: {}", failure_prefix, msg));
                    std::process::exit(1);
                }
            }
        }
        Ok(success)
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
    cwd: Option<String>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
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
            println!("{}", Colors::bold("Screen:"));
            if let Some(screen) = result.get("screen").and_then(|v| v.as_str()) {
                println!("{}", screen);
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
    use crate::presenter::WaitResult;

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

    match ctx.format {
        OutputFormat::Json => ctx.presenter().present_value(&result),
        OutputFormat::Text => {
            let wait_result = WaitResult::from_json(&result);
            ctx.presenter().present_wait_result(&wait_result);
        }
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

pub fn handle_health<C: DaemonClient>(ctx: &mut HandlerContext<C>, verbose: bool) -> HandlerResult {
    use crate::presenter::HealthResult;

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
    let cli_version = env!("CARGO_PKG_VERSION");

    let (daemon_version, daemon_error) = match ctx.client.call("health", None) {
        Ok(result) => (
            result
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            None,
        ),
        Err(e) => ("unavailable".to_string(), Some(e.to_string())),
    };

    match ctx.format {
        OutputFormat::Json => {
            let mut output = json!({
                "cli_version": cli_version,
                "daemon_version": daemon_version,
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
            if let Some(err) = &daemon_error {
                println!(
                    "  Daemon version: {} ({})",
                    Colors::dim(&daemon_version),
                    Colors::error(err)
                );
            } else {
                println!("  Daemon version: {}", daemon_version);
            }
        }
    }
    Ok(())
}

pub fn handle_cleanup<C: DaemonClient>(ctx: &mut HandlerContext<C>, all: bool) -> HandlerResult {
    use crate::presenter::{CleanupFailure, CleanupResult};

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

    match ctx.format {
        OutputFormat::Json => {
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
            ctx.presenter().present_value(&output);
        }
        OutputFormat::Text => ctx.presenter().present_cleanup(&result),
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
            let find_result = crate::presenter::FindResult::from_json(&result);
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
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text => {
            if result.bool_or("success", false) {
                println!(
                    "Selected: {}",
                    result.str_array_join("selected_options", ", ")
                );
            } else {
                eprintln!(
                    "Multiselect failed: {}",
                    result.str_or("message", "Unknown error")
                );
                std::process::exit(1);
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
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text => {
            if result.bool_or("success", false) {
                println!(
                    "Scrolled to {} ({} scrolls)",
                    element_ref,
                    result.u64_or("scrolls_needed", 0)
                );
            } else {
                eprintln!("{}", result.str_or("message", "Element not found"));
                std::process::exit(1);
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
            println!("{}", serde_json::to_string_pretty(&result)?);
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
                eprintln!(
                    "Toggle failed: {}",
                    result.str_or("message", "Unknown error")
                );
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_attach<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    session_id: String,
    interactive: bool,
) -> HandlerResult {
    use crate::attach;

    let params = json!({ "session": session_id });
    let result = ctx.client.call("attach", Some(params))?;

    if interactive {
        if !result.bool_or("success", false) {
            return Err(format!("Failed to attach to session: {}", session_id).into());
        }

        attach::attach_ipc(ctx.client, &session_id)?;
    } else {
        match ctx.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            OutputFormat::Text => {
                if result.bool_or("success", false) {
                    println!("Attached to session {}", Colors::session_id(&session_id));
                } else {
                    return Err(format!("Failed to attach to session: {}", session_id).into());
                }
            }
        }
    }
    Ok(())
}

pub fn handle_record_start<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = ctx
        .client
        .call("record_start", Some(ctx.session_params()))?;

    ctx.output_json_or(&result, || {
        println!(
            "{} Recording started for session {}",
            Colors::success("●"),
            Colors::session_id(result.str_or("session_id", "?"))
        );
    })
}

pub fn handle_record_stop<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    output: Option<String>,
    record_format: RecordFormat,
) -> HandlerResult {
    let format_str = record_format.as_str();
    let params = ctx.params_with(json!({ "format": format_str }));
    let result = ctx.client.call("record_stop", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text => {
            println!(
                "{} Recording stopped ({} frames captured)",
                Colors::success("■"),
                result.u64_or("frame_count", 0)
            );

            if let Some(output_path) = output {
                if let Some(data) = result.get("data") {
                    let content = if format_str == "asciicast" {
                        data.str_or("data", "").to_string()
                    } else {
                        serde_json::to_string_pretty(data).unwrap_or_default()
                    };
                    std::fs::write(&output_path, content)?;
                    println!("Saved to: {}", output_path);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_record_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let result = ctx
        .client
        .call("record_status", Some(ctx.session_params()))?;

    ctx.output_json_or(&result, || {
        if result.bool_or("recording", false) {
            let duration_secs = result.u64_or("duration_ms", 0) / 1000;
            println!("{} Recording in progress", Colors::success("●"));
            println!("  Frames: {}", result.u64_or("frame_count", 0));
            println!("  Duration: {}s", duration_secs);
        } else {
            println!("{}", Colors::dim("Not recording"));
        }
    })
}

pub fn handle_trace<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    count: usize,
    start: bool,
    stop: bool,
) -> HandlerResult {
    let rpc_params = params::TraceParams {
        session: ctx.session.clone(),
        start,
        stop,
        count,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("trace", Some(params))?;

    ctx.output_json_or(&result, || {
        if start {
            println!("{} Tracing started", Colors::success("●"));
        } else if stop {
            println!("{} Tracing stopped", Colors::dim("■"));
        } else {
            let status = if result.bool_or("tracing", false) {
                Colors::success("(active)")
            } else {
                Colors::dim("(inactive)")
            };
            println!("{} Trace {}", Colors::bold("Trace:"), status);

            if let Some(entries) = result.get("entries").and_then(|v| v.as_array()) {
                if entries.is_empty() {
                    println!("{}", Colors::dim("  No trace entries"));
                } else {
                    for entry in entries {
                        let details = entry.get("details").and_then(|v| v.as_str());
                        let details_str = details.map(|d| format!(" {}", d)).unwrap_or_default();
                        println!(
                            "  [{}ms] {}{}",
                            entry.u64_or("timestamp_ms", 0),
                            entry.str_or("action", "?"),
                            Colors::dim(&details_str)
                        );
                    }
                }
            }
        }
    })
}

pub fn handle_console<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    lines: usize,
    clear: bool,
) -> HandlerResult {
    let rpc_params = params::ConsoleParams {
        session: ctx.session.clone(),
        count: lines,
        clear,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("console", Some(params))?;

    ctx.output_json_or(&result, || {
        if clear {
            println!("{}", Colors::success("Console cleared"));
        }
        if let Some(output_lines) = result.get("lines").and_then(|v| v.as_array()) {
            for line in output_lines {
                if let Some(s) = line.as_str() {
                    println!("{}", s);
                }
            }
        }
    })
}

pub fn handle_errors<C: DaemonClient>(
    ctx: &mut HandlerContext<C>,
    count: usize,
    clear: bool,
) -> HandlerResult {
    let rpc_params = params::ErrorsParams {
        session: ctx.session.clone(),
        count,
        clear,
    };
    let params = serde_json::to_value(rpc_params)?;

    let result = ctx.client.call("errors", Some(params))?;

    ctx.output_json_or(&result, || {
        if clear {
            println!("{}", Colors::success("Errors cleared"));
        }
        if let Some(errors) = result.get("errors").and_then(|v| v.as_array()) {
            if errors.is_empty() {
                println!("{}", Colors::dim("No errors captured"));
            } else {
                println!(
                    "{} {} error(s) (showing last {}):",
                    Colors::bold("Errors:"),
                    result.u64_or("total_count", 0),
                    errors.len()
                );
                for err in errors {
                    println!(
                        "  {} [{}] {}",
                        Colors::dim(err.str_or("timestamp", "?")),
                        Colors::error(err.str_or("source", "?")),
                        err.str_or("message", "?")
                    );
                }
            }
        }
    })
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
        eprintln!("Invalid condition format. Use: text:pattern, element:ref, or session:id");
        std::process::exit(1);
    }

    let (cond_type, cond_value) = (parts[0], parts[1]);

    let passed = match cond_type {
        "text" => {
            let params = json!({
                "session": ctx.session,
                "strip_ansi": true
            });
            let result = ctx.client.call("snapshot", Some(params))?;
            result.str_or("screen", "").contains(cond_value)
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
            eprintln!(
                "Unknown condition type: {}. Use: text, element, or session",
                cond_type
            );
            std::process::exit(1);
        }
    };

    let assert_result = crate::presenter::AssertResult {
        passed,
        condition: condition.clone(),
    };

    match ctx.format {
        OutputFormat::Json => {
            let output = json!({
                "condition": condition,
                "passed": passed
            });
            ctx.presenter().present_value(&output);
        }
        OutputFormat::Text => ctx.presenter().present_assert_result(&assert_result),
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
    use crate::ipc::{PidLookupResult, UnixSocketClient, daemon_lifecycle, get_daemon_pid};

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

pub fn handle_daemon_status<C: DaemonClient>(ctx: &mut HandlerContext<C>) -> HandlerResult {
    let cli_version = env!("CARGO_PKG_VERSION");

    match ctx.client.call("health", None) {
        Ok(result) => {
            let daemon_version = result.str_or("version", "unknown");
            let status = result.str_or("status", "unknown");
            let pid = result.u64_or("pid", 0);
            let uptime_ms = result.u64_or("uptime_ms", 0);
            let session_count = result.u64_or("session_count", 0);

            let version_mismatch = cli_version != daemon_version;

            match ctx.format {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "running": true,
                            "status": status,
                            "pid": pid,
                            "uptime_ms": uptime_ms,
                            "session_count": session_count,
                            "daemon_version": daemon_version,
                            "cli_version": cli_version,
                            "version_mismatch": version_mismatch
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
                    println!("  CLI version: {}", cli_version);

                    if version_mismatch {
                        eprintln!();
                        eprintln!("{} Version mismatch detected!", Colors::warning("⚠"));
                        eprintln!(
                            "  Run '{}' to update the daemon.",
                            Colors::info("agent-tui daemon restart")
                        );
                    }
                }
            }
        }
        Err(e) => match ctx.format {
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "running": false,
                        "cli_version": cli_version,
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
            }
        },
    }
    Ok(())
}

pub fn handle_daemon_restart<C: DaemonClient>(ctx: &HandlerContext<C>) -> HandlerResult {
    use crate::ipc::{PidLookupResult, daemon_lifecycle, get_daemon_pid, start_daemon_background};

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
    use crate::presenter::{Presenter, TextPresenter};
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

        fn present_client_error(&self, error: &crate::ipc::ClientError) {
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

        fn present_wait_result(&self, result: &crate::presenter::WaitResult) {
            self.output.borrow_mut().push(format!(
                "wait: found={}, elapsed={}ms",
                result.found, result.elapsed_ms
            ));
        }

        fn present_assert_result(&self, result: &crate::presenter::AssertResult) {
            self.output.borrow_mut().push(format!(
                "assert: passed={}, condition={}",
                result.passed, result.condition
            ));
        }

        fn present_health(&self, health: &crate::presenter::HealthResult) {
            self.output.borrow_mut().push(format!(
                "health: status={}, pid={}, sessions={}",
                health.status, health.pid, health.session_count
            ));
        }

        fn present_cleanup(&self, result: &crate::presenter::CleanupResult) {
            self.output.borrow_mut().push(format!(
                "cleanup: cleaned={}, failed={}",
                result.cleaned,
                result.failures.len()
            ));
        }

        fn present_find(&self, result: &crate::presenter::FindResult) {
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
