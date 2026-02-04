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
use crate::app::daemon::start_daemon;
use crate::app::rpc_client::call_no_params;
use crate::common::Colors;
use crate::common::DaemonError;
use crate::common::color_init;
use crate::common::telemetry;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;
use crate::infra::ipc::UnixSocketClient;
use crate::infra::ipc::ensure_daemon;
use tracing::debug;

use crate::adapters::presenter::create_presenter;
use crate::app::attach::AttachError;
use crate::app::commands::Cli;
use crate::app::commands::Commands;
use crate::app::commands::DaemonCommand;
use crate::app::commands::LiveCommand;
use crate::app::commands::LiveStartArgs;
use crate::app::commands::Shell;
use crate::app::error::DaemonNotRunningError;
use crate::app::handlers::HandlerContext;

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
    shell: Option<Shell>,
    print: bool,
    install: bool,
    yes: bool,
) -> Result<()> {
    if install {
        let shell = resolve_shell(shell).ok_or_else(|| {
            anyhow::anyhow!("Shell not specified. Use one of: {}", supported_shells())
        })?;
        run_completions_wizard(shell, true, yes)?;
        return Ok(());
    }

    let stdout_tty = io::stdout().is_terminal();
    if print || !stdout_tty {
        let shell = resolve_shell(shell).ok_or_else(|| {
            anyhow::anyhow!("Shell not specified. Use one of: {}", supported_shells())
        })?;
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, PROGRAM_NAME, &mut io::stdout());
        return Ok(());
    }

    let shell = match resolve_shell(shell) {
        Some(shell) => shell,
        None => {
            print_shell_detection_help();
            return Ok(());
        }
    };

    run_completions_wizard(shell, install, yes)?;
    Ok(())
}

fn run_completions_wizard(shell: Shell, install: bool, yes: bool) -> Result<()> {
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

    if matches!(shell, Shell::Bash | Shell::Zsh) {
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
        if yes || (stdin_tty && prompt_yes_no("Install/update completions now?", true)?) {
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
    "bash, zsh, fish, powershell, elvish"
}

fn print_shell_detection_help() {
    println!("{}", Colors::warning("Shell not detected."));
    println!("Run: {} completions <shell>", PROGRAM_NAME);
    println!("Supported shells: {}", supported_shells());
}

fn print_install_guidance(shell: Shell) {
    println!("{}", Colors::bold("Recommended setup:"));
    match shell {
        Shell::Bash => {
            println!("Add this to ~/.bashrc:");
            println!("  source <(agent-tui completions bash --print)");
            println!(
                "{}",
                Colors::dim("This keeps completions in sync with your installed agent-tui.")
            );
        }
        Shell::Zsh => {
            println!("Add this to ~/.zshrc:");
            println!("  source <(agent-tui completions zsh --print)");
            println!(
                "{}",
                Colors::dim("This keeps completions in sync with your installed agent-tui.")
            );
        }
        Shell::PowerShell => {
            println!("Add this to $PROFILE:");
            println!("  agent-tui completions powershell --print | Out-String | Invoke-Expression");
            println!(
                "{}",
                Colors::dim("This keeps completions in sync with your installed agent-tui.")
            );
        }
        Shell::Fish => {
            println!("Install a completion file (fish loads it automatically):");
            println!(
                "  agent-tui completions fish --print > ~/.config/fish/completions/agent-tui.fish"
            );
            println!(
                "{}",
                Colors::dim("Re-run this after upgrading agent-tui to refresh the file.")
            );
        }
        Shell::Elvish => {
            println!("Install a completion file:");
            println!("  agent-tui completions elvish --print > ~/.elvish/lib/agent-tui.elv");
            println!(
                "{}",
                Colors::dim("Re-run this after upgrading agent-tui to refresh the file.")
            );
        }
        _ => {
            println!("Run: {} completions <shell> --print", PROGRAM_NAME);
            println!("Known shells: {}", supported_shells());
        }
    }
    println!();
}

fn resolve_shell(shell: Option<Shell>) -> Option<Shell> {
    shell.or_else(detect_shell_from_env)
}

fn detect_shell_from_env() -> Option<Shell> {
    let env_shell = std::env::var("SHELL")
        .ok()
        .or_else(|| std::env::var("COMSPEC").ok())?;
    let name = std::path::Path::new(&env_shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&env_shell)
        .to_ascii_lowercase();
    shell_from_name(&name)
}

fn shell_from_name(name: &str) -> Option<Shell> {
    if name.contains("bash") {
        Some(Shell::Bash)
    } else if name.contains("zsh") {
        Some(Shell::Zsh)
    } else if name.contains("fish") {
        Some(Shell::Fish)
    } else if name.contains("pwsh") || name.contains("powershell") {
        Some(Shell::PowerShell)
    } else if name.contains("elvish") {
        Some(Shell::Elvish)
    } else {
        None
    }
}

fn shell_label(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::PowerShell => "powershell",
        Shell::Elvish => "elvish",
        _ => "unknown",
    }
}

fn default_completion_path(shell: Shell) -> Option<PathBuf> {
    let home = home_dir()?;
    match shell {
        Shell::Bash => Some(home.join(".bash_completion.d").join("agent-tui")),
        Shell::Zsh => Some(home.join(".zsh").join("completions").join("_agent-tui")),
        Shell::Fish => Some(
            home.join(".config")
                .join("fish")
                .join("completions")
                .join("agent-tui.fish"),
        ),
        Shell::Elvish => Some(home.join(".elvish").join("lib").join("agent-tui.elv")),
        Shell::PowerShell => None,
        _ => None,
    }
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Some(PathBuf::from(home));
    }
    match (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        (Ok(drive), Ok(path)) => Some(PathBuf::from(format!("{}{}", drive, path))),
        _ => None,
    }
}

