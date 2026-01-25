use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;

use crate::common::key_names::is_key_name;
use crate::common::{Colors, color_init};
use crate::daemon::{DaemonError, start_daemon};
use crate::ipc::{ClientError, DaemonClient, ensure_daemon};

use crate::attach::AttachError;
use crate::commands::{Cli, Commands, DaemonCommand, DebugCommand, RecordAction};
use crate::handlers::{self, HandlerContext};

mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const USAGE: i32 = 64;
    pub const UNAVAILABLE: i32 = 69;
    pub const CANTCREAT: i32 = 73;
    pub const IOERR: i32 = 74;
    pub const TEMPFAIL: i32 = 75;
}

pub struct Application;

impl Application {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> i32 {
        match self.execute() {
            Ok(()) => exit_codes::SUCCESS,
            Err(e) => self.handle_error(e),
        }
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::parse();
        color_init(cli.no_color);

        if self.handle_standalone_commands(&cli)? {
            return Ok(());
        }

        let mut client = self.connect_to_daemon()?;

        if !matches!(cli.command, Commands::Daemon(_) | Commands::Version) {
            check_version_mismatch(&mut client);
        }

        let format = cli.effective_format();
        let mut ctx = HandlerContext::new(&mut client, cli.session, format);
        self.dispatch_command(&mut ctx, &cli.command)
    }

