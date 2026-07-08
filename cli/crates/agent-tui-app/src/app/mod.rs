#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI application layer and composition root wiring.

use anyhow::Context;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::IsTerminal;
use std::io::Write;
use std::path::PathBuf;

pub mod attach;
pub mod commands;
pub mod daemon;
pub mod error;
pub mod handlers;
pub mod rpc_client;

use crate::app::commands::OutputFormat;
use crate::common::Colors;
use crate::common::DaemonError;
use crate::common::color_init;
use crate::common::telemetry;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;
use crate::infra::ipc::UnixSocketClient;
use crate::infra::ipc::ensure_daemon;
use tracing::debug;

use crate::app::attach::AttachError;
use crate::app::commands::Cli;
use crate::app::commands::Commands;
use crate::app::commands::CompletionShell;
use crate::app::commands::DaemonCommand;
use crate::app::commands::LegacyActionOperation;
use crate::app::commands::LegacyActionParseError;
use crate::app::commands::LiveCommand;
use crate::app::commands::LiveStartArgs;
use crate::app::commands::env_assignments_to_map;
use crate::app::commands::parse_legacy_action_invocation;
use crate::app::error::CliError;
use crate::app::error::DaemonNotRunningError;
use crate::app::handlers::HandlerContext;

const PROGRAM_NAME: &str = "agent-tui";

/// Exit codes following sysexits.h and LSB init script conventions.
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const NOT_RUNNING: i32 = 3;
    pub const USAGE: i32 = 64;
    pub const UNAVAILABLE: i32 = 69;
    pub const CANTCREAT: i32 = 73;
    pub const IOERR: i32 = 74;
    pub const TEMPFAIL: i32 = 75;
}

