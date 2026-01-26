use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use regex::Regex;
use std::sync::LazyLock;

pub mod attach;
pub mod commands;
pub mod error;
pub mod handlers;

use crate::app::commands::OutputFormat;
use crate::common::key_names::is_key_name;
use crate::common::telemetry;
use crate::common::{Colors, color_init};
use crate::infra::daemon::{DaemonError, start_daemon};
use crate::infra::ipc::{ClientError, DaemonClient, UnixSocketClient, ensure_daemon};
use tracing::debug;

use crate::app::attach::AttachError;
use crate::app::commands::{Cli, Commands, DaemonCommand, LiveCommand, LiveStartArgs};
use crate::app::handlers::HandlerContext;

static ELEMENT_REF_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@(e|btn|inp)\d+$").expect("Invalid element ref regex"));
const PROGRAM_NAME: &str = "agent-tui";

/// Exit codes following sysexits.h and LSB init script conventions.
///
/// LSB init script exit codes for daemon status:
/// - 0: Program is running and OK
/// - 1: Program is dead but pid file exists
/// - 3: Program is not running
/// - 4: Program status is unknown
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    /// LSB: program is not running (for `daemon status`)
    pub const NOT_RUNNING: i32 = 3;
    pub const USAGE: i32 = 64;
    pub const UNAVAILABLE: i32 = 69;
    pub const CANTCREAT: i32 = 73;
    pub const IOERR: i32 = 74;
    pub const TEMPFAIL: i32 = 75;
}

/// Error indicating daemon is not running (for status command).
/// Maps to LSB exit code 3.
#[derive(Debug)]
struct DaemonNotRunningError;

impl std::fmt::Display for DaemonNotRunningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Daemon is not running")
    }
}

impl std::error::Error for DaemonNotRunningError {}

/// Validates element ref format: @e1, @btn2, @inp3, etc.
fn is_element_ref(selector: &str) -> bool {
    ELEMENT_REF_REGEX.is_match(selector)
}

/// Extracts text from a text selector like @"Yes, proceed" or @Submit.
/// Returns an error for malformed quoted selectors.
fn extract_text_selector(selector: &str) -> Result<Option<&str>, Box<dyn std::error::Error>> {
    let Some(after_at) = selector.strip_prefix('@') else {
        return Ok(None);
    };

    if after_at.starts_with('"') {
        if !after_at.ends_with('"') || after_at.len() < 2 {
            return Err(format!(
                "Malformed quoted selector: {}. Missing closing quote.",
                selector
            )
            .into());
        }
        let text = &after_at[1..after_at.len() - 1];
        if text.is_empty() {
            return Err("Text selector cannot be empty".into());
        }
        return Ok(Some(text));
    }

    if after_at.is_empty() {
        return Err("Text selector cannot be empty".into());
    }

    Ok(Some(after_at))
}