    fn handle_standalone_commands(&self, cli: &Cli) -> Result<bool, Box<dyn std::error::Error>> {
        match &cli.command {
            Commands::Daemon(DaemonCommand::Start { foreground: true }) => {
                start_daemon()?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Start { foreground: false }) => {
                crate::ipc::start_daemon_background()?;
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

            Commands::Screen {
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

            Commands::Action {
                element_ref,
                operation,
            } => {
                use crate::commands::{ActionOperation, ToggleState};
                match operation {
                    ActionOperation::Click => handlers::handle_click(ctx, element_ref.clone())?,
                    ActionOperation::DblClick => {
                        handlers::handle_dbl_click(ctx, element_ref.clone())?
                    }
                    ActionOperation::Fill { value } => {
                        handlers::handle_fill(ctx, element_ref.clone(), value.clone())?
                    }
                    ActionOperation::Select { options } => {
                        handlers::handle_select(ctx, element_ref.clone(), options.clone())?
                    }
                    ActionOperation::Toggle { state } => {
                        let state_bool = match state {
                            Some(ToggleState::On) => Some(true),
                            Some(ToggleState::Off) => Some(false),
                            None => None,
                        };
                        handlers::handle_toggle(ctx, element_ref.clone(), state_bool)?
                    }
                    ActionOperation::Focus => handlers::handle_focus(ctx, element_ref.clone())?,
                    ActionOperation::Clear => handlers::handle_clear(ctx, element_ref.clone())?,
                    ActionOperation::SelectAll => {
                        handlers::handle_select_all(ctx, element_ref.clone())?
                    }
                    ActionOperation::Scroll { direction, amount } => {
                        handlers::handle_scroll(ctx, *direction, *amount)?
                    }
                }
            }

            Commands::Press { keys } => {
                for key in keys {
                    handlers::handle_press(ctx, key.clone())?;
                }
            }

            Commands::Type { text } => handlers::handle_type(ctx, text.clone())?,

            Commands::Input {
                value,
                hold,
                release,
            } => {
                if let Some(input) = value {
                    if *hold {
                        handlers::handle_keydown(ctx, input.clone())?
                    } else if *release {
                        handlers::handle_keyup(ctx, input.clone())?
                    } else if is_key_name(input) {
                        handlers::handle_press(ctx, input.clone())?
                    } else {
                        handlers::handle_type(ctx, input.clone())?
                    }
                } else if *hold || *release {
                    return Err("--hold and --release require a key name".into());
                }
            }

            Commands::Wait { params } => handlers::handle_wait(ctx, params.clone())?,
            Commands::Kill => handlers::handle_kill(ctx)?,

            Commands::Sessions {
                session_id,
                cleanup,
                all,
                attach,
                status,
            } => {
                if let Some(attach_id) = attach {
                    handlers::handle_attach(ctx, attach_id.clone(), true)?
                } else if *cleanup {
                    handlers::handle_cleanup(ctx, *all)?
                } else if session_id.is_some() {
                    handlers::handle_sessions(ctx)?
                } else if *status {
                    handlers::handle_health(ctx, true)?
                } else {
                    handlers::handle_sessions(ctx)?
                }
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

            Commands::Version => handlers::handle_version(ctx)?,
            Commands::Env => handlers::handle_env(ctx)?,

            Commands::External(args) => self.handle_element_ref(ctx, args.clone())?,

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

    fn handle_element_ref<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        args: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if args.is_empty() {
            return Err("No element selector provided".into());
        }

        let selector = &args[0];

        // Parse the selector type and resolve to element ref
        let element_ref = self.resolve_selector(ctx, selector)?;

        // Dispatch based on action
        match args.get(1).map(|s| s.as_str()) {
            None => {
                // Default action: activate (click)
                handlers::handle_click(ctx, element_ref)?
            }
            Some("toggle") => {
                let state = match args.get(2).map(|s| s.as_str()) {
                    Some("on") => Some(true),
                    Some("off") => Some(false),
                    _ => None,
                };
                handlers::handle_toggle(ctx, element_ref, state)?
            }
            Some("choose") => {
                let options: Vec<String> = args[2..].to_vec();
                if options.is_empty() {
                    return Err("choose requires at least one option".into());
                }
                handlers::handle_select(ctx, element_ref, options)?
            }
            Some("clear") => handlers::handle_clear(ctx, element_ref)?,
            Some("focus") => handlers::handle_focus(ctx, element_ref)?,
            Some(value) => {
                // Value provided - treat as fill
                handlers::handle_fill(ctx, element_ref, value.to_string())?
            }
        }
        Ok(())
    }

    fn resolve_selector<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        selector: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Standard element refs: @e1, @btn1, @inp1
        if selector.starts_with("@e")
            || selector.starts_with("@btn")
            || selector.starts_with("@inp")
        {
            return Ok(selector.to_string());
        }

        // Partial text selector: :Submit
        if let Some(text) = selector.strip_prefix(':') {
            return self.find_element_by_text(ctx, text, false);
        }

        // Text selector: @"text" or @text
        if let Some(after_at) = selector.strip_prefix('@') {
            // Check for quoted text: @"Yes, proceed"
            let text = after_at
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(after_at);
            return self.find_element_by_text(ctx, text, true);
        }

        Err(format!(
            "Invalid selector: {}. Use @e1, @\"text\", or :partial",
            selector
        )
        .into())
    }

    fn find_element_by_text<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        text: &str,
        exact: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use crate::ipc::params;

        let find_params = params::FindParams {
            session: ctx.session.clone(),
            text: Some(text.to_string()),
            exact,
            nth: Some(0),
            ..Default::default()
        };
        let params_json = serde_json::to_value(find_params)?;
        let result = ctx.client.call("find", Some(params_json))?;

        // Extract element ref from result
        if let Some(elements) = result.get("elements").and_then(|v| v.as_array()) {
            if let Some(first) = elements.first() {
                if let Some(ref_str) = first.get("ref").and_then(|v| v.as_str()) {
                    return Ok(ref_str.to_string());
                }
            }
        }

        let match_type = if exact { "exact" } else { "partial" };
        Err(format!("No element found with {} text: {}", match_type, text).into())
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

fn check_version_mismatch<C: DaemonClient>(client: &mut C) {
    use crate::ipc::version::{VersionCheckResult, check_version};

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
    use crate::ipc::error_codes::ErrorCategory;

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
