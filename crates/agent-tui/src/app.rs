//! Application entry point and command dispatch.

use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;

use agent_tui_common::{Colors, color_init};
use agent_tui_daemon::{DaemonError, start_daemon};
use agent_tui_ipc::{ClientError, DaemonClient, ensure_daemon};

use crate::attach::AttachError;
use crate::commands::{Cli, Commands, DaemonCommand, DebugCommand, RecordAction};
use crate::handlers::{self, HandlerContext};

/// Exit codes based on BSD sysexits.h
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const USAGE: i32 = 64; // EX_USAGE: command line usage error
    pub const UNAVAILABLE: i32 = 69; // EX_UNAVAILABLE: service unavailable
    pub const CANTCREAT: i32 = 73; // EX_CANTCREAT: can't create output
    pub const IOERR: i32 = 74; // EX_IOERR: input/output error
    pub const TEMPFAIL: i32 = 75; // EX_TEMPFAIL: temporary failure
}

/// Application encapsulates the CLI runtime behavior.
pub struct Application;

impl Application {
    /// Create a new Application instance.
    pub fn new() -> Self {
        Self
    }

    /// Run the application, returning the exit code.
    pub fn run(&self) -> i32 {
        match self.execute() {
            Ok(()) => exit_codes::SUCCESS,
            Err(e) => self.handle_error(e),
        }
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::parse();
        color_init(cli.no_color);

        // Handle commands that don't need a daemon connection
        if self.handle_standalone_commands(&cli)? {
            return Ok(());
        }

        // Connect to daemon
        let mut client = self.connect_to_daemon()?;

        // Check for version mismatch (skip for daemon commands and version command)
        if !matches!(cli.command, Commands::Daemon(_) | Commands::Version) {
            check_version_mismatch(&mut client);
        }

        // Execute command
        let format = cli.effective_format();
        let mut ctx = HandlerContext::new(&mut client, cli.session, format);
        self.dispatch_command(&mut ctx, &cli.command)
    }

