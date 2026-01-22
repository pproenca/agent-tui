use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use clap_complete::Shell;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(
    name = "agent-tui",
    version,
    about = "CLI tool for AI agents to interact with TUI applications"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, help = "Target session ID")]
    pub session: Option<String>,

    #[arg(long, global = true, value_enum, help = "Output format")]
    pub format: Option<OutputFormat>,

    #[arg(
        long,
        global = true,
        help = "Output in JSON format (shorthand for --format json)"
    )]
    pub json: bool,

    #[arg(long, global = true, help = "Disable colored output")]
    pub no_color: bool,
}

impl Cli {
    pub fn effective_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            self.format.unwrap_or_default()
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start the daemon in foreground")]
    Daemon,

    #[command(about = "Check daemon health")]
    Health {
        #[arg(short, long, help = "Show detailed health information")]
        verbose: bool,
    },

    #[command(about = "List active sessions")]
    Sessions,

    #[command(about = "Show version information")]
    Version,

    #[command(about = "Show environment configuration")]
    Env,

    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}
