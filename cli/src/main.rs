//! agent-tui - Pure Rust CLI for AI agents to interact with TUI applications
//!
//! This CLI communicates with a daemon process that manages PTY sessions.
//! The daemon is automatically started if not running.

mod attach;
mod client;
mod color;
mod commands;
mod daemon;
mod demo;
mod detection;
mod handlers;
mod pty;
mod session;
mod sync_utils;
mod terminal;
mod wait;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use client::ensure_daemon;
use color::Colors;
use commands::{Cli, Commands};
use daemon::start_daemon;
use handlers::HandlerContext;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", Colors::error("Error:"), e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    color::init(cli.no_color);

    if matches!(cli.command, Commands::Daemon) {
        return start_daemon().map_err(|e| e.into());
    }

    if matches!(cli.command, Commands::DemoRun) {
        return demo::run_demo();
    }

    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "agent-tui", &mut std::io::stdout());
        return Ok(());
    }

    let mut client = match ensure_daemon() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{} Failed to connect to daemon: {}",
                Colors::error("Error:"),
                e
            );
            eprintln!();
            eprintln!("Troubleshooting:");
            eprintln!("  1. Check if socket directory is writable (usually /tmp)");
            eprintln!("  2. Try starting daemon manually: agent-tui daemon");
            eprintln!("  3. Check current configuration: agent-tui env");
            std::process::exit(1);
        }
    };

    let format = if cli.json {
        commands::OutputFormat::Json
    } else {
        cli.format
    };

    let mut ctx = HandlerContext::new(&mut client, cli.session, format);

    match cli.command {
        Commands::Daemon => unreachable!(),
        Commands::DemoRun => unreachable!(),
        Commands::Completions { .. } => unreachable!(),

        Commands::Demo => handlers::handle_demo(&mut ctx)?,
        Commands::Spawn {
            command,
            args,
            cwd,
            cols,
            rows,
        } => handlers::handle_spawn(&mut ctx, command, args, cwd, cols, rows)?,
        Commands::Snapshot {
            elements,
            interactive_only,
            compact,
            region,
        } => handlers::handle_snapshot(&mut ctx, elements, interactive_only, compact, region)?,
        Commands::Click { element_ref } => handlers::handle_click(&mut ctx, element_ref)?,
        Commands::DblClick { element_ref } => handlers::handle_dbl_click(&mut ctx, element_ref)?,
        Commands::Fill { element_ref, value } => {
            handlers::handle_fill(&mut ctx, element_ref, value)?
        }
        Commands::Press { key } => handlers::handle_press(&mut ctx, key)?,
        Commands::Type { text } => handlers::handle_type(&mut ctx, text)?,
        Commands::KeyDown { key } => handlers::handle_keydown(&mut ctx, key)?,
        Commands::KeyUp { key } => handlers::handle_keyup(&mut ctx, key)?,
        Commands::Wait { params } => handlers::handle_wait(&mut ctx, params)?,
        Commands::Kill => handlers::handle_kill(&mut ctx)?,
        Commands::Restart => handlers::handle_restart(&mut ctx)?,
        Commands::Sessions => handlers::handle_sessions(&mut ctx)?,
        Commands::Health { verbose } => handlers::handle_health(&mut ctx, verbose)?,
        Commands::Screenshot {
            strip_ansi,
            include_cursor,
        } => handlers::handle_screenshot(&mut ctx, strip_ansi, include_cursor)?,
        Commands::Resize { cols, rows } => handlers::handle_resize(&mut ctx, cols, rows)?,
        Commands::Version => handlers::handle_version(&mut ctx)?,
        Commands::Cleanup { all } => handlers::handle_cleanup(&mut ctx, all)?,
        Commands::Find { params } => handlers::handle_find(&mut ctx, params)?,
        Commands::Select {
            element_ref,
            option,
        } => handlers::handle_select(&mut ctx, element_ref, option)?,
        Commands::MultiSelect {
            element_ref,
            options,
        } => handlers::handle_multiselect(&mut ctx, element_ref, options)?,
        Commands::Scroll {
            direction,
            amount,
            element: _,
        } => handlers::handle_scroll(&mut ctx, direction, amount)?,
        Commands::ScrollIntoView { element_ref } => {
            handlers::handle_scroll_into_view(&mut ctx, element_ref)?
        }
        Commands::Focus { element_ref } => handlers::handle_focus(&mut ctx, element_ref)?,
        Commands::Clear { element_ref } => handlers::handle_clear(&mut ctx, element_ref)?,
        Commands::SelectAll { element_ref } => handlers::handle_select_all(&mut ctx, element_ref)?,
        Commands::Get(command) => match command {
            commands::GetCommand::Text { element_ref } => {
                handlers::handle_get_text(&mut ctx, element_ref)?
            }
            commands::GetCommand::Value { element_ref } => {
                handlers::handle_get_value(&mut ctx, element_ref)?
            }
            commands::GetCommand::Focused => handlers::handle_get_focused(&mut ctx)?,
            commands::GetCommand::Title => handlers::handle_get_title(&mut ctx)?,
        },
        Commands::Is(command) => match command {
            commands::IsCommand::Visible { element_ref } => {
                handlers::handle_is_visible(&mut ctx, element_ref)?
            }
            commands::IsCommand::Focused { element_ref } => {
                handlers::handle_is_focused(&mut ctx, element_ref)?
            }
            commands::IsCommand::Enabled { element_ref } => {
                handlers::handle_is_enabled(&mut ctx, element_ref)?
            }
            commands::IsCommand::Checked { element_ref } => {
                handlers::handle_is_checked(&mut ctx, element_ref)?
            }
        },
        Commands::Count { role, name, text } => handlers::handle_count(&mut ctx, role, name, text)?,
        Commands::Toggle { element_ref, state } => {
            handlers::handle_toggle(&mut ctx, element_ref, state)?
        }
        Commands::Check { element_ref } => handlers::handle_check(&mut ctx, element_ref)?,
        Commands::Uncheck { element_ref } => handlers::handle_uncheck(&mut ctx, element_ref)?,
        Commands::Attach {
            session_id,
            interactive,
        } => handlers::handle_attach(&mut ctx, session_id, interactive)?,
        Commands::RecordStart => handlers::handle_record_start(&mut ctx)?,
        Commands::RecordStop {
            output,
            record_format,
        } => handlers::handle_record_stop(&mut ctx, output, record_format)?,
        Commands::RecordStatus => handlers::handle_record_status(&mut ctx)?,
        Commands::Trace { count, start, stop } => {
            handlers::handle_trace(&mut ctx, count, start, stop)?
        }
        Commands::Console { lines, clear } => handlers::handle_console(&mut ctx, lines, clear)?,
        Commands::Errors { count, clear } => handlers::handle_errors(&mut ctx, count, clear)?,
        Commands::Env => handlers::handle_env(&ctx)?,
        Commands::Assert { condition } => handlers::handle_assert(&mut ctx, condition)?,
    }

    Ok(())
}
