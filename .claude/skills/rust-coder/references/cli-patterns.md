# CLI Patterns

## Table of Contents
1. [Clap Derive Structure](#1-clap-derive-structure)
2. [Subcommand Flattening](#2-subcommand-flattening)
3. [Argument Validation](#3-argument-validation)
4. [Configuration Integration](#4-configuration-integration)
5. [Command Dispatch](#5-command-dispatch)
6. [Shell Completions](#6-shell-completions)

---

## 1. Clap Derive Structure

Define CLI with derive macros:

```rust
use clap::Parser;
use clap::Subcommand;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(
    name = "mytool",
    about = "System monitoring and analysis tool",
    version,
    author
)]
pub struct Cli {
    /// Path to configuration file
    #[clap(
        long,
        short = 'c',
        default_value = "/etc/mytool/config.toml",
        env = "MYTOOL_CONFIG"
    )]
    pub config: PathBuf,

    /// Enable verbose logging
    #[clap(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Output format
    #[clap(long, default_value = "human", value_parser = ["human", "json", "csv"])]
    pub format: String,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the data collection daemon
    Record(RecordArgs),

    /// View system data interactively
    Live(LiveArgs),

    /// Replay historical data
    Replay(ReplayArgs),

    /// Dump data to stdout
    Dump(DumpArgs),

    /// Debug utilities
    #[clap(subcommand)]
    Debug(DebugCommand),
}

#[derive(Debug, Args)]
pub struct RecordArgs {
    /// Directory to store data
    #[clap(long, default_value = "/var/log/mytool")]
    pub store_dir: PathBuf,

    /// Collection interval in seconds
    #[clap(long, default_value = "5")]
    pub interval: u64,

    /// Retain data for this many days
    #[clap(long, default_value = "7")]
    pub retain_days: u32,
}
```

## 2. Subcommand Flattening

Compose commands with flatten:

```rust
#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(flatten)]
    pub global: GlobalArgs,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// Log level
    #[clap(long, default_value = "info")]
    pub log_level: String,

    /// Disable colors
    #[clap(long)]
    pub no_color: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Dump system data
    Dump {
        #[clap(flatten)]
        common: DumpCommonArgs,

        #[clap(subcommand)]
        target: DumpTarget,
    },
}

#[derive(Debug, Args)]
pub struct DumpCommonArgs {
    /// Start time (e.g., "1h ago", "2024-01-15 10:00")
    #[clap(long, short = 's')]
    pub start: Option<String>,

    /// End time
    #[clap(long, short = 'e')]
    pub end: Option<String>,

    /// Output file (stdout if not specified)
    #[clap(long, short = 'o')]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum DumpTarget {
    /// Dump system-wide stats
    System {
        /// Fields to include
        #[clap(long, value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Dump process data
    Process {
        /// Filter by PID
        #[clap(long)]
        pid: Option<u32>,

        /// Filter by command name pattern
        #[clap(long)]
        comm: Option<String>,
    },

    /// Dump cgroup data
    Cgroup {
        /// Cgroup path filter
        #[clap(long)]
        path: Option<String>,
    },
}
```

## 3. Argument Validation

Custom validation with value_parser:

```rust
use clap::builder::TypedValueParser;

#[derive(Debug, Parser)]
pub struct Args {
    /// Time range (e.g., "5m", "1h", "2d")
    #[clap(long, value_parser = parse_duration)]
    pub duration: Duration,

    /// CPU threshold percentage (0-100)
    #[clap(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub cpu_threshold: Option<u8>,

    /// Memory limit (e.g., "1G", "512M")
    #[clap(long, value_parser = parse_bytes)]
    pub memory_limit: Option<u64>,

    /// Port number
    #[clap(long, value_parser = clap::value_parser!(u16).range(1024..))]
    pub port: u16,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let (num, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(s.len())
    );

    let num: u64 = num.parse()
        .map_err(|_| format!("Invalid number: {}", num))?;

    let secs = match unit {
        "s" | "" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        _ => return Err(format!("Unknown unit: {}", unit)),
    };

    Ok(Duration::from_secs(secs))
}

fn parse_bytes(s: &str) -> Result<u64, String> {
    let s = s.trim().to_uppercase();
    let (num, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(s.len())
    );

    let num: u64 = num.parse()
        .map_err(|_| format!("Invalid number: {}", num))?;

    let multiplier = match unit {
        "" | "B" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        "G" | "GB" => 1024 * 1024 * 1024,
        _ => return Err(format!("Unknown unit: {}", unit)),
    };

    Ok(num * multiplier)
}
```

## 4. Configuration Integration

Merge CLI args with config file:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub store_dir: Option<PathBuf>,
    pub interval: Option<u64>,
    pub retain_days: Option<u32>,
    pub log_level: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config: {}", path.display()))
    }
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub store_dir: PathBuf,
    pub interval: u64,
    pub retain_days: u32,
    pub log_level: String,
}

impl ResolvedConfig {
    /// Merge CLI args with config file, CLI takes precedence
    pub fn resolve(cli: &Cli, file_config: &Config) -> Self {
        Self {
            store_dir: cli.store_dir.clone()
                .or_else(|| file_config.store_dir.clone())
                .unwrap_or_else(|| PathBuf::from("/var/log/mytool")),

            interval: cli.interval
                .or(file_config.interval)
                .unwrap_or(5),

            retain_days: cli.retain_days
                .or(file_config.retain_days)
                .unwrap_or(7),

            log_level: cli.log_level.clone()
                .or_else(|| file_config.log_level.clone())
                .unwrap_or_else(|| "info".to_string()),
        }
    }
}
```

## 5. Command Dispatch

Route commands to handlers:

```rust
pub fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config
    let config = Config::load(&cli.config)?;

    // Setup logging
    let logger = setup_logger(cli.verbose)?;

    // Dispatch command
    match cli.command {
        Command::Record(args) => cmd_record(&logger, &config, args),
        Command::Live(args) => cmd_live(&logger, &config, args),
        Command::Replay(args) => cmd_replay(&logger, &config, args),
        Command::Dump(args) => cmd_dump(&logger, &config, args),
        Command::Debug(sub) => match sub {
            DebugCommand::Stats => cmd_debug_stats(&logger),
            DebugCommand::Check => cmd_debug_check(&logger),
        },
    }
}

fn cmd_record(logger: &Logger, config: &Config, args: RecordArgs) -> Result<()> {
    info!(logger, "Starting recorder";
        "store_dir" => args.store_dir.display().to_string(),
        "interval" => args.interval,
    );

    let collector = Collector::new(args.interval)?;
    let store = Store::open(&args.store_dir)?;

    loop {
        let sample = collector.collect()?;
        store.append(&sample)?;
        std::thread::sleep(Duration::from_secs(args.interval));
    }
}

fn cmd_dump(logger: &Logger, config: &Config, args: DumpArgs) -> Result<()> {
    let store = Store::open(&config.store_dir)?;

    let time_range = TimeRange::parse(args.common.start, args.common.end)?;
    let output: Box<dyn Write> = match args.common.output {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(std::io::stdout()),
    };

    match args.target {
        DumpTarget::System { fields } => {
            dump_system(&store, time_range, fields, output)
        }
        DumpTarget::Process { pid, comm } => {
            dump_process(&store, time_range, pid, comm, output)
        }
        DumpTarget::Cgroup { path } => {
            dump_cgroup(&store, time_range, path, output)
        }
    }
}
```

## 6. Shell Completions

Generate completion scripts:

```rust
use clap::CommandFactory;
use clap_complete::Shell;
use clap_complete::generate;

#[derive(Debug, Subcommand)]
pub enum Command {
    // ... other commands ...

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        #[clap(value_enum)]
        shell: Shell,
    },
}

fn cmd_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();

    generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

// Usage:
// mytool completions bash > /etc/bash_completion.d/mytool
// mytool completions zsh > ~/.zsh/completions/_mytool
// mytool completions fish > ~/.config/fish/completions/mytool.fish
```

---

## Related Patterns

- [Daemon Patterns](daemon-rpc-patterns.md) - CLI entry points for daemon services
- [Error Handling](error-handling.md) - Error display and exit codes
- [Architecture](architecture.md) - CLI vs library separation
- [Serialization](serialization.md) - Config file parsing
