use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;

use agent_tui::attach::AttachError;
use agent_tui::commands::Cli;
use agent_tui::commands::Commands;
use agent_tui::commands::DebugCommand;
use agent_tui::commands::RecordAction;
use agent_tui::handlers;
use agent_tui::handlers::HandlerContext;
use agent_tui_common::Colors;
use agent_tui_common::color_init;
use agent_tui_daemon::DaemonError;
use agent_tui_daemon::start_daemon;
use agent_tui_ipc::ClientError;
use agent_tui_ipc::ensure_daemon;

fn main() {
    if let Err(e) = run() {
        if let Some(client_error) = e.downcast_ref::<ClientError>() {
            eprintln!("{} {}", Colors::error("Error:"), client_error);
            if let Some(suggestion) = client_error.suggestion() {
                eprintln!("{} {}", Colors::dim("Suggestion:"), suggestion);
            }
            if client_error.is_retryable() {
                eprintln!(
                    "{}",
                    Colors::dim("(This error may be transient - retry may succeed)")
                );
            }
            std::process::exit(exit_code_for_client_error(client_error));
        } else if let Some(attach_error) = e.downcast_ref::<AttachError>() {
            eprintln!("{} {}", Colors::error("Error:"), attach_error);
            eprintln!(
                "{} {}",
                Colors::dim("Suggestion:"),
                attach_error.suggestion()
            );
            if attach_error.is_retryable() {
                eprintln!(
                    "{}",
                    Colors::dim("(This error may be transient - retry may succeed)")
                );
            }
            std::process::exit(attach_error.exit_code());
        } else if let Some(daemon_error) = e.downcast_ref::<DaemonError>() {
            eprintln!("{} {}", Colors::error("Error:"), daemon_error);
            eprintln!(
                "{} {}",
                Colors::dim("Suggestion:"),
                daemon_error.suggestion()
            );
            if daemon_error.is_retryable() {
                eprintln!(
                    "{}",
                    Colors::dim("(This error may be transient - retry may succeed)")
                );
            }
            std::process::exit(74); // EX_IOERR for External category
        } else {
            eprintln!("{} {}", Colors::error("Error:"), e);
            std::process::exit(1);
        }
    }
}

fn exit_code_for_client_error(error: &ClientError) -> i32 {
    use agent_tui_ipc::error_codes::ErrorCategory;

    match error.category() {
        Some(ErrorCategory::InvalidInput) => 64, // EX_USAGE
        Some(ErrorCategory::NotFound) => 69,     // EX_UNAVAILABLE
        Some(ErrorCategory::Busy) => 73,         // EX_CANTCREAT
        Some(ErrorCategory::External) => 74,     // EX_IOERR
        Some(ErrorCategory::Internal) => 74,     // EX_IOERR
        Some(ErrorCategory::Timeout) => 75,      // EX_TEMPFAIL
        None => 1,
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    color_init(cli.no_color);

    if matches!(cli.command, Commands::Daemon) {
        return start_daemon().map_err(Into::into);
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

    let format = cli.effective_format();
    let mut ctx = HandlerContext::new(&mut client, cli.session, format);

    match cli.command {
        Commands::Daemon | Commands::Completions { .. } => unreachable!(),

        Commands::Run {
            command,
            args,
            cwd,
            cols,
            rows,
        } => handlers::handle_spawn(&mut ctx, command, args, cwd, cols, rows)?,

        Commands::Snap {
            elements,
            accessibility,
            interactive_only,
            region,
            strip_ansi,
            include_cursor,
        } => {
            if accessibility {
                handlers::handle_accessibility_snapshot(&mut ctx, interactive_only)?
            } else {
                handlers::handle_snapshot(&mut ctx, elements, region, strip_ansi, include_cursor)?
            }
        }

        Commands::Click {
            element_ref,
            double,
        } => {
            if double {
                handlers::handle_dbl_click(&mut ctx, element_ref)?
            } else {
                handlers::handle_click(&mut ctx, element_ref)?
            }
        }
        Commands::Fill { element_ref, value } => {
            handlers::handle_fill(&mut ctx, element_ref, value)?
        }

        Commands::Key {
            key,
            text,
            hold,
            release,
        } => {
            if let Some(text) = text {
                handlers::handle_type(&mut ctx, text)?
            } else if let Some(key) = key {
                if hold {
                    handlers::handle_keydown(&mut ctx, key)?
                } else if release {
                    handlers::handle_keyup(&mut ctx, key)?
                } else {
                    handlers::handle_press(&mut ctx, key)?
                }
            }
        }

        Commands::Wait { params } => handlers::handle_wait(&mut ctx, params)?,
        Commands::Kill => handlers::handle_kill(&mut ctx)?,
        Commands::Restart => handlers::handle_restart(&mut ctx)?,
        Commands::Ls => handlers::handle_sessions(&mut ctx)?,
        Commands::Status { verbose } => handlers::handle_health(&mut ctx, verbose)?,

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
            to_ref,
        } => {
            if let Some(element_ref) = to_ref {
                handlers::handle_scroll_into_view(&mut ctx, element_ref)?
            } else if let Some(dir) = direction {
                handlers::handle_scroll(&mut ctx, dir, amount)?
            }
        }

        Commands::Focus { element_ref } => handlers::handle_focus(&mut ctx, element_ref)?,
        Commands::Clear { element_ref } => handlers::handle_clear(&mut ctx, element_ref)?,
        Commands::SelectAll { element_ref } => handlers::handle_select_all(&mut ctx, element_ref)?,

        Commands::Count { role, name, text } => handlers::handle_count(&mut ctx, role, name, text)?,

        Commands::Toggle {
            element_ref,
            on,
            off,
        } => {
            let state = if on {
                Some(true)
            } else if off {
                Some(false)
            } else {
                None
            };
            handlers::handle_toggle(&mut ctx, element_ref, state)?
        }

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

        Commands::Resize { cols, rows } => handlers::handle_resize(&mut ctx, cols, rows)?,
        Commands::Attach {
            session_id,
            interactive,
        } => handlers::handle_attach(&mut ctx, session_id, interactive)?,

        Commands::Version => handlers::handle_version(&mut ctx)?,
        Commands::Env => handlers::handle_env(&ctx)?,
        Commands::Assert { condition } => handlers::handle_assert(&mut ctx, condition)?,
        Commands::Cleanup { all } => handlers::handle_cleanup(&mut ctx, all)?,
        Commands::Find { params } => handlers::handle_find(&mut ctx, params)?,

        Commands::Debug(debug_cmd) => match debug_cmd {
            DebugCommand::Record(action) => match action {
                RecordAction::Start => handlers::handle_record_start(&mut ctx)?,
                RecordAction::Stop { output, format } => {
                    handlers::handle_record_stop(&mut ctx, output, format)?
                }
                RecordAction::Status => handlers::handle_record_status(&mut ctx)?,
            },
            DebugCommand::Trace { count, start, stop } => {
                handlers::handle_trace(&mut ctx, count, start, stop)?
            }
            DebugCommand::Console { lines, clear } => {
                handlers::handle_console(&mut ctx, lines, clear)?
            }
            DebugCommand::Errors { count, clear } => {
                handlers::handle_errors(&mut ctx, count, clear)?
            }
            DebugCommand::Env => handlers::handle_env(&ctx)?,
        },
    }

    Ok(())
}
