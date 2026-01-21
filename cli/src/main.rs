//! agent-tui - Pure Rust CLI for AI agents to interact with TUI applications
//!
//! This CLI communicates with a daemon process that manages PTY sessions.
//! The daemon is automatically started if not running.

#![allow(dead_code)]

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

    // Initialize color support
    color::init(cli.no_color);

    // Handle daemon command specially - it runs the server
    if matches!(cli.command, Commands::Daemon) {
        return start_daemon().map_err(|e| e.into());
    }

    // Handle demo-run command specially - runs the TUI directly (no daemon needed)
    if matches!(cli.command, Commands::DemoRun) {
        return demo::run_demo();
    }

    // Handle completions command - doesn't need daemon connection
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "agent-tui", &mut std::io::stdout());
        return Ok(());
    }

    // For all other commands, connect to daemon (auto-starting if needed)
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

    // Create handler context
    let mut ctx = HandlerContext::new(&mut client, cli.session, cli.format);

    match cli.command {
        Commands::Daemon => unreachable!(),             // Handled above
        Commands::DemoRun => unreachable!(),            // Handled above
        Commands::Completions { .. } => unreachable!(), // Handled above

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
        Commands::Keystroke { key } => handlers::handle_keystroke(&mut ctx, key)?,
        Commands::Type { text } => handlers::handle_type(&mut ctx, text)?,
        Commands::KeyDown { key } => handlers::handle_keydown(&mut ctx, key)?,
        Commands::KeyUp { key } => handlers::handle_keyup(&mut ctx, key)?,
        Commands::Wait {
            text,
            timeout,
            condition,
            target,
            element,
            visible,
            focused,
            not_visible,
            text_gone,
            stable,
            value,
        } => handlers::handle_wait(
            &mut ctx,
            text,
            timeout,
            condition,
            target,
            element,
            visible,
            focused,
            not_visible,
            text_gone,
            stable,
            value,
        )?,
        Commands::Kill => handlers::handle_kill(&mut ctx)?,
        Commands::Restart => handlers::handle_restart(&mut ctx)?,
        Commands::Sessions => handlers::handle_sessions(&mut ctx)?,
        Commands::Health { verbose } => handlers::handle_health(&mut ctx, verbose)?,
        Commands::Screen {
            strip_ansi,
            include_cursor,
        } => handlers::handle_screen(&mut ctx, strip_ansi, include_cursor)?,
        Commands::Resize { cols, rows } => handlers::handle_resize(&mut ctx, cols, rows)?,
        Commands::Version => handlers::handle_version(&mut ctx)?,
        Commands::Cleanup { all } => handlers::handle_cleanup(&mut ctx, all)?,
        Commands::Find {
            role,
            name,
            text,
            placeholder,
            focused,
            nth,
            exact,
        } => handlers::handle_find(&mut ctx, role, name, text, placeholder, focused, nth, exact)?,
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
        Commands::GetText { element_ref } => handlers::handle_get_text(&mut ctx, element_ref)?,
        Commands::GetValue { element_ref } => handlers::handle_get_value(&mut ctx, element_ref)?,
        Commands::GetFocused => handlers::handle_get_focused(&mut ctx)?,
        Commands::GetTitle => handlers::handle_get_title(&mut ctx)?,
        Commands::IsVisible { element_ref } => handlers::handle_is_visible(&mut ctx, element_ref)?,
        Commands::IsFocused { element_ref } => handlers::handle_is_focused(&mut ctx, element_ref)?,
        Commands::IsEnabled { element_ref } => handlers::handle_is_enabled(&mut ctx, element_ref)?,
        Commands::IsChecked { element_ref } => handlers::handle_is_checked(&mut ctx, element_ref)?,
        Commands::Count { role, name, text } => handlers::handle_count(&mut ctx, role, name, text)?,
        Commands::Toggle { element_ref, state } => {
            handlers::handle_toggle(&mut ctx, element_ref, state)?
        }
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
