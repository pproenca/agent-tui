//! Command handlers for the agent-tui CLI
//!
//! This module contains the handler functions for each CLI command,
//! extracted from main.rs for better maintainability.

use crate::client::DaemonClient;
use crate::color::Colors;
use crate::commands::{OutputFormat, RecordFormat, ScrollDirection};
use crate::daemon;
use serde_json::json;
use std::collections::HashMap;

/// Result type for command handlers
pub type HandlerResult = Result<(), Box<dyn std::error::Error>>;

/// Context passed to all handlers
pub struct HandlerContext<'a> {
    pub client: &'a mut DaemonClient,
    pub session: Option<String>,
    pub format: OutputFormat,
}

impl<'a> HandlerContext<'a> {
    pub fn new(
        client: &'a mut DaemonClient,
        session: Option<String>,
        format: OutputFormat,
    ) -> Self {
        Self {
            client,
            session,
            format,
        }
    }

    /// Handle a success/failure result with standard output formatting
    ///
    /// Returns true if success, false otherwise. In text mode, prints appropriate message
    /// and exits with code 1 on failure.
    pub fn output_success_result(
        &self,
        result: &serde_json::Value,
        success_msg: &str,
        failure_prefix: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(result)?);
            }
            OutputFormat::Text | OutputFormat::Tree => {
                if success {
                    println!("{}", success_msg);
                } else {
                    let msg = result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    eprintln!("{}: {}", failure_prefix, msg);
                    std::process::exit(1);
                }
            }
        }
        Ok(success)
    }
}

pub fn handle_demo(ctx: &mut HandlerContext) -> HandlerResult {
    let exe_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "agent-tui".to_string());

    let params = json!({
        "command": exe_path,
        "args": ["demo-run"],
        "session": ctx.session,
        "cols": 80,
        "rows": 24
    });

    let result = ctx.client.call("spawn", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!(
                "{} {}",
                Colors::success("Demo started:"),
                Colors::session_id(session_id)
            );
            println!();
            println!("Try these commands:");
            println!(
                "  {} - See detected elements",
                Colors::dim("agent-tui snapshot -i")
            );
            println!(
                "  {} - Fill the name input",
                Colors::dim("agent-tui fill @e1 \"Hello\"")
            );
            println!(
                "  {} - Toggle the checkbox",
                Colors::dim("agent-tui toggle @e2")
            );
            println!(
                "  {} - Click Submit button",
                Colors::dim("agent-tui click @e3")
            );
            println!("  {} - End session", Colors::dim("agent-tui kill"));
        }
    }
    Ok(())
}

pub fn handle_spawn(
    ctx: &mut HandlerContext,
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
) -> HandlerResult {
    let params = json!({
        "command": command,
        "args": args,
        "cwd": cwd,
        "session": ctx.session,
        "cols": cols,
        "rows": rows
    });

    let result = ctx.client.call("spawn", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let pid = result.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
            println!(
                "{} {}",
                Colors::success("Session started:"),
                Colors::session_id(session_id)
            );
            println!("  PID: {}", pid);
        }
    }
    Ok(())
}