    /// Handle commands that don't require a daemon connection.
    /// Returns true if the command was handled, false if it needs daemon.
    fn handle_standalone_commands(&self, cli: &Cli) -> Result<bool, Box<dyn std::error::Error>> {
        match &cli.command {
            Commands::Daemon(DaemonCommand::Start { foreground: true }) => {
                start_daemon()?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Start { foreground: false }) => {
                agent_tui_ipc::start_daemon_background()?;
                println!("Daemon started in background");
                Ok(true)
            }
            Commands::Completions { shell } => {
                let mut cmd = Cli::command();
                generate(*shell, &mut cmd, "agent-tui", &mut std::io::stdout());
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn connect_to_daemon(&self) -> Result<impl DaemonClient, Box<dyn std::error::Error>> {
        ensure_daemon().map_err(|e| {
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
            e.into()
        })
    }

    fn dispatch_command<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        command: &Commands,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            Commands::Daemon(daemon_cmd) => match daemon_cmd {
                DaemonCommand::Start { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Stop { force } => handlers::handle_daemon_stop(ctx, *force)?,
                DaemonCommand::Status => handlers::handle_daemon_status(ctx)?,
                DaemonCommand::Restart => handlers::handle_daemon_restart(ctx)?,
            },
            Commands::Completions { .. } => unreachable!("Handled in standalone"),

            Commands::Run {
                command,
                args,
                cwd,
                cols,
                rows,
            } => handlers::handle_spawn(
                ctx,
                command.clone(),
                args.clone(),
                cwd.clone(),
                *cols,
                *rows,
            )?,

            Commands::Snap {
                elements,
                accessibility,
                interactive_only,
                region,
                strip_ansi,
                include_cursor,
            } => {
                if *accessibility {
                    handlers::handle_accessibility_snapshot(ctx, *interactive_only)?
                } else {
                    handlers::handle_snapshot(
                        ctx,
                        *elements,
                        region.clone(),
                        *strip_ansi,
                        *include_cursor,
                    )?
                }
            }

            Commands::Click {
                element_ref,
                double,
            } => {
                if *double {
                    handlers::handle_dbl_click(ctx, element_ref.clone())?
                } else {
                    handlers::handle_click(ctx, element_ref.clone())?
                }
            }
            Commands::Fill { element_ref, value } => {
                handlers::handle_fill(ctx, element_ref.clone(), value.clone())?
            }

            Commands::Key {
                key,
                text,
                hold,
                release,
            } => {
                if let Some(text) = text {
                    handlers::handle_type(ctx, text.clone())?
                } else if let Some(key) = key {
                    if *hold {
                        handlers::handle_keydown(ctx, key.clone())?
                    } else if *release {
                        handlers::handle_keyup(ctx, key.clone())?
                    } else {
                        handlers::handle_press(ctx, key.clone())?
                    }
                }
            }

            Commands::Wait { params } => handlers::handle_wait(ctx, params.clone())?,
            Commands::Kill => handlers::handle_kill(ctx)?,
            Commands::Restart => handlers::handle_restart(ctx)?,
            Commands::Ls => handlers::handle_sessions(ctx)?,
            Commands::Status { verbose } => handlers::handle_health(ctx, *verbose)?,

            Commands::Select {
                element_ref,
                option,
            } => handlers::handle_select(ctx, element_ref.clone(), option.clone())?,
            Commands::MultiSelect {
                element_ref,
                options,
            } => handlers::handle_multiselect(ctx, element_ref.clone(), options.clone())?,

            Commands::Scroll {
                direction,
                amount,
                element: _,
                to_ref,
            } => {
                if let Some(element_ref) = to_ref {
                    handlers::handle_scroll_into_view(ctx, element_ref.clone())?
                } else if let Some(dir) = direction {
                    handlers::handle_scroll(ctx, *dir, *amount)?
                }
            }

            Commands::Focus { element_ref } => handlers::handle_focus(ctx, element_ref.clone())?,
            Commands::Clear { element_ref } => handlers::handle_clear(ctx, element_ref.clone())?,
            Commands::SelectAll { element_ref } => {
                handlers::handle_select_all(ctx, element_ref.clone())?
            }

            Commands::Count { role, name, text } => {
                handlers::handle_count(ctx, role.clone(), name.clone(), text.clone())?
            }

            Commands::Toggle {
                element_ref,
                on,
                off,
            } => {
                let state = if *on {
                    Some(true)
                } else if *off {
                    Some(false)
                } else {
                    None
                };
                handlers::handle_toggle(ctx, element_ref.clone(), state)?
            }

            Commands::RecordStart => handlers::handle_record_start(ctx)?,
            Commands::RecordStop {
                output,
                record_format,
            } => handlers::handle_record_stop(ctx, output.clone(), *record_format)?,
            Commands::RecordStatus => handlers::handle_record_status(ctx)?,

            Commands::Trace { count, start, stop } => {
                handlers::handle_trace(ctx, *count, *start, *stop)?
            }
            Commands::Console { lines, clear } => handlers::handle_console(ctx, *lines, *clear)?,
            Commands::Errors { count, clear } => handlers::handle_errors(ctx, *count, *clear)?,

            Commands::Resize { cols, rows } => handlers::handle_resize(ctx, *cols, *rows)?,
            Commands::Attach {
                session_id,
                interactive,
            } => handlers::handle_attach(ctx, session_id.clone(), *interactive)?,

            Commands::Version => handlers::handle_version(ctx)?,
            Commands::Env => handlers::handle_env(ctx)?,
            Commands::Assert { condition } => handlers::handle_assert(ctx, condition.clone())?,
            Commands::Cleanup { all } => handlers::handle_cleanup(ctx, *all)?,
            Commands::Find { params } => handlers::handle_find(ctx, params.clone())?,

            Commands::Debug(debug_cmd) => match debug_cmd {
                DebugCommand::Record(action) => match action {
                    RecordAction::Start => handlers::handle_record_start(ctx)?,
                    RecordAction::Stop { output, format } => {
                        handlers::handle_record_stop(ctx, output.clone(), *format)?
                    }
                    RecordAction::Status => handlers::handle_record_status(ctx)?,
                },
                DebugCommand::Trace { count, start, stop } => {
                    handlers::handle_trace(ctx, *count, *start, *stop)?
                }
                DebugCommand::Console { lines, clear } => {
                    handlers::handle_console(ctx, *lines, *clear)?
                }
                DebugCommand::Errors { count, clear } => {
                    handlers::handle_errors(ctx, *count, *clear)?
                }
                DebugCommand::Env => handlers::handle_env(ctx)?,
            },
        }
        Ok(())
    }

    fn handle_error(&self, e: Box<dyn std::error::Error>) -> i32 {
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
            exit_code_for_client_error(client_error)
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
            attach_error.exit_code()
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
            exit_codes::IOERR
        } else {
            eprintln!("{} {}", Colors::error("Error:"), e);
            exit_codes::GENERAL_ERROR
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

/// Check for version mismatch between CLI and daemon, print warning if found.
fn check_version_mismatch<C: DaemonClient>(client: &mut C) {
    use agent_tui_ipc::version::{VersionCheckResult, check_version};

    match check_version(client, env!("CARGO_PKG_VERSION")) {
        VersionCheckResult::Match => {}
        VersionCheckResult::Mismatch(mismatch) => {
            eprintln!(
                "{} CLI version ({}) differs from daemon version ({})",
                Colors::warning("Warning:"),
                mismatch.cli_version,
                mismatch.daemon_version
            );
            eprintln!(
                "{} Run '{}' to update the daemon.",
                Colors::dim("Hint:"),
                Colors::info("agent-tui daemon restart")
            );
            eprintln!();
        }
        VersionCheckResult::CheckFailed(err) => {
            eprintln!(
                "{} Could not check daemon version: {}",
                Colors::dim("Note:"),
                err
            );
        }
    }
}

fn exit_code_for_client_error(error: &ClientError) -> i32 {
    use agent_tui_ipc::error_codes::ErrorCategory;

    match error.category() {
        Some(ErrorCategory::InvalidInput) => exit_codes::USAGE,
        Some(ErrorCategory::NotFound) => exit_codes::UNAVAILABLE,
        Some(ErrorCategory::Busy) => exit_codes::CANTCREAT,
        Some(ErrorCategory::External) => exit_codes::IOERR,
        Some(ErrorCategory::Internal) => exit_codes::IOERR,
        Some(ErrorCategory::Timeout) => exit_codes::TEMPFAIL,
        None => exit_codes::GENERAL_ERROR,
    }
}