fn daemon_start_requests_foreground() -> bool {
    std::env::var("AGENT_TUI_DAEMON_FOREGROUND")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

#[derive(Debug, PartialEq, Eq)]
enum CompletionStatus {
    Missing,
    UpToDate,
    OutOfDate,
}

#[derive(Debug)]
enum InstallOutcome {
    Installed(PathBuf),
    Updated(PathBuf),
    AlreadyUpToDate(PathBuf),
}

fn legacy_action_compatibility_error(
    format: OutputFormat,
    parse_error: LegacyActionParseError,
) -> CliError {
    let selector = parse_error.selector();
    let operation = parse_error.operation();
    let message =
        format!("Legacy action `{operation}` for selector `{selector}` is not supported.");
    let suggestion = "Use `agent-tui press`, `agent-tui type`, or `agent-tui scroll`.";
    let json = if format == OutputFormat::Json {
        let output = serde_json::json!({
            "code": exit_codes::USAGE,
            "message": message.as_str(),
            "category": "invalid_input",
            "retryable": false,
            "suggestion": suggestion,
            "context": {
                "selector": selector,
                "operation": operation,
            }
        });
        Some(output.to_string())
    } else {
        None
    };

    CliError::new(
        format,
        format!("{message} {suggestion}"),
        json,
        exit_codes::USAGE,
    )
}

fn legacy_action_compatibility_result(
    format: OutputFormat,
    form: &[String],
) -> Result<crate::app::commands::LegacyActionInvocation> {
    parse_legacy_action_invocation(form)
        .map_err(|err| anyhow::Error::new(legacy_action_compatibility_error(format, err)))
}

fn parse_legacy_scroll_into_view_invocation(
    format: OutputFormat,
    form: &[String],
) -> Result<String> {
    match form {
        [selector] => Ok(selector.clone()),
        [selector, unsupported, ..] => {
            Err(legacy_scroll_into_view_compatibility_error(format, selector, unsupported).into())
        }
        [] => {
            Err(legacy_scroll_into_view_compatibility_error(format, "<missing>", "missing").into())
        }
    }
}

fn legacy_scroll_into_view_compatibility_error(
    format: OutputFormat,
    selector: &str,
    unsupported: &str,
) -> CliError {
    let selector = if selector.is_empty() {
        "<missing>"
    } else {
        selector
    };
    let unsupported = if unsupported.is_empty() {
        "<missing>"
    } else {
        unsupported
    };
    let message = format!(
        "Legacy scroll-into-view option `{unsupported}` for selector `{selector}` is not supported."
    );
    let suggestion = "Use `agent-tui scroll <direction> [amount]` or `agent-tui press`.";
    let json = if format == OutputFormat::Json {
        let output = serde_json::json!({
            "code": exit_codes::USAGE,
            "message": message.as_str(),
            "category": "invalid_input",
            "retryable": false,
            "suggestion": suggestion,
            "context": {
                "selector": selector,
                "unsupported": unsupported,
            }
        });
        Some(output.to_string())
    } else {
        None
    };

    CliError::new(
        format,
        format!("{message} {suggestion}"),
        json,
        exit_codes::USAGE,
    )
}

fn handle_legacy_scroll_into_view(format: OutputFormat, selector: &str) -> Result<()> {
    #[derive(Serialize)]
    struct LegacyScrollIntoViewOutput<'a> {
        success: bool,
        selector: &'a str,
        scrolled: bool,
        message: &'a str,
    }

    let message = "Legacy scroll-into-view compatibility did not send terminal input. Use `agent-tui scroll <direction> [amount]` or `agent-tui press`.";

    match format {
        OutputFormat::Json => {
            let output = LegacyScrollIntoViewOutput {
                success: true,
                selector,
                scrolled: false,
                message,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Text => {
            println!("{message}");
        }
    }

    Ok(())
}

fn handle_completions_command(
    format: OutputFormat,
    shell: Option<CompletionShell>,
    print: bool,
    install: bool,
    yes: bool,
    no_input: bool,
) -> Result<()> {
    if format == OutputFormat::Json {
        return handle_completions_command_json(shell, print, install);
    }

    if install {
        let shell = resolve_shell(shell).ok_or_else(|| {
            CliError::new(
                format,
                "Shell not specified. Re-run with `agent-tui completions --install <bash|zsh|fish|elvish>`."
                    .to_string(),
                None,
                exit_codes::USAGE,
            )
        })?;
        run_completions_wizard(shell, true, yes, no_input)?;
        return Ok(());
    }

    let stdout_tty = io::stdout().is_terminal();
    if print || !stdout_tty {
        let shell = resolve_shell(shell).ok_or_else(|| {
            CliError::new(
                format,
                "Shell not specified. Re-run with `agent-tui completions --print <bash|zsh|fish|elvish>`."
                    .to_string(),
                None,
                exit_codes::USAGE,
            )
        })?;
        let mut cmd = Cli::command();
        generate(
            shell.clap_shell(),
            &mut cmd,
            PROGRAM_NAME,
            &mut io::stdout(),
        );
        return Ok(());
    }

    if no_input {
        let shell = resolve_shell(shell).ok_or_else(|| {
            CliError::new(
                format,
                "Shell not detected. Re-run with `agent-tui completions <bash|zsh|fish|elvish> --no-input` or `agent-tui completions --print <shell>`."
                    .to_string(),
                None,
                exit_codes::USAGE,
            )
        })?;
        run_completions_wizard(shell, install, yes, true)?;
        return Ok(());
    }

    let shell = match resolve_shell(shell) {
        Some(shell) => shell,
        None => {
            print_shell_detection_help();
            return Ok(());
        }
    };

    run_completions_wizard(shell, install, yes, false)?;
    Ok(())
}

fn single_modifier_key(format: OutputFormat, flag: &str, keys: &[String]) -> Result<String> {
    match keys {
        [key] => Ok(key.clone()),
        _ => Err(anyhow::Error::new(CliError::new(
            format,
            format!("Press {flag} requires exactly one key (Ctrl, Alt, Shift, Meta)"),
            None,
            exit_codes::USAGE,
        ))),
    }
}

fn completions_cli_error(
    exit_code: i32,
    message: impl Into<String>,
    category: &'static str,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    let message = message.into();
    let mut output = serde_json::json!({
        "code": exit_code,
        "message": message.as_str(),
        "category": category,
        "retryable": false,
    });
    if let Some(context) = context {
        output["context"] = context;
    }
    if let Some(suggestion) = suggestion.as_deref() {
        output["suggestion"] = serde_json::json!(suggestion);
    }

    CliError::new(
        OutputFormat::Json,
        message,
        Some(serde_json::to_string_pretty(&output).unwrap_or_default()),
        exit_code,
    )
}

fn completions_usage_error(
    message: impl Into<String>,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    completions_cli_error(
        exit_codes::USAGE,
        message,
        "invalid_input",
        suggestion,
        context,
    )
}

fn completions_unavailable_error(
    message: impl Into<String>,
    suggestion: Option<String>,
    context: Option<serde_json::Value>,
) -> CliError {
    completions_cli_error(
        exit_codes::UNAVAILABLE,
        message,
        "not_found",
        suggestion,
        context,
    )
}

fn handle_completions_command_json(
    shell: Option<CompletionShell>,
    print: bool,
    install: bool,
) -> Result<()> {
    #[derive(Serialize)]
    struct CompletionJsonOutput {
        action: &'static str,
        shell: String,
        install_supported: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        install_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        install_recommended: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        suggested_install_command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        script: Option<String>,
    }

    let shell = resolve_shell(shell).ok_or_else(|| {
        completions_usage_error(
            "Shell not detected. Re-run with `agent-tui completions <bash|zsh|fish|elvish> --no-input` or `agent-tui completions --print <shell>`.",
            Some("Specify one of bash, zsh, fish, or elvish.".to_string()),
            Some(serde_json::json!({
                "supported_shells": ["bash", "zsh", "fish", "elvish"]
            })),
        )
    })?;

    let script = generate_completions_bytes(shell)?;
    let shell_name = shell_label(shell).to_string();

    if print {
        let output = CompletionJsonOutput {
            action: "print",
            shell: shell_name,
            install_supported: default_completion_path(shell).is_some(),
            install_path: default_completion_path(shell).map(|path| path.display().to_string()),
            status: None,
            result: None,
            install_recommended: None,
            suggested_install_command: None,
            script: Some(
                String::from_utf8(script).context("generated completions were not valid UTF-8")?,
            ),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let install_path = default_completion_path(shell);
    let install_path_string = install_path.as_ref().map(|path| path.display().to_string());

    if install {
        let path = install_path.ok_or_else(|| {
            completions_unavailable_error(
                "Automatic completion install is not available because the home directory could not be determined.",
                Some("Set HOME or use `agent-tui completions --print <shell>` instead.".to_string()),
                None,
            )
        })?;

        let outcome = install_completions(&script, &path)?;
        let result = install_outcome_label(&outcome);
        let output = CompletionJsonOutput {
            action: "install",
            shell: shell_name,
            install_supported: true,
            install_path: Some(path.display().to_string()),
            status: Some(result),
            result: Some(result),
            install_recommended: Some(false),
            suggested_install_command: None,
            script: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let status = install_path
        .as_ref()
        .map(|path| completion_status(&script, path))
        .transpose()?;
    let install_recommended = status
        .as_ref()
        .map(|status| !matches!(status, CompletionStatus::UpToDate));
    let suggested_install_command = install_recommended
        .filter(|recommended| *recommended)
        .map(|_| format!("{PROGRAM_NAME} completions --install {shell_name}"));

    let output = CompletionJsonOutput {
        action: "status",
        shell: shell_name,
        install_supported: install_path.is_some(),
        install_path: install_path_string,
        status: status.as_ref().map(completion_status_label),
        result: None,
        install_recommended,
        suggested_install_command,
        script: None,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn run_completions_wizard(
    shell: CompletionShell,
    install: bool,
    yes: bool,
    no_input: bool,
) -> Result<()> {
    println!("{}", Colors::bold("Shell completions"));
    println!("Detected shell: {}", shell_label(shell));
    println!();

    print_install_guidance(shell);

    let Some(install_path) = default_completion_path(shell) else {
        println!(
            "{} Automatic install isn't supported for this shell.",
            Colors::warning("Note:")
        );
        return Ok(());
    };

    if matches!(shell, CompletionShell::Bash | CompletionShell::Zsh) {
        println!(
            "{} install a static completion file (not required if you use the line above).",
            Colors::dim("Optional:")
        );
    }

    let script = generate_completions_bytes(shell)?;
    let status = completion_status(&script, &install_path)?;
    match status {
        CompletionStatus::UpToDate => {
            println!(
                "{} Completions are up-to-date at {}",
                Colors::success("✓"),
                install_path.display()
            );
        }
        CompletionStatus::OutOfDate => {
            println!(
                "{} Completions are out of date at {}",
                Colors::warning("⚠"),
                install_path.display()
            );
        }
        CompletionStatus::Missing => {
            println!(
                "{} No completion file found at {}",
                Colors::warning("⚠"),
                install_path.display()
            );
        }
    }

    if install {
        let outcome = install_completions(&script, &install_path)?;
        print_install_outcome(outcome);
        print_static_install_note(shell);
        return Ok(());
    }

    if matches!(
        status,
        CompletionStatus::OutOfDate | CompletionStatus::Missing
    ) {
        let stdin_tty = io::stdin().is_terminal();
        if yes
            || (!no_input && stdin_tty && prompt_yes_no("Install/update completions now?", true)?)
        {
            let outcome = install_completions(&script, &install_path)?;
            print_install_outcome(outcome);
            print_static_install_note(shell);
        } else {
            println!(
                "Run: {} completions --install {}",
                PROGRAM_NAME,
                shell_label(shell)
            );
        }
    }

    Ok(())
}

fn supported_shells() -> &'static str {
    "bash, zsh, fish, elvish"
}

fn print_shell_detection_help() {
    println!("{}", Colors::warning("Shell not detected."));
    println!("Run: {PROGRAM_NAME} completions <shell>");
    println!("Supported shells: {}", supported_shells());
}

fn print_install_guidance(shell: CompletionShell) {
    println!("{}", Colors::bold("Recommended setup:"));
    match shell {
        CompletionShell::Bash => {
            println!("Add this to ~/.bashrc:");
            println!("  source <(agent-tui completions bash --print)");
            println!(
                "{}",
                Colors::dim("This keeps completions in sync with your installed agent-tui.")
            );
        }
        CompletionShell::Zsh => {
            println!("Add this to ~/.zshrc:");
            println!("  source <(agent-tui completions zsh --print)");
            println!(
                "{}",
                Colors::dim("This keeps completions in sync with your installed agent-tui.")
            );
        }
        CompletionShell::Fish => {
            println!("Install a completion file (fish loads it automatically):");
            println!(
                "  agent-tui completions fish --print > ~/.config/fish/completions/agent-tui.fish"
            );
            println!(
                "{}",
                Colors::dim("Re-run this after upgrading agent-tui to refresh the file.")
            );
        }
        CompletionShell::Elvish => {
            println!("Install a completion file:");
            println!("  agent-tui completions elvish --print > ~/.elvish/lib/agent-tui.elv");
            println!(
                "{}",
                Colors::dim("Re-run this after upgrading agent-tui to refresh the file.")
            );
        }
    }
    println!();
}

fn resolve_shell(shell: Option<CompletionShell>) -> Option<CompletionShell> {
    shell.or_else(detect_shell_from_env)
}

fn detect_shell_from_env() -> Option<CompletionShell> {
    let env_shell = std::env::var("SHELL").ok()?;
    let name = std::path::Path::new(&env_shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&env_shell)
        .to_ascii_lowercase();
    shell_from_name(&name)
}

fn shell_from_name(name: &str) -> Option<CompletionShell> {
    if name.contains("bash") {
        Some(CompletionShell::Bash)
    } else if name.contains("zsh") {
        Some(CompletionShell::Zsh)
    } else if name.contains("fish") {
        Some(CompletionShell::Fish)
    } else if name.contains("elvish") {
        Some(CompletionShell::Elvish)
    } else {
        None
    }
}

fn shell_label(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => "bash",
        CompletionShell::Zsh => "zsh",
        CompletionShell::Fish => "fish",
        CompletionShell::Elvish => "elvish",
    }
}

fn default_completion_path(shell: CompletionShell) -> Option<PathBuf> {
    let home = home_dir()?;
    match shell {
        CompletionShell::Bash => Some(home.join(".bash_completion.d").join("agent-tui")),
        CompletionShell::Zsh => Some(home.join(".zsh").join("completions").join("_agent-tui")),
        CompletionShell::Fish => Some(
            home.join(".config")
                .join("fish")
                .join("completions")
                .join("agent-tui.fish"),
        ),
        CompletionShell::Elvish => Some(home.join(".elvish").join("lib").join("agent-tui.elv")),
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn generate_completions_bytes(shell: CompletionShell) -> Result<Vec<u8>> {
    let mut cmd = Cli::command();
    let mut out = Vec::new();
    generate(shell.clap_shell(), &mut cmd, PROGRAM_NAME, &mut out);
    Ok(out)
}

fn completion_status(expected: &[u8], path: &PathBuf) -> Result<CompletionStatus> {
    match fs::read(path) {
        Ok(existing) => {
            if existing == expected {
                Ok(CompletionStatus::UpToDate)
            } else {
                Ok(CompletionStatus::OutOfDate)
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(CompletionStatus::Missing),
        Err(err) => {
            Err(err).with_context(|| format!("failed to read completion file {}", path.display()))
        }
    }
}

fn install_completions(script: &[u8], path: &PathBuf) -> Result<InstallOutcome> {
    let status = completion_status(script, path)?;
    let path = path.clone();
    if matches!(status, CompletionStatus::UpToDate) {
        return Ok(InstallOutcome::AlreadyUpToDate(path));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create completion directory {}", parent.display())
        })?;
    }
    fs::write(&path, script)
        .with_context(|| format!("failed to write completions to {}", path.display()))?;
    Ok(match status {
        CompletionStatus::Missing => InstallOutcome::Installed(path),
        CompletionStatus::OutOfDate => InstallOutcome::Updated(path),
        CompletionStatus::UpToDate => InstallOutcome::AlreadyUpToDate(path),
    })
}

fn completion_status_label(status: &CompletionStatus) -> &'static str {
    match status {
        CompletionStatus::Missing => "missing",
        CompletionStatus::UpToDate => "up_to_date",
        CompletionStatus::OutOfDate => "out_of_date",
    }
}

fn install_outcome_label(outcome: &InstallOutcome) -> &'static str {
    match outcome {
        InstallOutcome::Installed(_) => "installed",
        InstallOutcome::Updated(_) => "updated",
        InstallOutcome::AlreadyUpToDate(_) => "already_up_to_date",
    }
}

fn print_install_outcome(outcome: InstallOutcome) {
    match outcome {
        InstallOutcome::Installed(path) => {
            println!(
                "{} Installed completions to {}",
                Colors::success("✓"),
                path.display()
            );
        }
        InstallOutcome::Updated(path) => {
            println!(
                "{} Updated completions at {}",
                Colors::success("✓"),
                path.display()
            );
        }
        InstallOutcome::AlreadyUpToDate(path) => {
            println!(
                "{} Completions already up-to-date at {}",
                Colors::success("✓"),
                path.display()
            );
        }
    }
}

fn print_static_install_note(shell: CompletionShell) {
    match shell {
        CompletionShell::Bash => println!(
            "{}",
            Colors::dim(
                "Note: ensure your shell loads ~/.bash_completion.d (or source the file in ~/.bashrc)."
            )
        ),
        CompletionShell::Zsh => println!(
            "{}",
            Colors::dim("Note: ensure ~/.zsh/completions is in $fpath and compinit is enabled.")
        ),
        _ => {}
    }
}

pub(crate) fn prompt_yes_no(prompt: &str, default_yes: bool) -> io::Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let mut input = String::new();
    loop {
        print!("{prompt} {suffix} ");
        io::stdout().flush()?;
        input.clear();
        if io::stdin().read_line(&mut input)? == 0 {
            return Ok(default_yes);
        }
        let answer = input.trim().to_ascii_lowercase();
        if answer.is_empty() {
            return Ok(default_yes);
        }
        if matches!(answer.as_str(), "y" | "yes") {
            return Ok(true);
        }
        if matches!(answer.as_str(), "n" | "no") {
            return Ok(false);
        }
        println!("Please answer y or n.");
    }
}

pub struct Application;

enum ParsedCli {
    Ready(Cli),
    Exit(i32),
}

impl Application {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> Result<i32> {
        let cli = match self.parse_cli() {
            ParsedCli::Ready(cli) => cli,
            ParsedCli::Exit(code) => return Ok(code),
        };

        let exit_code = match self.execute_with_cli(cli) {
            Ok(()) => exit_codes::SUCCESS,
            Err(e) => self.handle_error(e),
        };
        Ok(exit_code)
    }

    fn parse_cli(&self) -> ParsedCli {
        let raw_args: Vec<OsString> = std::env::args_os().collect();

        match Cli::try_parse_from(raw_args.clone()) {
            Ok(cli) => ParsedCli::Ready(cli),
            Err(err) => ParsedCli::Exit(render_parse_error(err, &raw_args)),
        }
    }

    fn execute_with_cli(&self, cli: Cli) -> Result<()> {
        let _telemetry = telemetry::init_tracing("warn");
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
            .map_err(|e| self.wrap_error(e, format))
            .context("failed to handle standalone command")?
        {
            return Ok(());
        }

        let mut client: UnixSocketClient = if Self::requires_daemon_autostart(&cli.command) {
            self.connect_to_daemon_autostart()
                .map_err(|e| self.wrap_error(e, format))
                .context("failed to connect to daemon with autostart")?
        } else {
            self.connect_to_daemon_no_autostart()
                .map_err(|e| self.wrap_error(e, format))
                .context("failed to connect to daemon")?
        };

        let mut ctx = HandlerContext::new(&mut client, cli.session, format, cli.no_input);
        self.dispatch_command(&mut ctx, cli.command)
            .map_err(|e| self.wrap_error(e, format))
            .context("failed to execute command")
    }

    fn handle_standalone_commands(&self, cli: &Cli) -> Result<bool> {
        match &cli.command {
            Commands::Daemon(DaemonCommand::Start {}) => {
                if daemon_start_requests_foreground() {
                    crate::app::daemon::start_daemon()?;
                } else {
                    handlers::handle_daemon_start_standalone(cli.effective_format())?;
                }
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Run) => {
                crate::app::daemon::start_daemon()?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Status) => {
                handlers::handle_daemon_status_standalone(cli.effective_format())?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Stop {
                force,
                dry_run,
                yes,
            }) => {
                handlers::handle_daemon_stop_standalone(
                    cli.effective_format(),
                    *force,
                    *dry_run,
                    *yes,
                    cli.no_input,
                )?;
                Ok(true)
            }
            Commands::Daemon(DaemonCommand::Restart { dry_run, yes }) => {
                handlers::handle_daemon_restart_standalone(
                    cli.effective_format(),
                    *dry_run,
                    *yes,
                    cli.no_input,
                )?;
                Ok(true)
            }
            Commands::Live { command: None } => {
                handlers::handle_live_start_standalone(
                    cli.effective_format(),
                    LiveStartArgs::default(),
                )?;
                Ok(true)
            }
            Commands::Live {
                command: Some(LiveCommand::Start(args)),
            } => {
                handlers::handle_live_start_standalone(cli.effective_format(), args.clone())?;
                Ok(true)
            }
            Commands::Live {
                command: Some(LiveCommand::Stop),
            } => {
                handlers::handle_live_stop_standalone(cli.effective_format())?;
                Ok(true)
            }
            Commands::Live {
                command: Some(LiveCommand::Status),
            } => {
                handlers::handle_live_status_standalone(cli.effective_format())?;
                Ok(true)
            }
            Commands::Completions {
                shell,
                print,
                install,
                yes,
            } => {
                handle_completions_command(
                    cli.effective_format(),
                    *shell,
                    *print,
                    *install,
                    *yes,
                    cli.no_input,
                )?;
                Ok(true)
            }
            Commands::Version => {
                handlers::handle_version_standalone(cli.effective_format())?;
                Ok(true)
            }
            Commands::Env => {
                handlers::handle_env(cli.effective_format())?;
                Ok(true)
            }
            Commands::Action { form } => {
                match legacy_action_compatibility_result(cli.effective_format(), form) {
                    Ok(_) => Ok(false),
                    Err(error) => {
                        warn_legacy_action_deprecation();
                        Err(error)
                    }
                }
            }
            Commands::ScrollIntoView { form } => {
                warn_legacy_scroll_into_view_deprecation();
                let selector =
                    parse_legacy_scroll_into_view_invocation(cli.effective_format(), form)?;
                handle_legacy_scroll_into_view(cli.effective_format(), &selector)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn requires_daemon_autostart(command: &Commands) -> bool {
        matches!(command, Commands::Run { .. })
    }

    fn connect_to_daemon_autostart(&self) -> Result<UnixSocketClient> {
        ensure_daemon().map_err(Into::into)
    }

    fn connect_to_daemon_no_autostart(&self) -> Result<UnixSocketClient> {
        match UnixSocketClient::connect() {
            Ok(client) => Ok(client),
            Err(ClientError::DaemonNotRunning) => Err(ClientError::DaemonNotRunning.into()),
            Err(e) => Err(e.into()),
        }
    }

    fn dispatch_command<C: DaemonClient>(
        &self,
        ctx: &mut HandlerContext<C>,
        command: Commands,
    ) -> Result<()> {
        match command {
            Commands::Daemon(daemon_cmd) => match daemon_cmd {
                DaemonCommand::Start { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Run => unreachable!("Handled in standalone"),
                DaemonCommand::Status => unreachable!("Handled in standalone"),
                DaemonCommand::Stop { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Restart { .. } => unreachable!("Handled in standalone"),
            },
            Commands::Completions { .. } => unreachable!("Handled in standalone"),

            Commands::Run {
                command,
                args,
                cwd,
                env,
                cols,
                rows,
            } => handlers::handle_spawn(
                ctx,
                command,
                args,
                cwd,
                env_assignments_to_map(env),
                cols,
                rows,
            )?,

            Commands::Screenshot {
                region,
                strip_ansi,
                retain_ansi,
                include_cursor,
                legacy_element,
                legacy_accessibility,
                legacy_interactive_only,
            } => {
                if legacy_element {
                    handlers::warn_legacy_deprecation("screenshot -e", "screenshot");
                }
                if legacy_accessibility {
                    handlers::warn_legacy_deprecation("screenshot -a", "screenshot");
                }
                if legacy_interactive_only {
                    handlers::warn_legacy_deprecation(
                        "screenshot --interactive-only",
                        "screenshot",
                    );
                }
                handlers::handle_snapshot(ctx, region, strip_ansi, retain_ansi, include_cursor)?
            }
            Commands::Action { form } => {
                warn_legacy_action_deprecation();
                let action = legacy_action_compatibility_result(ctx.format, &form)?;
                match action.operation {
                    LegacyActionOperation::Click => {
                        handlers::handle_press(ctx, "Enter".to_string())?
                    }
                    LegacyActionOperation::Fill(text) => handlers::handle_type(ctx, text)?,
                }
            }

            Commands::Resize { cols, rows } => handlers::handle_resize(ctx, cols, rows)?,
            Commands::Restart { dry_run, yes } => handlers::handle_restart(ctx, dry_run, yes)?,

            Commands::Press {
                keys,
                hold,
                release,
            } => {
                const PRESS_INTER_KEY_DELAY_MS: u64 = 50;
                if hold {
                    let key = single_modifier_key(ctx.format, "--hold", &keys)?;
                    handlers::handle_keydown(ctx, key)?
                } else if release {
                    let key = single_modifier_key(ctx.format, "--release", &keys)?;
                    handlers::handle_keyup(ctx, key)?
                } else {
                    let key_count = keys.len();
                    for (idx, key) in keys.into_iter().enumerate() {
                        handlers::handle_press(ctx, key)?;
                        if idx + 1 < key_count {
                            std::thread::park_timeout(std::time::Duration::from_millis(
                                PRESS_INTER_KEY_DELAY_MS,
                            ));
                        }
                    }
                }
            }

            Commands::Type { text } => handlers::handle_type(ctx, text)?,
            Commands::Input { text } => {
                handlers::warn_legacy_deprecation("input", "type");
                handlers::handle_type(ctx, text)?
            }
            Commands::Scroll { direction, amount } => {
                handlers::handle_scroll(ctx, direction, amount)?
            }
            Commands::ScrollIntoView { .. } => unreachable!("Handled in standalone"),

            Commands::Wait { params } => {
                if params.legacy_element.is_some() {
                    handlers::warn_legacy_deprecation("wait -e", "wait <text>");
                }
                handlers::handle_wait(ctx, params)?
            }
            Commands::Kill { dry_run, yes } => handlers::handle_kill(ctx, dry_run, yes)?,

            Commands::Sessions { command } => {
                use crate::app::commands::SessionsCommand;

                match command {
                    None | Some(SessionsCommand::List) => handlers::handle_sessions(ctx)?,
                    Some(SessionsCommand::Show { session_id }) => {
                        handlers::handle_session_show(ctx, session_id)?
                    }
                    Some(SessionsCommand::Attach {
                        no_tty,
                        detach_keys,
                    }) => {
                        let attach_id = handlers::resolve_attach_session_id(ctx)?;
                        handlers::handle_attach(
                            ctx,
                            attach_id,
                            !no_tty && !ctx.no_input,
                            detach_keys,
                        )?
                    }
                    Some(SessionsCommand::Switch { session_id }) => {
                        handlers::handle_session_switch(ctx, session_id)?
                    }
                    Some(SessionsCommand::Cleanup { all, dry_run, yes }) => {
                        handlers::handle_cleanup(ctx, all, dry_run, yes)?
                    }
                }
            }

            Commands::Live { command } => match command {
                None => handlers::handle_live_start(ctx, LiveStartArgs::default())?,
                Some(LiveCommand::Start(args)) => handlers::handle_live_start(ctx, args)?,
                Some(LiveCommand::Stop) => handlers::handle_live_stop(ctx)?,
                Some(LiveCommand::Status) => handlers::handle_live_status(ctx)?,
            },

            Commands::Version => unreachable!("Handled in standalone"),
            Commands::Env => handlers::handle_env(ctx.format)?,
        }
        Ok(())
    }

    fn handle_error(&self, e: anyhow::Error) -> i32 {
        if find_error::<DaemonNotRunningError>(&e).is_some() {
            return exit_codes::NOT_RUNNING;
        }

        if let Some(cli_error) = find_error::<crate::app::error::CliError>(&e) {
            print_cli_error(cli_error);
            return cli_error.exit_code;
        }

        if let Some(client_error) = find_error::<ClientError>(&e) {
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
        } else if let Some(attach_error) = find_error::<AttachError>(&e) {
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
        } else if let Some(daemon_error) = find_error::<DaemonError>(&e) {
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

fn warn_legacy_action_deprecation() {
    handlers::warn_legacy_deprecation_with_replacement(
        "action",
        "`agent-tui press`, `agent-tui type`, or `agent-tui scroll`",
    );
}

fn warn_legacy_scroll_into_view_deprecation() {
    handlers::warn_legacy_deprecation_with_replacement(
        "scroll-into-view",
        "`agent-tui scroll` or `agent-tui press`",
    );
}

impl Application {
    fn wrap_error(&self, error: anyhow::Error, format: OutputFormat) -> anyhow::Error {
        if find_error::<DaemonNotRunningError>(&error).is_some() {
            return error;
        }
        if find_error::<crate::app::error::CliError>(&error).is_some() {
            return error;
        }
        if format != OutputFormat::Json {
            return error;
        }

        if let Some(client_error) = find_error::<ClientError>(&error) {
            return anyhow::Error::new(crate::app::error::CliError::new(
                format,
                client_error.to_string(),
                Some(client_error.to_json_string()),
                exit_code_for_client_error(client_error),
            ));
        }
        if let Some(attach_error) = find_error::<AttachError>(&error) {
            return anyhow::Error::new(crate::app::error::CliError::new(
                format,
                attach_error.to_string(),
                Some(serde_json::to_string_pretty(&attach_error.to_payload()).unwrap_or_default()),
                attach_error.exit_code(),
            ));
        }
        if let Some(daemon_error) = find_error::<DaemonError>(&error) {
            return anyhow::Error::new(crate::app::error::CliError::new(
                format,
                daemon_error.to_string(),
                None,
                exit_codes::IOERR,
            ));
        }

        anyhow::Error::new(crate::app::error::CliError::new(
            format,
            error.to_string(),
            None,
            exit_codes::GENERAL_ERROR,
        ))
    }
}

fn find_error<T: std::error::Error + 'static>(error: &anyhow::Error) -> Option<&T> {
    error.chain().find_map(|source| source.downcast_ref::<T>())
}

fn print_cli_error(error: &crate::app::error::CliError) {
    match error.format {
        OutputFormat::Json => {
            if let Some(json) = &error.json {
                eprintln!("{json}");
            } else {
                #[derive(Serialize)]
                struct ErrorOutput<'a> {
                    success: bool,
                    error: &'a str,
                }
                let output = ErrorOutput {
                    success: false,
                    error: &error.message,
                };
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_default()
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

fn render_parse_error(err: clap::Error, raw_args: &[OsString]) -> i32 {
    let kind = err.kind();
    let uses_stderr = !matches!(
        kind,
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    );
    let exit_code = err.exit_code();

    if uses_stderr {
        eprint!("{err}");
    } else {
        print!("{err}");
    }

    if uses_stderr && parse_error_needs_example(kind) {
        eprintln!("Example:");
        eprintln!("  {}", parse_error_example(raw_args));
    }

    exit_code
}

fn parse_error_needs_example(kind: clap::error::ErrorKind) -> bool {
    matches!(
        kind,
        clap::error::ErrorKind::ArgumentConflict
            | clap::error::ErrorKind::InvalidSubcommand
            | clap::error::ErrorKind::InvalidValue
            | clap::error::ErrorKind::MissingRequiredArgument
            | clap::error::ErrorKind::MissingSubcommand
            | clap::error::ErrorKind::NoEquals
            | clap::error::ErrorKind::TooFewValues
            | clap::error::ErrorKind::TooManyValues
            | clap::error::ErrorKind::UnknownArgument
            | clap::error::ErrorKind::ValueValidation
            | clap::error::ErrorKind::WrongNumberOfValues
    )
}

fn parse_error_example(raw_args: &[OsString]) -> String {
    let command_path = parse_error_command_path(raw_args);
    if command_path.is_empty() {
        format!("{PROGRAM_NAME} --help")
    } else {
        format!("{PROGRAM_NAME} {} --help", command_path.join(" "))
    }
}

fn parse_error_command_path(raw_args: &[OsString]) -> Vec<String> {
    let mut command = Cli::command();
    let mut path = Vec::new();

    for arg in raw_args.iter().skip(1) {
        let token = arg.to_string_lossy();
        if token.is_empty() || token == "--" || token.starts_with('-') {
            break;
        }

        let Some(subcommand) = command.find_subcommand(token.as_ref()) else {
            break;
        };

        path.push(token.into_owned());
        command = subcommand.clone();
    }

    path
}

fn exit_code_for_client_error(error: &ClientError) -> i32 {
    use crate::common::error_codes::ErrorCategory;

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
#[path = "mod_tests.rs"]
mod tests;