pub fn handle_snapshot(
    ctx: &mut HandlerContext,
    elements: bool,
    interactive_only: bool,
    compact: bool,
    region: Option<String>,
) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "include_elements": elements,
        "interactive_only": interactive_only,
        "compact": compact,
        "region": region
    });

    let result = ctx.client.call("snapshot", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Tree => {
            if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
                for el in elements {
                    let ref_str = el.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                    let el_type = el.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let label = el.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let focused = el.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
                    let checked = el.get("checked").and_then(|v| v.as_bool());
                    let value = el.get("value").and_then(|v| v.as_str());

                    let mut attrs = vec![format!("ref={}", ref_str)];
                    if focused {
                        attrs.push("focused".to_string());
                    }
                    if let Some(true) = checked {
                        attrs.push("checked".to_string());
                    }
                    if let Some(v) = value {
                        if !v.is_empty() && el_type == "input" {
                            attrs.push(format!("value=\"{}\"", v));
                        }
                    }

                    let display_text = if !label.is_empty() {
                        label.to_string()
                    } else if let Some(v) = value {
                        v.to_string()
                    } else {
                        String::new()
                    };

                    println!("- {} \"{}\" [{}]", el_type, display_text, attrs.join("] ["));
                }
            }
            println!();
            if let Some(screen) = result.get("screen").and_then(|v| v.as_str()) {
                println!("{}", screen);
            }
        }
        OutputFormat::Text => {
            if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
                if !elements.is_empty() {
                    println!("{}", Colors::bold("Elements:"));
                    for el in elements {
                        let ref_str = el.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                        let el_type = el.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let label = el.get("label").and_then(|v| v.as_str()).unwrap_or("");
                        let focused = el.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
                        let selected = el
                            .get("selected")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let row = el
                            .get("position")
                            .and_then(|p| p.get("row"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let col = el
                            .get("position")
                            .and_then(|p| p.get("col"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        let focused_str = if focused {
                            Colors::success(" *focused*")
                        } else {
                            String::new()
                        };
                        let selected_str = if selected {
                            Colors::info(" *selected*")
                        } else {
                            String::new()
                        };
                        let value = el
                            .get("value")
                            .and_then(|v| v.as_str())
                            .map(|v| format!(" \"{}\"", v))
                            .unwrap_or_default();

                        println!(
                            "{} [{}{}]{} {}{}{}",
                            Colors::element_ref(ref_str),
                            el_type,
                            if label.is_empty() {
                                "".to_string()
                            } else {
                                format!(":{}", label)
                            },
                            value,
                            Colors::dim(&format!("({},{})", row, col)),
                            focused_str,
                            selected_str
                        );
                    }
                    println!();
                }
            }
            println!("{}", Colors::bold("Screen:"));
            if let Some(screen) = result.get("screen").and_then(|v| v.as_str()) {
                println!("{}", screen);
            }
        }
    }
    Ok(())
}

pub fn handle_click(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("click", Some(params))?;
    ctx.output_success_result(&result, "Clicked successfully", "Click failed")?;
    Ok(())
}

pub fn handle_dbl_click(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("dbl_click", Some(params))?;
    ctx.output_success_result(
        &result,
        "Double-clicked successfully",
        "Double-click failed",
    )?;
    Ok(())
}

pub fn handle_fill(ctx: &mut HandlerContext, element_ref: String, value: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "value": value,
        "session": ctx.session
    });

    let result = ctx.client.call("fill", Some(params))?;
    ctx.output_success_result(&result, "Filled successfully", "Fill failed")?;
    Ok(())
}

