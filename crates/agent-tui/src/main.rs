use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;

use agent_tui::commands::Cli;
use agent_tui::commands::Commands;
use agent_tui::handlers::HandlerContext;
use agent_tui::handlers::{self};
use agent_tui_common::color_init;
use agent_tui_common::Colors;
use agent_tui_ipc::ensure_daemon;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", Colors::error("Error:"), e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    color_init(cli.no_color);

    if matches!(cli.command, Commands::Daemon) {
        return start_daemon().map_err(|e| e.into());
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
        Commands::Health { verbose } => handlers::handle_health(&mut ctx, verbose)?,
        Commands::Sessions => handlers::handle_sessions(&mut ctx)?,
        Commands::Version => handlers::handle_version(&mut ctx)?,
        Commands::Env => handlers::handle_env(&ctx)?,
        _ => {
            eprintln!("Command not yet migrated to workspace structure");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Daemon command not yet migrated to workspace structure");
    std::process::exit(1);
}
