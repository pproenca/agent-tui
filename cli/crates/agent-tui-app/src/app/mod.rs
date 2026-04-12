#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI application layer and composition root wiring.

use anyhow::Context;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use serde::Serialize;
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
use crate::app::commands::LiveCommand;
use crate::app::commands::LiveStartArgs;
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

fn handle_completions_command(
    format: OutputFormat,
    shell: Option<CompletionShell>,
    print: bool,
    install: bool,
    yes: bool,
    no_input: bool,
) -> Result<()> {
    if install {
        let shell = resolve_shell(shell).ok_or_else(|| {
            crate::app::error::CliError::new(
                format,
                format!(
                    "Shell not specified. Re-run with `agent-tui completions --install <bash|zsh|fish|elvish>`."
                ),
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
            crate::app::error::CliError::new(
                format,
                format!(
                    "Shell not specified. Re-run with `agent-tui completions --print <bash|zsh|fish|elvish>`."
                ),
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
            crate::app::error::CliError::new(
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
        if yes || (!no_input && stdin_tty && prompt_yes_no("Install/update completions now?", true)?) {
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
    println!("Run: {} completions <shell>", PROGRAM_NAME);
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
        print!("{} {} ", prompt, suffix);
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

impl Application {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> Result<i32> {
        let exit_code = match self.execute() {
            Ok(()) => exit_codes::SUCCESS,
            Err(e) => self.handle_error(e),
        };
        Ok(exit_code)
    }

    fn execute(&self) -> Result<()> {
        let cli = Cli::parse();
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
                cols,
                rows,
            } => handlers::handle_spawn(ctx, command, args, cwd, cols, rows)?,

            Commands::Screenshot {
                region,
                strip_ansi,
                retain_ansi,
                include_cursor,
            } => handlers::handle_snapshot(ctx, region, strip_ansi, retain_ansi, include_cursor)?,

            Commands::Resize { cols, rows } => handlers::handle_resize(ctx, cols, rows)?,
            Commands::Restart { dry_run, yes } => handlers::handle_restart(ctx, dry_run, yes)?,

            Commands::Press {
                mut keys,
                hold,
                release,
            } => {
                const PRESS_INTER_KEY_DELAY_MS: u64 = 50;
                if hold {
                    if keys.len() != 1 {
                        return Err(anyhow::Error::new(crate::app::error::CliError::new(
                            ctx.format,
                            "Press --hold requires exactly one key (Ctrl, Alt, Shift, Meta)",
                            None,
                            exit_codes::USAGE,
                        )));
                    }
                    let key = match keys.pop() {
                        Some(key) => key,
                        None => {
                            return Err(anyhow::Error::new(crate::app::error::CliError::new(
                                ctx.format,
                                "Press --hold requires exactly one key (Ctrl, Alt, Shift, Meta)",
                                None,
                                exit_codes::USAGE,
                            )));
                        }
                    };
                    handlers::handle_keydown(ctx, key)?
                } else if release {
                    if keys.len() != 1 {
                        return Err(anyhow::Error::new(crate::app::error::CliError::new(
                            ctx.format,
                            "Press --release requires exactly one key (Ctrl, Alt, Shift, Meta)",
                            None,
                            exit_codes::USAGE,
                        )));
                    }
                    let key = match keys.pop() {
                        Some(key) => key,
                        None => {
                            return Err(anyhow::Error::new(crate::app::error::CliError::new(
                                ctx.format,
                                "Press --release requires exactly one key (Ctrl, Alt, Shift, Meta)",
                                None,
                                exit_codes::USAGE,
                            )));
                        }
                    };
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
            Commands::Scroll { direction, amount } => {
                handlers::handle_scroll(ctx, direction, amount)?
            }

            Commands::Wait { params } => handlers::handle_wait(ctx, params)?,
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
                eprintln!("{}", json);
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
mod tests {
    use super::*;

    mod daemon_standalone_tests {
        use super::*;
        use crate::app::commands::Cli;
        use crate::app::commands::Commands;
        use crate::app::commands::DaemonCommand;
        use crate::app::commands::LiveCommand;
        use crate::app::commands::OutputFormat;
        use crate::test_support::env_lock;
        use std::env;
        use std::path::Path;
        use tempfile::TempDir;

        struct EnvVarGuard {
            key: &'static str,
            prev: Option<String>,
        }

        impl EnvVarGuard {
            fn set(key: &'static str, value: &str) -> Self {
                let prev = env::var(key).ok();
                // SAFETY: Test-only environment override.
                unsafe {
                    env::set_var(key, value);
                }
                Self { key, prev }
            }

            fn set_path(key: &'static str, value: &Path) -> Self {
                let prev = env::var(key).ok();
                // SAFETY: Test-only environment override.
                unsafe {
                    env::set_var(key, value);
                }
                Self { key, prev }
            }
        }

        impl Drop for EnvVarGuard {
            fn drop(&mut self) {
                if let Some(prev) = self.prev.take() {
                    // SAFETY: Test-only environment restoration.
                    unsafe {
                        env::set_var(self.key, prev);
                    }
                } else {
                    // SAFETY: Test-only environment cleanup.
                    unsafe {
                        env::remove_var(self.key);
                    }
                }
            }
        }

        fn make_cli(command: Commands) -> Cli {
            Cli {
                command,
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                no_input: false,
            }
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_stop() {
            let _env_lock = env_lock();
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
                force: false,
                dry_run: false,
                yes: true,
            }));

            // When daemon is not running, should succeed (idempotent semantics)
            // The result should be Ok(true), indicating the command was handled
            let result = app.handle_standalone_commands(&cli);
            assert!(
                result.is_ok(),
                "daemon stop should succeed when daemon not running (idempotent)"
            );
            assert!(
                matches!(result, Ok(true)),
                "daemon stop should be handled as standalone"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_start() {
            let _env_lock = env_lock();
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            // Use stub to prevent spawning a real daemon process.
            crate::infra::ipc::transport::USE_DAEMON_START_STUB
                .store(true, std::sync::atomic::Ordering::SeqCst);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Start {}));

            let result = app.handle_standalone_commands(&cli);
            // Error is acceptable (daemon may fail to start), but it was handled
            if let Ok(handled) = result {
                assert!(handled, "daemon start should be handled as standalone");
            }

            // Clean up stub state.
            crate::infra::ipc::transport::USE_DAEMON_START_STUB
                .store(false, std::sync::atomic::Ordering::SeqCst);
            crate::infra::ipc::transport::clear_test_listener();
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_restart() {
            let _env_lock = env_lock();
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            // Use stub to prevent spawning a real daemon process.
            crate::infra::ipc::transport::USE_DAEMON_START_STUB
                .store(true, std::sync::atomic::Ordering::SeqCst);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Restart {
                dry_run: false,
                yes: true,
            }));

            // Restart should be handled as standalone (may error if start fails)
            let result = app.handle_standalone_commands(&cli);
            if let Ok(handled) = result {
                assert!(handled, "daemon restart should be handled as standalone");
            }

            // Clean up stub state.
            crate::infra::ipc::transport::USE_DAEMON_START_STUB
                .store(false, std::sync::atomic::Ordering::SeqCst);
            crate::infra::ipc::transport::clear_test_listener();
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_status_without_autostart() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Status));

            let result = app.handle_standalone_commands(&cli);
            let err = result.expect_err("daemon status should report not running");
            assert!(
                err.downcast_ref::<DaemonNotRunningError>().is_some(),
                "daemon status should map to daemon-not-running exit handling"
            );
            assert!(
                !socket_path.exists(),
                "daemon status must not autostart daemon or create socket"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_daemon_status_locally_when_ws_transport_selected() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
            let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "ws");
            let _ws_addr_guard = EnvVarGuard::set("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws");

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Status));

            let result = app.handle_standalone_commands(&cli);
            let err = result.expect_err("daemon status should still inspect the local daemon");
            assert!(
                err.downcast_ref::<DaemonNotRunningError>().is_some(),
                "daemon status should ignore websocket transport selection"
            );
            assert!(
                !socket_path.exists(),
                "daemon status must not create a local socket when daemon is not running"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_live_status_without_autostart() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let ws_state = tmp.path().join("api.json");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
            let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

            let app = Application::new();
            let cli = make_cli(Commands::Live {
                command: Some(LiveCommand::Status),
            });

            let result = app.handle_standalone_commands(&cli);
            assert!(result.is_ok(), "live status should be handled");
            assert!(
                matches!(result, Ok(true)),
                "live status should be standalone"
            );
            assert!(
                !socket_path.exists(),
                "live status must not autostart daemon or create socket"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_live_start_locally_when_ws_transport_selected() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let ws_state = tmp.path().join("api.json");
            std::fs::write(
                &ws_state,
                format!(
                    r#"{{"pid":{},"ws_url":"ws://127.0.0.1:43210/ws","ui_url":"http://127.0.0.1:43210/ui","listen":"127.0.0.1:43210","started_at":1735689600}}"#,
                    std::process::id()
                ),
            )
            .expect("write ws state");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
            let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);
            let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "ws");
            let _ws_addr_guard = EnvVarGuard::set("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws");

            let app = Application::new();
            let cli = make_cli(Commands::Live { command: None });

            let result = app.handle_standalone_commands(&cli);
            assert!(result.is_ok(), "live start should be handled");
            assert!(
                matches!(result, Ok(true)),
                "live start should be standalone"
            );
            assert!(
                !socket_path.exists(),
                "live start should not use the selected remote websocket transport"
            );
        }

        #[test]
        fn handle_standalone_commands_routes_live_stop_without_autostart() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let ws_state = tmp.path().join("api.json");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
            let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

            let app = Application::new();
            let cli = make_cli(Commands::Live {
                command: Some(LiveCommand::Stop),
            });

            let result = app.handle_standalone_commands(&cli);
            assert!(result.is_ok(), "live stop should be handled");
            assert!(matches!(result, Ok(true)), "live stop should be standalone");
            assert!(
                !socket_path.exists(),
                "live stop must not autostart daemon or create socket"
            );
        }

        #[test]
        fn handle_standalone_commands_daemon_stop_stale_lock_is_idempotent() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let lock_path = socket_path.with_extension("lock");
            std::fs::write(&lock_path, "999999").expect("write stale lock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
                force: false,
                dry_run: false,
                yes: true,
            }));

            let result = app.handle_standalone_commands(&cli);
            assert!(
                matches!(result, Ok(true)),
                "daemon stop should be idempotent with stale lock"
            );
            assert!(
                !lock_path.exists(),
                "stale lock file should be cleaned after stop"
            );
        }

        #[test]
        fn handle_standalone_commands_daemon_force_stop_stale_lock_is_idempotent() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let lock_path = socket_path.with_extension("lock");
            std::fs::write(&lock_path, "999999").expect("write stale lock");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
                force: true,
                dry_run: false,
                yes: true,
            }));

            let result = app.handle_standalone_commands(&cli);
            assert!(
                matches!(result, Ok(true)),
                "daemon stop --force should be idempotent with stale lock"
            );
            assert!(
                !lock_path.exists(),
                "stale lock file should be cleaned after forced stop"
            );
        }

        #[test]
        fn handle_standalone_commands_daemon_stop_removes_stale_ws_state() {
            let _env_lock = env_lock();
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            let ws_state = tmp.path().join("api.json");
            std::fs::write(&ws_state, r#"{"pid":1}"#).expect("write ws state");
            let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
            let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

            let app = Application::new();
            let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
                force: true,
                dry_run: false,
                yes: true,
            }));

            let result = app.handle_standalone_commands(&cli);
            assert!(
                matches!(result, Ok(true)),
                "daemon stop should succeed when daemon is already stopped"
            );
            assert!(
                !ws_state.exists(),
                "WS state file should be cleaned on successful stop path"
            );
        }

        #[test]
        fn handle_error_returns_not_running_exit_code() {
            let app = Application::new();
            let exit_code = app.handle_error(anyhow::Error::new(DaemonNotRunningError));
            assert_eq!(exit_code, exit_codes::NOT_RUNNING);
        }

        #[test]
        fn daemon_start_requests_foreground_accepts_truthy_values() {
            let _env_lock = env_lock();
            let _foreground_guard = EnvVarGuard::set("AGENT_TUI_DAEMON_FOREGROUND", "true");
            assert!(daemon_start_requests_foreground());
        }
    }
}