pub fn handle_press(ctx: &mut HandlerContext, key: String) -> HandlerResult {
    let params = json!({
        "key": key,
        "session": ctx.session
    });

    let result = ctx.client.call("keystroke", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                println!("Key pressed");
            } else {
                eprintln!("Press failed");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_type(ctx: &mut HandlerContext, text: String) -> HandlerResult {
    let params = json!({
        "text": text,
        "session": ctx.session
    });

    let result = ctx.client.call("type", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                println!("Text typed");
            } else {
                eprintln!("Type failed");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_keydown(ctx: &mut HandlerContext, key: String) -> HandlerResult {
    let params = json!({
        "key": key,
        "session": ctx.session
    });

    let result = ctx.client.call("keydown", Some(params))?;
    ctx.output_success_result(&result, &format!("Key held: {}", key), "Keydown failed")?;
    Ok(())
}

pub fn handle_keyup(ctx: &mut HandlerContext, key: String) -> HandlerResult {
    let params = json!({
        "key": key,
        "session": ctx.session
    });

    let result = ctx.client.call("keyup", Some(params))?;
    ctx.output_success_result(&result, &format!("Key released: {}", key), "Keyup failed")?;
    Ok(())
}

pub fn handle_wait(ctx: &mut HandlerContext, params: crate::commands::WaitParams) -> HandlerResult {
    let (cond, tgt) = if params.stable {
        (Some("stable".to_string()), None)
    } else if let Some(el) = params.element {
        (Some("element".to_string()), Some(el))
    } else if let Some(vis) = params.visible {
        (Some("element".to_string()), Some(vis))
    } else if let Some(f) = params.focused {
        (Some("focused".to_string()), Some(f))
    } else if let Some(nv) = params.not_visible {
        (Some("not_visible".to_string()), Some(nv))
    } else if let Some(tg) = params.text_gone {
        (Some("text_gone".to_string()), Some(tg))
    } else if let Some(v) = params.value {
        (Some("value".to_string()), Some(v))
    } else if let Some(c) = params.condition {
        (Some(c.to_string()), params.target)
    } else {
        (None, None)
    };

    let rpc_params = json!({
        "text": params.text,
        "timeout_ms": params.timeout,
        "session": ctx.session,
        "condition": cond,
        "target": tgt
    });

    let result = ctx.client.call("wait", Some(rpc_params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let elapsed_ms = result
        .get("elapsed_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if found {
                println!("Found after {}ms", elapsed_ms);
            } else {
                eprintln!("Timeout after {}ms - not found", elapsed_ms);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_kill(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("kill", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Session {} killed", session_id);
        }
    }
    Ok(())
}

pub fn handle_restart(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("restart", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let old_id = result
                .get("old_session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let new_id = result
                .get("new_session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let command = result
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Restarted '{}': {} -> {}", command, old_id, new_id);
        }
    }
    Ok(())
}

pub fn handle_sessions(ctx: &mut HandlerContext) -> HandlerResult {
    let result = ctx.client.call("sessions", None)?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let active_id = result.get("active_session").and_then(|v| v.as_str());

            match result.get("sessions").and_then(|v| v.as_array()) {
                Some(sessions) if !sessions.is_empty() => {
                    println!("{}", Colors::bold("Active sessions:"));
                    for session in sessions {
                        let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        let command = session
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let pid = session.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
                        let running = session
                            .get("running")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
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
        }
    }
    Ok(())
}

pub fn handle_health(ctx: &mut HandlerContext, verbose: bool) -> HandlerResult {
    let result = ctx.client.call("health", None)?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let status = result
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let pid = result.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
            let uptime_ms = result
                .get("uptime_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let session_count = result
                .get("session_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let version = result
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("?");

            let uptime_secs = uptime_ms / 1000;
            let uptime_mins = uptime_secs / 60;
            let uptime_hours = uptime_mins / 60;
            let uptime_display = if uptime_hours > 0 {
                format!(
                    "{}h {}m {}s",
                    uptime_hours,
                    uptime_mins % 60,
                    uptime_secs % 60
                )
            } else if uptime_mins > 0 {
                format!("{}m {}s", uptime_mins, uptime_secs % 60)
            } else {
                format!("{}s", uptime_secs)
            };

            println!(
                "{} {}",
                Colors::bold("Daemon status:"),
                Colors::success(status)
            );
            println!("  PID: {}", pid);
            println!("  Uptime: {}", uptime_display);
            println!("  Sessions: {}", session_count);
            println!("  Version: {}", Colors::dim(version));

            if verbose {
                let socket = daemon::socket_path();
                let pid_file = socket.with_extension("pid");
                println!();
                println!("{}", Colors::bold("Connection:"));
                println!("  Socket: {}", socket.display());
                println!("  PID file: {}", pid_file.display());
            }
        }
    }
    Ok(())
}

pub fn handle_screenshot(
    ctx: &mut HandlerContext,
    strip_ansi: bool,
    include_cursor: bool,
) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "strip_ansi": strip_ansi,
        "include_cursor": include_cursor
    });

    let result = ctx.client.call("screen", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if let Some(screen) = result.get("screen").and_then(|v| v.as_str()) {
                println!("{}", screen);
            }
            if include_cursor {
                if let Some(cursor) = result.get("cursor") {
                    let row = cursor.get("row").and_then(|v| v.as_u64()).unwrap_or(0);
                    let col = cursor.get("col").and_then(|v| v.as_u64()).unwrap_or(0);
                    let visible = cursor
                        .get("visible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let vis_str = if visible { "visible" } else { "hidden" };
                    println!("\nCursor: row={}, col={} ({})", row, col, vis_str);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_resize(ctx: &mut HandlerContext, cols: u16, rows: u16) -> HandlerResult {
    let params = json!({
        "cols": cols,
        "rows": rows,
        "session": ctx.session
    });

    let result = ctx.client.call("resize", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!(
                "Session {} resized to {}x{}",
                Colors::session_id(session_id),
                cols,
                rows
            );
        }
    }
    Ok(())
}

pub fn handle_version(ctx: &mut HandlerContext) -> HandlerResult {
    let cli_version = env!("CARGO_PKG_VERSION");

    let daemon_version = match ctx.client.call("health", None) {
        Ok(result) => result
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        Err(_) => "unavailable".to_string(),
    };

    match ctx.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "cli_version": cli_version,
                    "daemon_version": daemon_version,
                    "mode": "daemon"
                })
            );
        }
        OutputFormat::Text | OutputFormat::Tree => {
            println!("{}", Colors::bold("agent-tui"));
            println!("  CLI version: {}", cli_version);
            println!("  Daemon version: {}", daemon_version);
        }
    }
    Ok(())
}

pub fn handle_cleanup(ctx: &mut HandlerContext, all: bool) -> HandlerResult {
    let sessions_result = ctx.client.call("sessions", None)?;
    let sessions = sessions_result.get("sessions").and_then(|v| v.as_array());

    let mut cleaned = 0;
    if let Some(sessions) = sessions {
        for session in sessions {
            let id = session.get("id").and_then(|v| v.as_str());
            let running = session
                .get("running")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let should_cleanup = all || !running;
            if should_cleanup {
                if let Some(id) = id {
                    let params = json!({ "session": id });
                    if ctx.client.call("kill", Some(params)).is_ok() {
                        cleaned += 1;
                        if ctx.format == OutputFormat::Text {
                            println!("Cleaned up session: {}", id);
                        }
                    }
                }
            }
        }
    }

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::json!({ "sessions_cleaned": cleaned }));
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if cleaned > 0 {
                println!(
                    "{} Cleaned up {} session(s)",
                    Colors::success("Done:"),
                    cleaned
                );
            } else {
                println!("{}", Colors::dim("No sessions to clean up"));
            }
        }
    }
    Ok(())
}

pub fn handle_find(ctx: &mut HandlerContext, params: crate::commands::FindParams) -> HandlerResult {
    let rpc_params = json!({
        "session": ctx.session,
        "role": params.role,
        "name": params.name,
        "text": params.text,
        "placeholder": params.placeholder,
        "focused": params.focused,
        "nth": params.nth,
        "exact": params.exact
    });

    let result = ctx.client.call("find", Some(rpc_params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Tree => {
            if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
                for el in elements {
                    let ref_str = el.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                    let el_type = el.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let label = el.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    println!("- {} \"{}\" [ref={}]", el_type, label, ref_str);
                }
            }
        }
        OutputFormat::Text => {
            let count = result.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
            if count == 0 {
                println!("{}", Colors::dim("No elements found"));
            } else {
                println!("{} Found {} element(s):", Colors::success("✓"), count);
                if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
                    for el in elements {
                        let ref_str = el.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                        let el_type = el.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let label = el.get("label").and_then(|v| v.as_str()).unwrap_or("");
                        let el_focused =
                            el.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
                        let focused_str = if el_focused {
                            Colors::success(" *focused*")
                        } else {
                            String::new()
                        };
                        println!(
                            "  {} [{}:{}]{}",
                            Colors::element_ref(ref_str),
                            el_type,
                            label,
                            focused_str
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn handle_select(
    ctx: &mut HandlerContext,
    element_ref: String,
    option: String,
) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "option": option,
        "session": ctx.session
    });

    let result = ctx.client.call("select", Some(params))?;
    ctx.output_success_result(&result, &format!("Selected: {}", option), "Select failed")?;
    Ok(())
}

pub fn handle_multiselect(
    ctx: &mut HandlerContext,
    element_ref: String,
    options: Vec<String>,
) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "options": options,
        "session": ctx.session
    });

    let result = ctx.client.call("multiselect", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                let selected = result
                    .get("selected_options")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                println!("Selected: {}", selected);
            } else {
                let msg = result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                eprintln!("Multiselect failed: {}", msg);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_scroll(
    ctx: &mut HandlerContext,
    direction: ScrollDirection,
    amount: u16,
) -> HandlerResult {
    let dir_str = match direction {
        ScrollDirection::Up => "up",
        ScrollDirection::Down => "down",
        ScrollDirection::Left => "left",
        ScrollDirection::Right => "right",
    };

    let params = json!({
        "direction": dir_str,
        "amount": amount,
        "session": ctx.session
    });

    let result = ctx.client.call("scroll", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                println!("Scrolled {} {} times", dir_str, amount);
            } else {
                eprintln!("Scroll failed");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_scroll_into_view(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("scroll_into_view", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                let scrolls = result
                    .get("scrolls_needed")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                println!("Scrolled to {} ({} scrolls)", element_ref, scrolls);
            } else {
                let msg = result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Element not found");
                eprintln!("{}", msg);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_focus(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("focus", Some(params))?;
    ctx.output_success_result(
        &result,
        &format!("Focused: {}", element_ref),
        "Focus failed",
    )?;
    Ok(())
}

pub fn handle_clear(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("clear", Some(params))?;
    ctx.output_success_result(
        &result,
        &format!("Cleared: {}", element_ref),
        "Clear failed",
    )?;
    Ok(())
}

pub fn handle_select_all(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("select_all", Some(params))?;
    ctx.output_success_result(
        &result,
        &format!("Selected all in: {}", element_ref),
        "Select all failed",
    )?;
    Ok(())
}

pub fn handle_get_text(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("get_text", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if found {
                let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("");
                println!("{}", text);
            } else {
                eprintln!("Element not found: {}", element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_get_value(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("get_value", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if found {
                let value = result.get("value").and_then(|v| v.as_str()).unwrap_or("");
                println!("{}", value);
            } else {
                eprintln!("Element not found: {}", element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_get_focused(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("get_focused", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if found {
                let ref_str = result.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                let el_type = result.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let label = result.get("label").and_then(|v| v.as_str()).unwrap_or("");
                println!("- {} \"{}\" [ref={}] [focused]", el_type, label, ref_str);
            } else {
                eprintln!("No focused element found");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_get_title(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("get_title", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let title = result.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("Session: {} | Command: {}", session_id, title);
        }
    }
    Ok(())
}

pub fn handle_is_visible(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("is_visible", Some(params))?;
    let visible = result
        .get("visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if visible {
                println!("{} {} is visible", Colors::success("✓"), element_ref);
            } else {
                println!("{} {} is not visible", Colors::error("✗"), element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_is_focused(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("is_focused", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let focused = result
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if !found {
                eprintln!("Element not found: {}", element_ref);
                std::process::exit(1);
            } else if focused {
                println!("{} {} is focused", Colors::success("✓"), element_ref);
            } else {
                println!("{} {} is not focused", Colors::error("✗"), element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_is_enabled(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("is_enabled", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let enabled = result
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if !found {
                eprintln!("Element not found: {}", element_ref);
                std::process::exit(1);
            } else if enabled {
                println!("{} {} is enabled", Colors::success("✓"), element_ref);
            } else {
                println!("{} {} is disabled", Colors::error("✗"), element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_is_checked(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session
    });

    let result = ctx.client.call("is_checked", Some(params))?;
    let found = result
        .get("found")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let checked = result
        .get("checked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if !found {
                eprintln!("Element not found: {}", element_ref);
                std::process::exit(1);
            } else if checked {
                println!("{} {} is checked", Colors::success("✓"), element_ref);
            } else {
                println!("{} {} is not checked", Colors::error("✗"), element_ref);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_count(
    ctx: &mut HandlerContext,
    role: Option<String>,
    name: Option<String>,
    text: Option<String>,
) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "role": role,
        "name": name,
        "text": text
    });

    let result = ctx.client.call("count", Some(params))?;
    let count = result.get("count").and_then(|v| v.as_u64()).unwrap_or(0);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            println!("{}", count);
        }
    }
    Ok(())
}

pub fn handle_toggle(
    ctx: &mut HandlerContext,
    element_ref: String,
    state: Option<bool>,
) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session,
        "state": state
    });

    let result = ctx.client.call("toggle", Some(params))?;
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let checked = result
        .get("checked")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if success {
                let state = if checked { "checked" } else { "unchecked" };
                println!("{} is now {}", element_ref, state);
            } else {
                let msg = result
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                eprintln!("Toggle failed: {}", msg);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

pub fn handle_check(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session,
        "state": true
    });

    let result = ctx.client.call("toggle", Some(params))?;
    ctx.output_success_result(
        &result,
        &format!("{} is now checked", element_ref),
        "Check failed",
    )?;
    Ok(())
}

pub fn handle_uncheck(ctx: &mut HandlerContext, element_ref: String) -> HandlerResult {
    let params = json!({
        "ref": element_ref,
        "session": ctx.session,
        "state": false
    });

    let result = ctx.client.call("toggle", Some(params))?;
    ctx.output_success_result(
        &result,
        &format!("{} is now unchecked", element_ref),
        "Uncheck failed",
    )?;
    Ok(())
}

pub fn handle_attach(
    ctx: &mut HandlerContext,
    session_id: String,
    interactive: bool,
) -> HandlerResult {
    use crate::attach;

    if interactive {
        let params = json!({ "session": session_id });
        let result = ctx.client.call("attach", Some(params))?;
        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !success {
            eprintln!("Failed to attach to session: {}", session_id);
            std::process::exit(1);
        }

        if let Err(e) = attach::attach_ipc(ctx.client, &session_id) {
            eprintln!("Attach failed: {}", e);
            std::process::exit(1);
        }
    } else {
        let params = json!({
            "session": session_id
        });

        let result = ctx.client.call("attach", Some(params))?;
        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match ctx.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            OutputFormat::Text | OutputFormat::Tree => {
                if success {
                    println!("Attached to session {}", Colors::session_id(&session_id));
                } else {
                    eprintln!("Failed to attach to session: {}", session_id);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_record_start(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("record_start", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let session_id = result
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!(
                "{} Recording started for session {}",
                Colors::success("●"),
                Colors::session_id(session_id)
            );
        }
    }
    Ok(())
}

pub fn handle_record_stop(
    ctx: &mut HandlerContext,
    output: Option<String>,
    record_format: RecordFormat,
) -> HandlerResult {
    let format_str = match record_format {
        RecordFormat::Json => "json",
        RecordFormat::Asciicast => "asciicast",
    };

    let params = json!({
        "session": ctx.session,
        "format": format_str
    });

    let result = ctx.client.call("record_stop", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let frame_count = result
                .get("frame_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            println!(
                "{} Recording stopped ({} frames captured)",
                Colors::success("■"),
                frame_count
            );

            if let Some(output_path) = output {
                if let Some(data) = result.get("data") {
                    let content = if format_str == "asciicast" {
                        data.get("data")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
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

pub fn handle_record_status(ctx: &mut HandlerContext) -> HandlerResult {
    let params = json!({
        "session": ctx.session
    });

    let result = ctx.client.call("record_status", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            let recording = result
                .get("recording")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let frame_count = result
                .get("frame_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let duration_ms = result
                .get("duration_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if recording {
                let duration_secs = duration_ms / 1000;
                println!("{} Recording in progress", Colors::success("●"));
                println!("  Frames: {}", frame_count);
                println!("  Duration: {}s", duration_secs);
            } else {
                println!("{}", Colors::dim("Not recording"));
            }
        }
    }
    Ok(())
}

pub fn handle_trace(
    ctx: &mut HandlerContext,
    count: usize,
    start: bool,
    stop: bool,
) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "start": start,
        "stop": stop,
        "count": count
    });

    let result = ctx.client.call("trace", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if start {
                println!("{} Tracing started", Colors::success("●"));
            } else if stop {
                println!("{} Tracing stopped", Colors::dim("■"));
            } else {
                let tracing = result
                    .get("tracing")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let status = if tracing {
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
                            let ts = entry
                                .get("timestamp_ms")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            let action =
                                entry.get("action").and_then(|v| v.as_str()).unwrap_or("?");
                            let details = entry.get("details").and_then(|v| v.as_str());
                            let details_str =
                                details.map(|d| format!(" {}", d)).unwrap_or_default();
                            println!("  [{}ms] {}{}", ts, action, Colors::dim(&details_str));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn handle_console(ctx: &mut HandlerContext, lines: usize, clear: bool) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "count": lines,
        "clear": clear
    });

    let result = ctx.client.call("console", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
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
        }
    }
    Ok(())
}

pub fn handle_errors(ctx: &mut HandlerContext, count: usize, clear: bool) -> HandlerResult {
    let params = json!({
        "session": ctx.session,
        "count": count,
        "clear": clear
    });

    let result = ctx.client.call("errors", Some(params))?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if clear {
                println!("{}", Colors::success("Errors cleared"));
            }
            let total = result
                .get("total_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if let Some(errors) = result.get("errors").and_then(|v| v.as_array()) {
                if errors.is_empty() {
                    println!("{}", Colors::dim("No errors captured"));
                } else {
                    println!(
                        "{} {} error(s) (showing last {}):",
                        Colors::bold("Errors:"),
                        total,
                        errors.len()
                    );
                    for err in errors {
                        let timestamp =
                            err.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
                        let message = err.get("message").and_then(|v| v.as_str()).unwrap_or("?");
                        let source = err.get("source").and_then(|v| v.as_str()).unwrap_or("?");
                        println!(
                            "  {} [{}] {}",
                            Colors::dim(timestamp),
                            Colors::error(source),
                            message
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn handle_env(ctx: &HandlerContext) -> HandlerResult {
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
                    "socket_path": daemon::socket_path().display().to_string()
                }))?
            );
        }
        OutputFormat::Text | OutputFormat::Tree => {
            println!("{}", Colors::bold("Environment Configuration:"));
            let transport = vars
                .iter()
                .find(|(n, _)| *n == "AGENT_TUI_TRANSPORT")
                .and_then(|(_, v)| v.as_ref());
            println!(
                "  Transport: {}",
                transport.map(|v| v.as_str()).unwrap_or("unix")
            );
            println!("  Socket: {}", daemon::socket_path().display());
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

pub fn handle_assert(ctx: &mut HandlerContext, condition: String) -> HandlerResult {
    let parts: Vec<&str> = condition.splitn(2, ':').collect();
    if parts.len() != 2 {
        eprintln!("Invalid condition format. Use: text:pattern, element:ref, or session:id");
        std::process::exit(1);
    }

    let (cond_type, cond_value) = (parts[0], parts[1]);

    let passed = match cond_type {
        "text" => {
            let params = json!({ "session": ctx.session });
            let result = ctx.client.call("screen", Some(params))?;
            let screen = result.get("screen").and_then(|v| v.as_str()).unwrap_or("");
            screen.contains(cond_value)
        }
        "element" => {
            let params = json!({ "ref": cond_value, "session": ctx.session });
            let result = ctx.client.call("is_visible", Some(params))?;
            result
                .get("visible")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        }
        "session" => {
            let result = ctx.client.call("sessions", None)?;
            if let Some(sessions) = result.get("sessions").and_then(|v| v.as_array()) {
                sessions.iter().any(|s| {
                    let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let running = s.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                    id == cond_value && running
                })
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

    match ctx.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "condition": condition,
                    "passed": passed
                })
            );
        }
        OutputFormat::Text | OutputFormat::Tree => {
            if passed {
                println!("{} Assertion passed: {}", Colors::success("✓"), condition);
            } else {
                eprintln!("{} Assertion failed: {}", Colors::error("✗"), condition);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
