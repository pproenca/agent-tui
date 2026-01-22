use serde_json::json;

use agent_tui_common::Colors;
use agent_tui_ipc::socket_path;
use agent_tui_ipc::DaemonClient;

use crate::commands::OutputFormat;

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
}

pub fn handle_health(
    ctx: &mut HandlerContext,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = if verbose {
        Some(json!({"verbose": true}))
    } else {
        None
    };

    let result = ctx.client.call("health", params)?;

    if ctx.format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let status = result
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let sessions = result.get("sessions").and_then(|v| v.as_u64()).unwrap_or(0);
        let active = result
            .get("active_session")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        println!(
            "{} Daemon is {}",
            Colors::success("✓"),
            Colors::success(status)
        );
        println!("  Sessions: {}", sessions);
        if let Some(active_id) = active {
            println!("  Active: {}", Colors::session_id(&active_id));
        }

        if verbose {
            if let Some(version) = result.get("version").and_then(|v| v.as_str()) {
                println!("  Version: {}", version);
            }
            if let Some(uptime) = result.get("uptime_secs").and_then(|v| v.as_u64()) {
                println!("  Uptime: {}s", uptime);
            }
        }
    }

    Ok(())
}

pub fn handle_sessions(ctx: &mut HandlerContext) -> Result<(), Box<dyn std::error::Error>> {
    let result = ctx.client.call("sessions", None)?;

    if ctx.format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let sessions = result
            .get("sessions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if sessions.is_empty() {
            println!("No active sessions");
            println!();
            println!("Start one with: agent-tui spawn <command>");
        } else {
            for session in &sessions {
                let id = session.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let command = session.get("command").and_then(|v| v.as_str()).unwrap_or("?");
                let running = session
                    .get("running")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let status = if running {
                    Colors::success("running")
                } else {
                    Colors::error("stopped")
                };

                println!("{}: {} [{}]", Colors::session_id(id), command, status);
            }
        }
    }

    Ok(())
}

pub fn handle_version(ctx: &mut HandlerContext) -> Result<(), Box<dyn std::error::Error>> {
    let result = ctx.client.call("version", None)?;

    if ctx.format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let version = result
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        println!("agent-tui {}", version);
    }

    Ok(())
}

pub fn handle_env(ctx: &HandlerContext) -> Result<(), Box<dyn std::error::Error>> {
    let socket = socket_path();
    let transport = std::env::var("AGENT_TUI_TRANSPORT").unwrap_or_else(|_| "unix".to_string());

    if ctx.format == OutputFormat::Json {
        let result = json!({
            "socket_path": socket.display().to_string(),
            "transport": transport,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Configuration:");
        println!("  Socket: {}", socket.display());
        println!("  Transport: {}", transport);
        println!();
        println!("Environment variables:");
        println!("  AGENT_TUI_SOCKET - Override socket path");
        println!("  AGENT_TUI_TRANSPORT - unix or tcp (default: unix)");
        println!("  AGENT_TUI_TCP_PORT - TCP port (default: 19847)");
    }

    Ok(())
}