/// Extracts element ref from find RPC result.
fn extract_element_ref_from_result(result: &serde_json::Value) -> Option<String> {
    result
        .get("elements")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|el| el.get("ref"))
        .and_then(|v| v.as_str())
        .map(String::from)
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
        let _telemetry = telemetry::init_tracing(if cli.verbose { "debug" } else { "warn" });
        color_init(cli.no_color);
        let format = cli.effective_format();
        debug!(
            command = ?cli.command,
            session = ?cli.session,
            format = ?format,
            "CLI command parsed"
        );

        if self
            .handle_standalone_commands(&cli)
            .map_err(|e| self.wrap_error(e, format))?
        {
            return Ok(());
        }

        let mut client: UnixSocketClient = match &cli.command {
            Commands::Run { .. } => self
                .connect_to_daemon_autostart()
                .map_err(|e| self.wrap_error(e, format))?,
            _ => self
                .connect_to_daemon_no_autostart()
                .map_err(|e| self.wrap_error(e, format))?,
        };

        if !matches!(cli.command, Commands::Daemon(_) | Commands::Version) {
            check_version_mismatch(&mut client);
        }

        let mut ctx = HandlerContext::new(&mut client, cli.session, format);
        self.dispatch_command(&mut ctx, &cli.command, cli.verbose)
            .map_err(|e| self.wrap_error(e, format))
    }

    fn handle_standalone_commands(&self, cli: &Cli) -> Result<bool, Box<dyn std::error::Error>> {
        match &cli.command {
            Commands::Daemon(DaemonCommand::Start { foreground: true }) => {
                start_daemon()?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Start { foreground: false }) => {
                crate::infra::ipc::start_daemon_background()?;
                println!("Daemon started in background");
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Status) => {
                self.handle_daemon_status_without_autostart(cli)?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Stop { force }) => {
                self.handle_daemon_stop_without_autostart(*force)?;
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

    fn handle_daemon_status_without_autostart(
        &self,
        cli: &Cli,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match UnixSocketClient::connect() {
            Ok(mut client) => {
                // Verify daemon is actually responding before showing status
                match client.call("health", None) {
                    Ok(result) => {
                        let format = cli.effective_format();
                        handlers::print_daemon_status_from_result(&result, format);
                        Ok(())
                    }
                    Err(_) => {
                        // Connected but daemon not responding - treat as not running
                        self.print_daemon_not_running_status(cli);
                        Err(Box::new(DaemonNotRunningError))
                    }
                }
            }
            Err(ClientError::DaemonNotRunning) => {
                self.print_daemon_not_running_status(cli);
                Err(Box::new(DaemonNotRunningError))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn print_daemon_not_running_status(&self, cli: &Cli) {
        let cli_version = env!("AGENT_TUI_VERSION");
        let cli_commit = env!("AGENT_TUI_GIT_SHA");
        match cli.effective_format() {
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "running": false,
                        "cli_version": cli_version,
                        "cli_commit": cli_commit
                    })
                );
            }
            _ => {
                println!("Daemon is not running");
                println!("  CLI version: {}", cli_version);
                println!("  CLI commit: {}", cli_commit);
            }
        }
    }

    fn handle_daemon_stop_without_autostart(
        &self,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match handlers::stop_daemon_core(force)? {
            handlers::StopResult::Stopped { warnings, .. } => {
                for warning in &warnings {
                    eprintln!("{}", Colors::warning(warning));
                }
                println!("{}", Colors::success("✓ Daemon stopped"));
            }
            handlers::StopResult::AlreadyStopped => {
                println!("Daemon is not running (already stopped)");
            }
        }
        Ok(())
    }

    fn connect_to_daemon_autostart(&self) -> Result<UnixSocketClient, Box<dyn std::error::Error>> {
        ensure_daemon().map_err(Into::into)
    }

    fn connect_to_daemon_no_autostart(
        &self,
    ) -> Result<UnixSocketClient, Box<dyn std::error::Error>> {
        match UnixSocketClient::connect() {
            Ok(client) => Ok(client),
            Err(ClientError::DaemonNotRunning) => Err(Box::new(ClientError::DaemonNotRunning)),
            Err(e) => Err(e.into()),
        }
    }

    fn dispatch_command<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        command: &Commands,
        verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            Commands::Daemon(daemon_cmd) => match daemon_cmd {
                DaemonCommand::Start { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Stop { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Status => unreachable!("Handled in standalone"),
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

            Commands::Screenshot {
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
                use crate::app::commands::{ActionOperation, ToggleState};
                match operation.as_ref() {
                    None | Some(ActionOperation::Click) => {
                        handlers::handle_click(ctx, element_ref.clone())?
                    }
                    Some(ActionOperation::DblClick) => {
                        handlers::handle_dbl_click(ctx, element_ref.clone())?
                    }
                    Some(ActionOperation::Fill { value }) => {
                        handlers::handle_fill(ctx, element_ref.clone(), value.clone())?
                    }
                    Some(ActionOperation::Select { options }) => {
                        handlers::handle_select(ctx, element_ref.clone(), options.clone())?
                    }
                    Some(ActionOperation::Toggle { state }) => {
                        let state_bool = match state {
                            Some(ToggleState::On) => Some(true),
                            Some(ToggleState::Off) => Some(false),
                            None => None,
                        };
                        handlers::handle_toggle(ctx, element_ref.clone(), state_bool)?
                    }
                    Some(ActionOperation::Focus) => {
                        handlers::handle_focus(ctx, element_ref.clone())?
                    }
                    Some(ActionOperation::Clear) => {
                        handlers::handle_clear(ctx, element_ref.clone())?
                    }
                    Some(ActionOperation::SelectAll) => {
                        handlers::handle_select_all(ctx, element_ref.clone())?
                    }
                    Some(ActionOperation::Scroll { direction, amount }) => {
                        handlers::handle_scroll(ctx, *direction, *amount)?
                    }
                }
            }

            Commands::Press { keys } => {
                for key in keys {
                    handlers::handle_press(ctx, key.to_string())?;
                }
            }

            Commands::Type { text } => handlers::handle_type(ctx, text.to_string())?,

            Commands::Input {
                value,
                hold,
                release,
            } => {
                if *hold {
                    handlers::handle_keydown(ctx, value.clone())?
                } else if *release {
                    handlers::handle_keyup(ctx, value.clone())?
                } else if is_key_name(value) {
                    handlers::handle_press(ctx, value.clone())?
                } else {
                    handlers::handle_type(ctx, value.clone())?
                }
            }

            Commands::Wait { params } => handlers::handle_wait(ctx, params.clone())?,
            Commands::Kill => handlers::handle_kill(ctx)?,

            Commands::Sessions { command } => {
                use crate::app::commands::SessionsCommand;

                match command {
                    None | Some(SessionsCommand::List) => handlers::handle_sessions(ctx)?,
                    Some(SessionsCommand::Show { session_id }) => {
                        handlers::handle_session_show(ctx, session_id.clone())?
                    }
                    Some(SessionsCommand::Attach {
                        session_id,
                        no_tty,
                        detach_keys,
                    }) => {
                        let attach_id =
                            handlers::resolve_attach_session_id(ctx, session_id.clone())?;
                        handlers::handle_attach(ctx, attach_id, !*no_tty, detach_keys.clone())?
                    }
                    Some(SessionsCommand::Cleanup { all }) => handlers::handle_cleanup(ctx, *all)?,
                    Some(SessionsCommand::Status) => handlers::handle_health(ctx, verbose)?,
                }
            }

            Commands::Live { command } => match command {
                None => handlers::handle_live_start(ctx, LiveStartArgs::default())?,
                Some(LiveCommand::Start(args)) => handlers::handle_live_start(ctx, args.clone())?,
                Some(LiveCommand::Stop) => handlers::handle_live_stop(ctx)?,
                Some(LiveCommand::Status) => handlers::handle_live_status(ctx)?,
            },

            Commands::Version => handlers::handle_version(ctx)?,
            Commands::Env => handlers::handle_env(ctx)?,

            Commands::External(args) => self.dispatch_selector_action(ctx, args)?,
        }
        Ok(())
    }

    fn dispatch_selector_action<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        args: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if args.is_empty() {
            return Err("No element selector provided".into());
        }

        if !args[0].starts_with('@') && !args[0].starts_with(':') {
            return Err(format!(
                "Unknown command: {}. Run 'agent-tui --help' to see available commands.",
                args[0]
            )
            .into());
        }

        let element_ref = self.resolve_selector(ctx, &args[0])?;

        match args.get(1).map(|s| s.as_str()) {
            None => handlers::handle_click(ctx, element_ref)?,
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
            Some("fill") => {
                let value = args.get(2).ok_or("fill requires a value")?;
                handlers::handle_fill(ctx, element_ref, value.to_string())?
            }
            Some(value) => handlers::handle_fill(ctx, element_ref, value.to_string())?,
        }
        Ok(())
    }

    fn resolve_selector<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        selector: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if is_element_ref(selector) {
            return Ok(selector.to_string());
        }

        if let Some(text) = selector.strip_prefix(':') {
            if text.is_empty() {
                return Err("Partial text selector cannot be empty".into());
            }
            return self.find_element_by_text(ctx, text, false);
        }

        if let Some(text) = extract_text_selector(selector)? {
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
        use crate::infra::ipc::params;

        let find_params = params::FindParams {
            session: ctx.session.clone(),
            text: Some(text.to_string()),
            exact,
            nth: Some(0),
            ..Default::default()
        };
        let params_json = serde_json::to_value(find_params)?;
        let result = ctx.client.call("find", Some(params_json))?;

        if let Some(ref_str) = extract_element_ref_from_result(&result) {
            return Ok(ref_str);
        }

        let match_type = if exact { "exact" } else { "partial" };
        Err(format!("No element found with {} text: {}", match_type, text).into())
    }

    fn handle_error(&self, e: Box<dyn std::error::Error>) -> i32 {
        // Handle DaemonNotRunningError specially - no error message printed,
        // output was already shown by the handler, just return LSB exit code 3
        if e.downcast_ref::<DaemonNotRunningError>().is_some() {
            return exit_codes::NOT_RUNNING;
        }

        if let Some(cli_error) = e.downcast_ref::<crate::app::error::CliError>() {
            print_cli_error(cli_error);
            return cli_error.exit_code;
        }

        if let Some(client_error) = e.downcast_ref::<ClientError>() {
            eprintln!(
                "{}: {} {}",
                PROGRAM_NAME,
                Colors::error("Error:"),
                client_error
            );
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
            eprintln!(
                "{}: {} {}",
                PROGRAM_NAME,
                Colors::error("Error:"),
                attach_error
            );
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
            eprintln!(
                "{}: {} {}",
                PROGRAM_NAME,
                Colors::error("Error:"),
                daemon_error
            );
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
            eprintln!("{}: {} {}", PROGRAM_NAME, Colors::error("Error:"), e);
            exit_codes::GENERAL_ERROR
        }
    }
}

impl Application {
    fn wrap_error(
        &self,
        error: Box<dyn std::error::Error>,
        format: OutputFormat,
    ) -> Box<dyn std::error::Error> {
        if error.downcast_ref::<DaemonNotRunningError>().is_some() {
            return error;
        }
        if error
            .downcast_ref::<crate::app::error::CliError>()
            .is_some()
        {
            return error;
        }
        if format != OutputFormat::Json {
            return error;
        }

        if let Some(client_error) = error.downcast_ref::<ClientError>() {
            return Box::new(crate::app::error::CliError::new(
                format,
                client_error.to_string(),
                Some(client_error.to_json()),
                exit_code_for_client_error(client_error),
            ));
        }
        if let Some(attach_error) = error.downcast_ref::<AttachError>() {
            return Box::new(crate::app::error::CliError::new(
                format,
                attach_error.to_string(),
                Some(attach_error.to_json()),
                attach_error.exit_code(),
            ));
        }
        if let Some(daemon_error) = error.downcast_ref::<DaemonError>() {
            return Box::new(crate::app::error::CliError::new(
                format,
                daemon_error.to_string(),
                None,
                exit_codes::IOERR,
            ));
        }

        Box::new(crate::app::error::CliError::new(
            format,
            error.to_string(),
            None,
            exit_codes::GENERAL_ERROR,
        ))
    }
}

fn print_cli_error(error: &crate::app::error::CliError) {
    match error.format {
        OutputFormat::Json => {
            if let Some(json) = &error.json {
                eprintln!("{}", serde_json::to_string_pretty(json).unwrap_or_default());
            } else {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "success": false,
                        "error": error.message
                    })
                );
            }
        }
        OutputFormat::Text => {
            eprintln!(
                "{}: {} {}",
                PROGRAM_NAME,
                Colors::error("Error:"),
                error.message
            );
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

fn check_version_mismatch<C: DaemonClient>(client: &mut C) {
    use crate::infra::ipc::version::{VersionCheckResult, check_version};

    match check_version(client, env!("AGENT_TUI_VERSION"), env!("AGENT_TUI_GIT_SHA")) {
        VersionCheckResult::Match => {}
        VersionCheckResult::Mismatch(mismatch) => {
            if mismatch.cli_version == mismatch.daemon_version
                && mismatch.cli_commit != mismatch.daemon_commit
            {
                eprintln!(
                    "{} CLI commit ({}) differs from daemon commit ({})",
                    Colors::warning("Warning:"),
                    mismatch.cli_commit,
                    mismatch.daemon_commit
                );
            } else {
                eprintln!(
                    "{} CLI version ({}) differs from daemon version ({})",
                    Colors::warning("Warning:"),
                    mismatch.cli_version,
                    mismatch.daemon_version
                );
            }
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
    use crate::infra::ipc::error_codes::ErrorCategory;

    if matches!(error, ClientError::DaemonNotRunning) {
        return exit_codes::UNAVAILABLE;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    mod is_element_ref_tests {
        use super::*;

        #[test]
        fn valid_element_refs() {
            assert!(is_element_ref("@e1"));
            assert!(is_element_ref("@e42"));
            assert!(is_element_ref("@btn1"));
            assert!(is_element_ref("@btn999"));
            assert!(is_element_ref("@inp1"));
            assert!(is_element_ref("@inp123"));
        }

        #[test]
        fn invalid_element_refs() {
            assert!(!is_element_ref("@e"));
            assert!(!is_element_ref("@btn"));
            assert!(!is_element_ref("@inp"));
            assert!(!is_element_ref("@elephant"));
            assert!(!is_element_ref("@button"));
            assert!(!is_element_ref("@e1a"));
            assert!(!is_element_ref("e1"));
            assert!(!is_element_ref("@E1"));
            assert!(!is_element_ref("@Submit"));
            assert!(!is_element_ref(":Submit"));
        }
    }

    mod extract_text_selector_tests {
        use super::*;

        #[test]
        fn quoted_text_selector() {
            assert_eq!(
                extract_text_selector("@\"Yes, proceed\"").unwrap(),
                Some("Yes, proceed")
            );
            assert_eq!(
                extract_text_selector("@\"Submit\"").unwrap(),
                Some("Submit")
            );
        }

        #[test]
        fn unquoted_text_selector() {
            assert_eq!(extract_text_selector("@Submit").unwrap(), Some("Submit"));
            assert_eq!(
                extract_text_selector("@Yes, proceed").unwrap(),
                Some("Yes, proceed")
            );
        }

        #[test]
        fn malformed_quoted_selector_missing_end_quote() {
            let result = extract_text_selector("@\"Missing end");
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Missing closing quote")
            );
        }

        #[test]
        fn empty_quoted_selector() {
            let result = extract_text_selector("@\"\"");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn empty_unquoted_selector() {
            let result = extract_text_selector("@");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        }

        #[test]
        fn non_at_prefix_returns_none() {
            assert_eq!(extract_text_selector(":Submit").unwrap(), None);
            assert_eq!(extract_text_selector("Submit").unwrap(), None);
        }
    }

    mod extract_element_ref_from_result_tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn extracts_ref_from_valid_result() {
            let result = json!({
                "elements": [{"ref": "@e1", "text": "Submit"}]
            });
            assert_eq!(
                extract_element_ref_from_result(&result),
                Some("@e1".to_string())
            );
        }

        #[test]
        fn returns_none_for_empty_elements() {
            let result = json!({"elements": []});
            assert_eq!(extract_element_ref_from_result(&result), None);
        }

        #[test]
        fn returns_none_for_missing_ref() {
            let result = json!({
                "elements": [{"text": "Submit"}]
            });
            assert_eq!(extract_element_ref_from_result(&result), None);
        }

        #[test]
        fn returns_none_for_null_ref() {
            let result = json!({
                "elements": [{"ref": null}]
            });
            assert_eq!(extract_element_ref_from_result(&result), None);
        }

        #[test]
        fn returns_none_for_missing_elements() {
            let result = json!({"success": true});
            assert_eq!(extract_element_ref_from_result(&result), None);
        }
    }

    mod daemon_standalone_tests {
        use super::*;
        use crate::app::commands::{Cli, Commands, DaemonCommand, OutputFormat};
        use std::env;
        use tempfile::TempDir;

        #[test]
        fn handle_standalone_commands_routes_daemon_status() {
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            env::set_var("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = Cli {
                command: Commands::Daemon(DaemonCommand::Status),
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                verbose: false,
            };

            // When daemon is not running, should return DaemonNotRunningError
            let result = app.handle_standalone_commands(&cli);
            assert!(result.is_err());
            let err = result.unwrap_err();
            // Verify it's DaemonNotRunningError (can be converted to exit code 3)
            assert!(err.downcast_ref::<DaemonNotRunningError>().is_some());
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_stop() {
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            env::set_var("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = Cli {
                command: Commands::Daemon(DaemonCommand::Stop { force: false }),
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                verbose: false,
            };

            // When daemon is not running, should succeed (idempotent semantics)
            // The result should be Ok(true), indicating the command was handled
            let result = app.handle_standalone_commands(&cli);
            assert!(
                result.is_ok(),
                "daemon stop should succeed when daemon not running (idempotent)"
            );
            assert!(
                result.unwrap(),
                "daemon stop should be handled as standalone"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_start() {
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            env::set_var("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = Cli {
                command: Commands::Daemon(DaemonCommand::Start { foreground: false }),
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                verbose: false,
            };

            // Verify daemon start IS handled as standalone (returns Ok(true) or error)
            // This test runs without a daemon, so start will either succeed or fail,
            // but importantly it returns Ok(true) indicating "handled" (not Ok(false))
            let result = app.handle_standalone_commands(&cli);
            // Error is acceptable (daemon may fail to start), but it was handled
            if let Ok(handled) = result {
                assert!(handled, "daemon start should be handled as standalone");
            }
        }

        #[test]
        fn handle_standalone_commands_does_not_route_restart() {
            let app = Application::new();
            let cli = Cli {
                command: Commands::Daemon(DaemonCommand::Restart),
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                verbose: false,
            };

            // Restart should NOT be handled as standalone
            let result = app.handle_standalone_commands(&cli);
            assert!(result.is_ok());
            assert!(!result.unwrap());
        }

        #[test]
        fn handle_error_returns_not_running_exit_code() {
            let app = Application::new();
            let err: Box<dyn std::error::Error> = Box::new(DaemonNotRunningError);
            let exit_code = app.handle_error(err);
            assert_eq!(exit_code, exit_codes::NOT_RUNNING);
        }
    }
}