fn generate_completions_bytes(shell: Shell) -> Result<Vec<u8>> {
    let mut cmd = Cli::command();
    let mut out = Vec::new();
    generate(shell, &mut cmd, PROGRAM_NAME, &mut out);
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
    if matches!(status, CompletionStatus::UpToDate) {
        return Ok(InstallOutcome::AlreadyUpToDate(path.clone()));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create completion directory {}", parent.display())
        })?;
    }
    fs::write(path, script)
        .with_context(|| format!("failed to write completions to {}", path.display()))?;
    Ok(match status {
        CompletionStatus::Missing => InstallOutcome::Installed(path.clone()),
        CompletionStatus::OutOfDate => InstallOutcome::Updated(path.clone()),
        CompletionStatus::UpToDate => InstallOutcome::AlreadyUpToDate(path.clone()),
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

fn print_static_install_note(shell: Shell) {
    match shell {
        Shell::Bash => println!(
            "{}",
            Colors::dim(
                "Note: ensure your shell loads ~/.bash_completion.d (or source the file in ~/.bashrc)."
            )
        ),
        Shell::Zsh => println!(
            "{}",
            Colors::dim("Note: ensure ~/.zsh/completions is in $fpath and compinit is enabled.")
        ),
        _ => {}
    }
}

fn prompt_yes_no(prompt: &str, default_yes: bool) -> io::Result<bool> {
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
            .map_err(|e| self.wrap_error(e, format))
            .context("failed to handle standalone command")?
        {
            return Ok(());
        }

        let mut client: UnixSocketClient = match &cli.command {
            Commands::Run { .. } | Commands::Live { .. } => self
                .connect_to_daemon_autostart()
                .map_err(|e| self.wrap_error(e, format))
                .context("failed to connect to daemon with autostart")?,
            _ => self
                .connect_to_daemon_no_autostart()
                .map_err(|e| self.wrap_error(e, format))
                .context("failed to connect to daemon")?,
        };

        if !matches!(cli.command, Commands::Daemon(_) | Commands::Version) {
            check_version_mismatch(&mut client);
        }

        let mut ctx = HandlerContext::new(&mut client, cli.session, format);
        self.dispatch_command(&mut ctx, &cli.command, cli.verbose)
            .map_err(|e| self.wrap_error(e, format))
            .with_context(|| format!("failed to execute command {:?}", cli.command))
    }

    fn handle_standalone_commands(&self, cli: &Cli) -> Result<bool> {
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
            Commands::Daemon(DaemonCommand::Restart) => {
                self.handle_daemon_restart_without_autostart(cli)?;
                Ok(true)
            }
            Commands::Completions {
                shell,
                print,
                install,
                yes,
            } => {
                handle_completions_command(*shell, *print, *install, *yes)?;
                Ok(true)
            }
            Commands::Env => {
                handlers::handle_env(cli.effective_format())?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_daemon_status_without_autostart(&self, cli: &Cli) -> Result<()> {
        match UnixSocketClient::connect() {
            Ok(mut client) => {
                // Verify daemon is actually responding before showing status
                match call_no_params(&mut client, "health") {
                    Ok(result) => {
                        let format = cli.effective_format();
                        handlers::print_daemon_status_from_result(&result, format);
                        Ok(())
                    }
                    Err(_) => {
                        // Connected but daemon not responding - treat as not running
                        self.print_daemon_not_running_status(cli);
                        Err(anyhow::Error::new(DaemonNotRunningError))
                    }
                }
            }
            Err(ClientError::DaemonNotRunning) => {
                self.print_daemon_not_running_status(cli);
                Err(anyhow::Error::new(DaemonNotRunningError))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn print_daemon_not_running_status(&self, cli: &Cli) {
        let cli_version = env!("AGENT_TUI_VERSION");
        let cli_commit = env!("AGENT_TUI_GIT_SHA");
        match cli.effective_format() {
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct DaemonNotRunningOutput {
                    running: bool,
                    cli_version: &'static str,
                    cli_commit: &'static str,
                }
                let output = DaemonNotRunningOutput {
                    running: false,
                    cli_version,
                    cli_commit,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).unwrap_or_default()
                );
            }
            _ => {
                println!("Daemon is not running");
                println!("  CLI version: {}", cli_version);
                println!("  CLI commit: {}", cli_commit);
            }
        }
    }

    fn handle_daemon_stop_without_autostart(&self, force: bool) -> Result<()> {
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

    fn handle_daemon_restart_without_autostart(&self, cli: &Cli) -> Result<()> {
        let format = cli.effective_format();
        let presenter = create_presenter(&format);

        if let OutputFormat::Text = format {
            presenter.present_info("Restarting daemon...");
        }

        let warnings = handlers::restart_daemon_core()?;
        for warning in &warnings {
            eprintln!("{}", Colors::warning(warning));
        }
        presenter.present_success("Daemon restarted", None);
        Ok(())
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
        command: &Commands,
        verbose: bool,
    ) -> Result<()> {
        match command {
            Commands::Daemon(daemon_cmd) => match daemon_cmd {
                DaemonCommand::Start { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Stop { .. } => unreachable!("Handled in standalone"),
                DaemonCommand::Status => unreachable!("Handled in standalone"),
                DaemonCommand::Restart => unreachable!("Handled in standalone"),
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
                region,
                strip_ansi,
                include_cursor,
            } => handlers::handle_snapshot(ctx, region.clone(), *strip_ansi, *include_cursor)?,

            Commands::Resize { cols, rows } => handlers::handle_resize(ctx, *cols, *rows)?,
            Commands::Restart => handlers::handle_restart(ctx)?,

            Commands::Press {
                keys,
                hold,
                release,
            } => {
                const PRESS_INTER_KEY_DELAY_MS: u64 = 50;
                if *hold {
                    if keys.len() != 1 {
                        return Err(anyhow::Error::new(crate::app::error::CliError::new(
                            ctx.format,
                            "Press --hold requires exactly one key (Ctrl, Alt, Shift, Meta)",
                            None,
                            exit_codes::USAGE,
                        )));
                    }
                    handlers::handle_keydown(ctx, keys[0].clone())?
                } else if *release {
                    if keys.len() != 1 {
                        return Err(anyhow::Error::new(crate::app::error::CliError::new(
                            ctx.format,
                            "Press --release requires exactly one key (Ctrl, Alt, Shift, Meta)",
                            None,
                            exit_codes::USAGE,
                        )));
                    }
                    handlers::handle_keyup(ctx, keys[0].clone())?
                } else {
                    for (idx, key) in keys.iter().enumerate() {
                        handlers::handle_press(ctx, key.to_string())?;
                        if idx + 1 < keys.len() {
                            std::thread::sleep(std::time::Duration::from_millis(
                                PRESS_INTER_KEY_DELAY_MS,
                            ));
                        }
                    }
                }
            }

            Commands::Type { text } => handlers::handle_type(ctx, text.to_string())?,

            Commands::Scroll { direction, amount } => {
                handlers::handle_scroll(ctx, *direction, *amount)?
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
                        no_tty,
                        detach_keys,
                    }) => {
                        let attach_id = handlers::resolve_attach_session_id(ctx)?;
                        handlers::handle_attach(ctx, attach_id, !*no_tty, detach_keys.clone())?
                    }
                    Some(SessionsCommand::Switch { session_id }) => {
                        handlers::handle_session_switch(ctx, session_id.clone())?
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
            Commands::Env => handlers::handle_env(ctx.format)?,
        }
        Ok(())
    }

    fn handle_error(&self, e: anyhow::Error) -> i32 {
        // Handle DaemonNotRunningError specially - no error message printed,
        // output was already shown by the handler, just return LSB exit code 3
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

fn check_version_mismatch<C: DaemonClient>(client: &mut C) {
    use crate::infra::ipc::version::VersionCheckResult;
    use crate::infra::ipc::version::check_version;

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
        use crate::app::commands::OutputFormat;
        use std::env;
        use tempfile::TempDir;

        #[test]
        fn handle_standalone_commands_routes_daemon_status() {
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            // SAFETY: Test-only environment override to isolate the daemon socket.
            unsafe {
                env::set_var("AGENT_TUI_SOCKET", &socket_path);
            }

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
            // SAFETY: Test-only environment override to isolate the daemon socket.
            unsafe {
                env::set_var("AGENT_TUI_SOCKET", &socket_path);
            }

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
            // SAFETY: Test-only environment override to isolate the daemon socket.
            unsafe {
                env::set_var("AGENT_TUI_SOCKET", &socket_path);
            }

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
        fn handle_standalone_commands_routes_daemon_restart() {
            // Isolate from any real daemon by pointing socket to a temp path.
            let tmp = TempDir::new().expect("temp dir");
            let socket_path = tmp.path().join("agent-tui-test.sock");
            // SAFETY: Test-only environment override to isolate the daemon socket.
            unsafe {
                env::set_var("AGENT_TUI_SOCKET", &socket_path);
            }

            let app = Application::new();
            let cli = Cli {
                command: Commands::Daemon(DaemonCommand::Restart),
                session: None,
                format: OutputFormat::Text,
                json: false,
                no_color: true,
                verbose: false,
            };

            // Restart should be handled as standalone (may error if start fails)
            let result = app.handle_standalone_commands(&cli);
            if let Ok(handled) = result {
                assert!(handled, "daemon restart should be handled as standalone");
            }
        }

        #[test]
        fn handle_error_returns_not_running_exit_code() {
            let app = Application::new();
            let err = anyhow::Error::new(DaemonNotRunningError);
            let exit_code = app.handle_error(err);
            assert_eq!(exit_code, exit_codes::NOT_RUNNING);
        }
    }
}
