//! CLI command parsing and configuration.

use clap::ArgGroup;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueHint;
pub use clap_complete::Shell;
use std::path::PathBuf;

pub use crate::adapters::presenter::OutputFormat;
use crate::app::attach::DetachKeys;

const AFTER_HELP: &str =
    "Use --help for full details and examples. Use --format json for machine-readable output.";

const LONG_ABOUT: &str = "\
Drive TUI (text UI) applications programmatically or interactively.\n\
\n\
Common flow: run -> screenshot -> press/type -> wait -> kill.\n\
Use --format json for automation-friendly output.";

const AFTER_LONG_HELP: &str = r#"WORKFLOW:
    1. Run a TUI application
    2. View the screenshot
    3. Interact with keys/text
    4. Wait for UI changes
    5. Kill the session when done

OUTPUT:
    --format json  Machine-readable JSON (recommended for automation)
    --format text  Human-readable text (default)

CONFIGURATION:
    AGENT_TUI_TRANSPORT         IPC transport (unix or ws; default: unix)
    AGENT_TUI_WS_ADDR           Remote WS-RPC target when transport is ws (e.g. ws://host:port/ws)
    AGENT_TUI_DETACH_KEYS       Detach keys for `sessions attach` (default: Ctrl-P Ctrl-Q)
    AGENT_TUI_DAEMON_FOREGROUND Run daemon start in foreground (internal)
    AGENT_TUI_WS_LISTEN         Daemon WS bind address (default: 127.0.0.1:0)
    AGENT_TUI_WS_ALLOW_REMOTE   Allow non-loopback WS bind (default: false)
    AGENT_TUI_WS_STATE          Daemon WS state file path (default: ~/.agent-tui/api.json)
    AGENT_TUI_WS_DISABLED       Disable daemon WS server (default: false)
    AGENT_TUI_WS_MAX_CONNECTIONS  Max WebSocket connections (default: 32)
    AGENT_TUI_WS_QUEUE          WS outbound queue size (default: 128)
    AGENT_TUI_API_LISTEN / AGENT_TUI_API_ALLOW_REMOTE / AGENT_TUI_API_STATE
                              Deprecated aliases for WS settings
    AGENT_TUI_SESSION_STORE     Session metadata log path (default: ~/.agent-tui/sessions.jsonl)
    AGENT_TUI_RECORD_STATE      Recording state file path (default: ~/.agent-tui/recordings.json)
    AGENT_TUI_RECORDINGS_DIR    Default recordings output directory (default: current directory)
    AGENT_TUI_LOG               Log file path (optional)
    AGENT_TUI_LOG_FORMAT        Log format (text or json; default: text)
    AGENT_TUI_LOG_STREAM        Log output stream (stderr or stdout; default: stderr)
    AGENT_TUI_UI_URL            External UI URL (optional)
    AGENT_TUI_UI_MODE           UI mode override (optional)
    AGENT_TUI_UI_PORT           UI port override (optional)
    AGENT_TUI_UI_ROOT           UI root path override (optional)
    AGENT_TUI_UI_STATE          UI state file path (optional)

EXAMPLES:
    # Start and interact with a TUI app
    agent-tui run "npx create-next-app"
    agent-tui screenshot
    agent-tui type "my-project"         # Type text
    agent-tui press Enter                 # Press Enter key
    agent-tui wait "success"
    agent-tui kill

    # Navigate menus efficiently
    agent-tui run htop
    agent-tui press F10
    agent-tui press ArrowDown ArrowDown Enter

    "#;

#[derive(Parser)]
#[command(name = "agent-tui")]
#[command(author, version, propagate_version = true)]
#[command(about = "CLI tool for AI agents to interact with TUI applications")]
#[command(long_about = LONG_ABOUT)]
#[command(after_help = AFTER_HELP)]
#[command(after_long_help = AFTER_LONG_HELP)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Session ID to use (defaults to the most recent session)
    #[arg(
        short,
        long,
        global = true,
        value_name = "ID",
        help_heading = "Session Options"
    )]
    pub session: Option<String>,

    /// Output format (text or json)
    #[arg(
        short,
        long,
        global = true,
        value_enum,
        value_name = "FORMAT",
        default_value_t = OutputFormat::Text,
        help_heading = "Output Options"
    )]
    pub format: OutputFormat,

    /// Shorthand for --format json (overrides --format if both are set)
    #[arg(long, global = true, help_heading = "Output Options")]
    pub json: bool,

    /// Disable colored output (also respects NO_COLOR)
    #[arg(
        long,
        global = true,
        env = "NO_COLOR",
        value_parser = clap::builder::BoolishValueParser::new(),
        help_heading = "Output Options"
    )]
    pub no_color: bool,
}

impl Cli {
    pub fn effective_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            self.format
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run a TUI application in a virtual terminal
    #[command(long_about = "\
Run a new TUI application in a virtual terminal.

Creates a new PTY session with the specified command and returns a session ID.
The session runs in the background and can be interacted with using other commands.
Use `--` before COMMAND args that start with `-` (e.g., `run -- vim -n`).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui run bash
    agent-tui run htop
    agent-tui run \"npx create-next-app\"
    agent-tui run vim -- file.txt
    agent-tui run --cols 80 --rows 24 nano")]
    Run {
        /// Command to run inside the virtual terminal
        #[arg(value_name = "COMMAND", value_hint = ValueHint::CommandName)]
        command: String,

        /// Arguments for the command (use -- to pass flags through)
        #[arg(trailing_var_arg = true, value_name = "ARG")]
        args: Vec<String>,

        /// Working directory for the command
        #[arg(short = 'd', long, value_name = "DIR", value_hint = ValueHint::DirPath)]
        cwd: Option<PathBuf>,

        /// Terminal columns (default: 120)
        #[arg(
            long,
            default_value_t = 120,
            value_name = "COLS",
            help_heading = "Terminal Size"
        )]
        cols: u16,

        /// Terminal rows (default: 40)
        #[arg(
            long,
            default_value_t = 40,
            value_name = "ROWS",
            help_heading = "Terminal Size"
        )]
        rows: u16,
    },

    /// Capture a screenshot of the current session
    #[command(long_about = "\
View the current screenshot state.

Returns the current terminal screenshot content.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui screenshot              # Just the screenshot
    agent-tui screenshot --strip-ansi # Plain text without colors")]
    Screenshot {
        /// Limit capture to a named region (if supported)
        #[arg(long, value_name = "REGION", help_heading = "Filtering")]
        region: Option<String>,

        /// Strip ANSI color codes from output
        #[arg(long, help_heading = "Output Options")]
        strip_ansi: bool,

        /// Include cursor position in output
        #[arg(long, help_heading = "Output Options")]
        include_cursor: bool,
    },
    /// Resize the session terminal
    #[command(long_about = "\
Resize the current session terminal.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui resize --cols 120 --rows 40")]
    Resize {
        /// Terminal columns
        #[arg(long, value_name = "COLS")]
        cols: u16,

        /// Terminal rows
        #[arg(long, value_name = "ROWS")]
        rows: u16,
    },

    /// Restart the current session
    #[command(long_about = "\
Restart the current session command, creating a new session.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui restart
    agent-tui --session abc123 restart")]
    Restart,
    /// Send key press(es) to the terminal (supports modifier hold/release)
    #[command(after_long_help = "\
NOTES:
    --hold/--release require a single modifier key (Ctrl, Alt, Shift, Meta)

EXAMPLES:
    agent-tui press Enter
    agent-tui press Ctrl+C
    agent-tui press ArrowDown ArrowDown Enter
    agent-tui press Shift --hold
    agent-tui press Shift --release")]
    Press {
        /// Keys to press (e.g., Enter, Ctrl+C, ArrowDown)
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,

        /// Hold a modifier key down (Ctrl, Alt, Shift, Meta)
        #[arg(long, conflicts_with = "release", help_heading = "Modifiers")]
        hold: bool,

        /// Release a held modifier key (Ctrl, Alt, Shift, Meta)
        #[arg(long, conflicts_with = "hold", help_heading = "Modifiers")]
        release: bool,
    },

    /// Type literal text character by character
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui type \"hello world\"
    agent-tui type \"user@example.com\"")]
    Type {
        /// Text to type
        #[arg(value_name = "TEXT", allow_hyphen_values = true)]
        text: String,
    },

    /// Wait for text or screenshot stability
    #[command(long_about = "\
Wait for a condition to be met before continuing.

Waits for text to appear/disappear or the screenshot to stabilize.
Returns success if the condition is met within the timeout period.

WAIT CONDITIONS:
    <text>       Wait for text to appear on screenshot
    --stable     Wait for screenshot to stop changing
    -g, --gone   Modifier: wait for text to disappear

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 1 if timeout.
                        Without --assert, always exit 0 (timeout still reported).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui wait \"Continue\"           # Wait for text
    agent-tui wait --stable             # Wait for screenshot stability
    agent-tui wait \"Loading\" --gone     # Wait for text to disappear
    agent-tui wait -t 5000 \"Done\"       # 5 second timeout")]
    Wait {
        #[command(flatten)]
        params: WaitParams,
    },

    /// Kill the current session
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui kill
    agent-tui --session abc123 kill")]
    Kill,

    /// List and manage sessions
    #[command(long_about = "\
Manage sessions - list, show details, attach, switch active, or cleanup.

By default, lists all active sessions.

MODES:
    list              List active sessions (default)
    show <id>         Show details for a session
    attach            Attach with TTY (defaults to --session or active)
    record            Record session activity to VHS artifacts
    switch <id>       Set the active session
    cleanup [--all]   Remove dead/orphaned sessions")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions                    # List sessions
    agent-tui sessions list               # List sessions (explicit)
    agent-tui sessions show abc123        # Show session details
    agent-tui sessions attach             # Attach to active session (TTY)
    agent-tui -s abc123 sessions attach   # Attach to session by id (TTY)
    agent-tui sessions switch abc123      # Set active session
    agent-tui -s abc123 sessions attach -T # Attach without TTY (stream output only)
    agent-tui sessions record             # Record active session in background
    agent-tui sessions record --foreground
    agent-tui sessions record -o docs/recordings
    agent-tui -s abc123 sessions record stop
    agent-tui sessions attach --detach-keys 'ctrl-]'  # Custom detach sequence
    agent-tui sessions cleanup            # Remove dead sessions
    agent-tui sessions cleanup --all      # Remove all sessions")]
    #[command(after_help = "Default action: list (same as `sessions list`).")]
    Sessions {
        #[command(subcommand)]
        command: Option<SessionsCommand>,
    },

    /// Live preview API for the current session
    #[command(long_about = "\
Show the daemon's live preview WebSocket endpoints.

The daemon serves a built-in web UI at /ui and exposes JSON-RPC over WebSocket at /ws.
Use this command to print WS/UI URLs so external frontends can connect.

CONFIGURATION:
    AGENT_TUI_WS_LISTEN          Bind address (default: 127.0.0.1:0)
    AGENT_TUI_WS_ALLOW_REMOTE    Allow non-loopback bind (default: false)
    AGENT_TUI_WS_STATE           State file path (default: ~/.agent-tui/api.json)
    AGENT_TUI_UI_URL             External UI URL to open with --open (CLI appends ws/session/auto)

SECURITY:
    Remote exposure is opt-in. Set AGENT_TUI_WS_ALLOW_REMOTE=1 for non-loopback binds.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui live start
    agent-tui live status
    agent-tui live stop")]
    #[command(after_help = "Default action: info (same as `live start`).")]
    Live {
        #[command(subcommand)]
        command: Option<LiveCommand>,
    },
    /// Manage the background daemon
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Show version information
    #[command(long_about = "\
Show detailed version information.

Shows version info for both the CLI binary and the running daemon.
Useful for verifying CLI/daemon compatibility.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui version
    agent-tui --format json version")]
    Version,

    /// Show environment diagnostics
    #[command(long_about = "\
Show environment diagnostics.

Displays all environment variables and configuration that affect
agent-tui behavior. Useful for troubleshooting connection issues.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui env
    agent-tui --format json env")]
    Env,
    /// Generate or install shell completions
    #[command(long_about = "\
Generate or install shell completions for bash, zsh, fish, powershell, or elvish.

Runs an interactive setup by default (auto-detects your shell) and checks
whether your installed completions are up-to-date. Use --print to output the
raw completion script for scripting or redirection.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui completions
    agent-tui completions zsh
    agent-tui completions --print bash
    agent-tui completions --install fish

INSTALLATION:
    # Bash - add to ~/.bashrc
    source <(agent-tui completions bash --print)

    # Zsh - add to ~/.zshrc
    source <(agent-tui completions zsh --print)

    # Fish - run once
    agent-tui completions fish --print > ~/.config/fish/completions/agent-tui.fish

    # PowerShell - add to $PROFILE
    agent-tui completions powershell --print | Out-String | Invoke-Expression")]
    Completions {
        #[arg(value_enum, value_name = "SHELL")]
        shell: Option<Shell>,
        /// Print the completion script to stdout
        #[arg(long, conflicts_with = "install")]
        print: bool,
        /// Install completions to the default location for the shell
        #[arg(long, conflicts_with = "print")]
        install: bool,
        /// Skip prompts and accept defaults
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommand {
    /// List active sessions
    #[command(alias = "ls")]
    List,

    /// Show details for a specific session
    Show {
        #[arg(value_name = "ID")]
        session_id: String,
    },

    /// Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-Q or --detach-keys)
    Attach {
        /// Disable TTY mode (stream output only)
        #[arg(short = 'T', long = "no-tty")]
        no_tty: bool,
        /// Detach key sequence (docker-style, e.g. "ctrl-p,ctrl-q"; use "none" to disable)
        #[arg(
            long = "detach-keys",
            value_name = "KEYS",
            env = "AGENT_TUI_DETACH_KEYS"
        )]
        detach_keys: Option<DetachKeys>,
    },

    /// Record a running session to VHS artifacts (.gif + .tape)
    #[command(long_about = "\
Record a running session to VHS artifacts.

By default recording starts in background and returns immediately.
Use --foreground to wait until recording exits.

OUTPUT PATH RULES:
    -o/--output-file omitted     Uses AGENT_TUI_RECORDINGS_DIR or current directory
    Existing directory           Creates timestamped <session>-<time>.gif/.tape
    Existing file                Uses file stem for .gif/.tape pair
    Non-existing path w/ ext     Treated as file path
    Non-existing path no ext     Treated as directory")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions record
    agent-tui sessions record --foreground
    agent-tui sessions record -o docs/recordings
    agent-tui sessions record -o docs/recordings/demo.gif
    agent-tui sessions record stop")]
    #[command(args_conflicts_with_subcommands = true)]
    Record {
        /// Output file or directory for recording artifacts
        #[arg(
            short = 'o',
            long = "output-file",
            value_name = "PATH",
            value_hint = ValueHint::AnyPath
        )]
        output_file: Option<PathBuf>,

        /// Run recorder in foreground (wait until recording exits)
        #[arg(long)]
        foreground: bool,

        #[command(subcommand)]
        command: Option<RecordCommand>,
    },

    /// Set the active session without attaching
    #[command(alias = "select")]
    Switch {
        #[arg(value_name = "ID")]
        session_id: String,
    },

    /// Remove dead/orphaned sessions
    Cleanup {
        /// Remove all sessions (including active)
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RecordCommand {
    /// Stop recording for the selected or active session
    Stop,
}

#[derive(Debug, Subcommand)]
pub enum LiveCommand {
    /// Show the live preview API details
    #[command(alias = "info")]
    Start(LiveStartArgs),

    /// Stop the live preview API (stop the daemon)
    Stop,

    /// Show live preview API status
    Status,
}

#[derive(Debug, Clone, Default, Args)]
pub struct LiveStartArgs {
    /// Deprecated (use AGENT_TUI_WS_LISTEN and restart the daemon)
    #[arg(
        short = 'l',
        long,
        value_name = "ADDR",
        num_args = 0..=1,
        default_missing_value = "127.0.0.1:0",
        help_heading = "Deprecated"
    )]
    pub listen: Option<String>,

    /// Deprecated (use AGENT_TUI_WS_ALLOW_REMOTE and restart the daemon)
    #[arg(long, help_heading = "Deprecated")]
    pub allow_remote: bool,

    /// Open the preview URL in a browser (uses AGENT_TUI_UI_URL if set)
    #[arg(long)]
    pub open: bool,

    /// Browser command to use (overrides $BROWSER)
    #[arg(long, value_name = "CMD", value_hint = ValueHint::CommandName)]
    pub browser: Option<String>,

    /// Deprecated (use AGENT_TUI_WS_MAX_CONNECTIONS and restart the daemon)
    #[arg(
        long,
        value_name = "COUNT",
        env = "AGENT_TUI_WS_MAX_CONNECTIONS",
        help_heading = "Deprecated"
    )]
    pub max_viewers: Option<u16>,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum DaemonCommand {
    /// Start the daemon process
    #[command(long_about = "\
Start the daemon process.

Starts the daemon in the background.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon start              # Start in background")]
    Start {},

    /// Stop the running daemon
    #[command(long_about = "\
Stop the running daemon.

Sends SIGTERM to gracefully stop the daemon, allowing it to clean up
sessions and resources. Use --force to send SIGKILL for immediate
termination (not recommended unless daemon is unresponsive).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon stop          # Graceful stop
    agent-tui daemon stop --force  # Force kill")]
    Stop {
        /// Force kill the daemon (SIGKILL)
        #[arg(long)]
        force: bool,
    },

    /// Restart the daemon
    #[command(long_about = "\
Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.")]
    Restart,
}

#[derive(Debug, Clone, Default, Args)]
#[command(
    group = ArgGroup::new("wait_condition")
        .multiple(false)
        .required(true)
        .args(&["text", "stable"])
)]
pub struct WaitParams {
    /// Text to wait for (positional)
    #[arg(value_name = "TEXT", allow_hyphen_values = true)]
    pub text: Option<String>,

    /// Timeout in milliseconds (default: 30000)
    #[arg(
        short,
        long,
        default_value_t = 30_000,
        value_name = "MILLIS",
        help_heading = "Timing"
    )]
    pub timeout: u64,

    /// Wait for the screenshot to stop changing
    #[arg(long, group = "wait_condition", help_heading = "Wait Condition")]
    pub stable: bool,

    /// Wait for the text to disappear
    #[arg(short = 'g', long, requires = "text", help_heading = "Wait Condition")]
    pub gone: bool,

    /// Exit with status 0 if met, 1 on timeout
    #[arg(long, help_heading = "Behavior")]
    pub assert: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ColorChoice;
    use clap::CommandFactory;
    use clap::Parser;
    use clap::error::ErrorKind;

    #[test]
    fn test_cli_defaults() {
        // SAFETY: Test-only cleanup of NO_COLOR to verify default parsing.
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        assert!(cli.session.is_none());
        assert_eq!(cli.format, OutputFormat::Text);
        assert!(!cli.no_color);
    }

    #[test]
    fn test_global_args() {
        let cli = Cli::parse_from([
            "agent-tui",
            "--session",
            "my-session",
            "--format",
            "json",
            "--no-color",
            "sessions",
        ]);
        assert_eq!(cli.session, Some("my-session".to_string()));
        assert_eq!(cli.format, OutputFormat::Json);
        assert!(cli.no_color);
    }

    #[test]
    fn test_run_requires_command() {
        let err = Cli::try_parse_from(["agent-tui", "run"])
            .err()
            .expect("expected parse error");
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_run_defaults() {
        let cli = Cli::parse_from(["agent-tui", "run", "bash"]);
        let Commands::Run {
            command,
            args,
            cwd,
            cols,
            rows,
        } = cli.command
        else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(command, "bash");
        assert!(args.is_empty());
        assert!(cwd.is_none());

        assert_eq!(cols, 120, "Default cols should be 120");
        assert_eq!(rows, 40, "Default rows should be 40");
    }

    #[test]
    fn test_run_custom_dimensions() {
        let cli = Cli::parse_from(["agent-tui", "run", "--cols", "80", "--rows", "24", "vim"]);
        let Commands::Run {
            cols,
            rows,
            command,
            ..
        } = cli.command
        else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(cols, 80);
        assert_eq!(rows, 24);
        assert_eq!(command, "vim");
    }

    #[test]
    fn test_run_with_args() {
        let cli = Cli::parse_from(["agent-tui", "run", "vim", "--", "file.txt", "-n"]);
        let Commands::Run { command, args, .. } = cli.command else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(command, "vim");
        assert_eq!(args, vec!["file.txt".to_string(), "-n".to_string()]);
    }

    #[test]
    fn test_screenshot_flags() {
        let cli = Cli::parse_from([
            "agent-tui",
            "screenshot",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ]);
        let Commands::Screenshot {
            region,
            strip_ansi,
            include_cursor,
        } = cli.command
        else {
            panic!("Expected Screenshot command, got {:?}", cli.command);
        };
        assert_eq!(region, Some("modal".to_string()));
        assert!(strip_ansi);
        assert!(include_cursor);
    }

    #[test]
    fn test_wait_requires_condition() {
        let err = Cli::try_parse_from(["agent-tui", "wait"])
            .err()
            .expect("expected parse error");
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn test_wait_defaults() {
        let cli = Cli::parse_from(["agent-tui", "wait", "Loading"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Loading".to_string()));

        assert_eq!(params.timeout, 30000, "Default timeout should be 30000ms");
    }

    #[test]
    fn test_wait_custom_timeout() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-t", "5000", "Done"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Done".to_string()));
        assert_eq!(params.timeout, 5000);
    }

    #[test]
    fn test_wait_allows_hyphen_text() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-flaglike"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("-flaglike".to_string()));
    }

    #[test]
    fn test_wait_stable() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--stable"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(params.stable);
        assert!(params.text.is_none());
    }

    #[test]
    fn test_wait_text_gone() {
        let cli = Cli::parse_from(["agent-tui", "wait", "Loading...", "--gone"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Loading...".to_string()));
        assert!(params.gone);
    }

    #[test]
    fn test_wait_assert_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--assert", "Success"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(params.assert);
        assert_eq!(params.text, Some("Success".to_string()));
    }

    #[test]
    fn test_missing_required_args() {
        assert!(Cli::try_parse_from(["agent-tui", "run"]).is_err());
    }

    #[test]
    fn test_output_format_values() {
        let cli = Cli::parse_from(["agent-tui", "-f", "text", "sessions"]);
        assert_eq!(cli.format, OutputFormat::Text);

        let cli = Cli::parse_from(["agent-tui", "-f", "json", "sessions"]);
        assert_eq!(cli.format, OutputFormat::Json);

        assert!(Cli::try_parse_from(["agent-tui", "-f", "xml", "sessions"]).is_err());
    }

    #[test]
    fn test_json_shorthand_flag() {
        let cli = Cli::parse_from(["agent-tui", "--json", "sessions"]);
        assert!(cli.json);
    }

    #[test]
    fn test_run_with_cwd() {
        let cli = Cli::parse_from(["agent-tui", "run", "-d", "/tmp", "bash"]);
        let Commands::Run { command, cwd, .. } = cli.command else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(command, "bash");
        assert_eq!(cwd, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_sessions_list() {
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(command.is_none());
    }

    #[test]
    fn test_sessions_all_flag_rejected() {
        let err = Cli::try_parse_from(["agent-tui", "sessions", "--all"])
            .err()
            .expect("expected parse error");
        assert!(matches!(
            err.kind(),
            ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
        ));
    }

    #[test]
    fn test_sessions_list_explicit() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "list"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(command, Some(SessionsCommand::List)));
    }

    #[test]
    fn test_sessions_list_alias_ls() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "ls"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(command, Some(SessionsCommand::List)));
    }

    #[test]
    fn test_sessions_attach_default() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "attach"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Attach {
                no_tty: false,
                detach_keys: None
            })
        ));
    }

    #[test]
    fn test_sessions_attach_with_id_rejected() {
        let err = Cli::try_parse_from(["agent-tui", "sessions", "attach", "my-session"])
            .err()
            .expect("expected parse error");
        assert!(matches!(
            err.kind(),
            ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
        ));
    }

    #[test]
    fn test_sessions_attach_no_tty() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "attach", "-T"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Attach {
                no_tty: true,
                detach_keys: None
            })
        ));
    }

    #[test]
    fn test_sessions_record_defaults() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "record"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Record {
                output_file: None,
                foreground: false,
                command: None
            })
        ));
    }

    #[test]
    fn test_sessions_record_foreground() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "record", "--foreground"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Record {
                foreground: true,
                command: None,
                ..
            })
        ));
    }

    #[test]
    fn test_sessions_record_output_file() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "record", "-o", "out.gif"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Record {
                output_file: Some(_),
                foreground: false,
                command: None
            })
        ));
    }

    #[test]
    fn test_sessions_record_stop() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "record", "stop"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Record {
                command: Some(RecordCommand::Stop),
                ..
            })
        ));
    }

    #[test]
    fn test_sessions_cleanup() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "cleanup"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Cleanup { all: false })
        ));
    }

    #[test]
    fn test_sessions_cleanup_all() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "cleanup", "--all"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Cleanup { all: true })
        ));
    }

    #[test]
    fn test_sessions_show_with_id() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "show", "abc123"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Show { session_id: _ })
        ));
    }

    #[test]
    fn test_sessions_switch_with_id() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "switch", "abc123"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Switch { session_id: _ })
        ));
    }

    #[test]
    fn test_version_command() {
        let cli = Cli::parse_from(["agent-tui", "version"]);
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn test_env_command() {
        let cli = Cli::parse_from(["agent-tui", "env"]);
        assert!(matches!(cli.command, Commands::Env));
    }

    #[test]
    fn test_kill_command() {
        let cli = Cli::parse_from(["agent-tui", "kill"]);
        assert!(matches!(cli.command, Commands::Kill));
    }

    #[test]
    fn test_completions_command() {
        let cli = Cli::parse_from(["agent-tui", "completions", "bash"]);
        let Commands::Completions { shell, .. } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Some(Shell::Bash)));
    }

    #[test]
    fn test_completions_fish() {
        let cli = Cli::parse_from(["agent-tui", "completions", "fish"]);
        let Commands::Completions { shell, .. } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Some(Shell::Fish)));
    }

    #[test]
    fn test_completions_default_guided() {
        let cli = Cli::parse_from(["agent-tui", "completions"]);
        let Commands::Completions {
            shell,
            print,
            install,
            yes,
        } = cli.command
        else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(shell.is_none());
        assert!(!print);
        assert!(!install);
        assert!(!yes);
    }

    #[test]
    fn test_live_info_alias() {
        let cli = Cli::parse_from(["agent-tui", "live", "info"]);
        let Commands::Live { command } = cli.command else {
            panic!("Expected Live command, got {:?}", cli.command);
        };
        assert!(matches!(command, Some(LiveCommand::Start(_))));
    }

    #[test]
    fn test_daemon_start_default() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "start"]);
        let Commands::Daemon(DaemonCommand::Start {}) = cli.command else {
            panic!("Expected Daemon Start command, got {:?}", cli.command);
        };
    }

    #[test]
    fn test_daemon_stop_default() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "stop"]);
        let Commands::Daemon(DaemonCommand::Stop { force }) = cli.command else {
            panic!("Expected Daemon Stop command, got {:?}", cli.command);
        };
        assert!(!force, "Default should be graceful stop");
    }

    #[test]
    fn test_daemon_stop_force() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "stop", "--force"]);
        let Commands::Daemon(DaemonCommand::Stop { force }) = cli.command else {
            panic!("Expected Daemon Stop command, got {:?}", cli.command);
        };
        assert!(force, "Should be force stop");
    }

    #[test]
    fn test_daemon_restart() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "restart"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommand::Restart)
        ));
    }

    #[test]
    fn test_restart_command_parses() {
        let cli = Cli::parse_from(["agent-tui", "restart"]);
        assert!(matches!(cli.command, Commands::Restart));
    }

    #[test]
    fn test_resize_command_parses() {
        let cli = Cli::parse_from(["agent-tui", "resize", "--cols", "80", "--rows", "24"]);
        let Commands::Resize { cols, rows } = cli.command else {
            panic!("Expected Resize command, got {:?}", cli.command);
        };
        assert_eq!(cols, 80);
        assert_eq!(rows, 24);
    }

    // Phase 1: Press and Type commands
    #[test]
    fn test_press_enter_command() {
        let cli = Cli::parse_from(["agent-tui", "press", "Enter"]);
        let Commands::Press {
            keys,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "Enter");
        assert!(!hold);
        assert!(!release);
    }

    #[test]
    fn test_press_key_sequence() {
        let cli = Cli::parse_from(["agent-tui", "press", "ArrowDown", "ArrowDown", "Enter"]);
        let Commands::Press {
            keys,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "ArrowDown");
        assert_eq!(keys[1], "ArrowDown");
        assert_eq!(keys[2], "Enter");
        assert!(!hold);
        assert!(!release);
    }

    #[test]
    fn test_press_with_modifier() {
        let cli = Cli::parse_from(["agent-tui", "press", "Ctrl+C"]);
        let Commands::Press {
            keys,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys[0], "Ctrl+C");
        assert!(!hold);
        assert!(!release);
    }

    #[test]
    fn test_press_allows_hyphen_key() {
        let cli = Cli::parse_from(["agent-tui", "press", "-"]);
        let Commands::Press { keys, .. } = cli.command else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys, vec!["-".to_string()]);
    }

    #[test]
    fn test_press_hold_command() {
        let cli = Cli::parse_from(["agent-tui", "press", "Shift", "--hold"]);
        let Commands::Press {
            keys,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys[0], "Shift");
        assert!(hold);
        assert!(!release);
    }

    #[test]
    fn test_press_release_command() {
        let cli = Cli::parse_from(["agent-tui", "press", "Shift", "--release"]);
        let Commands::Press {
            keys,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys[0], "Shift");
        assert!(!hold);
        assert!(release);
    }

    #[test]
    fn test_press_flag_conflicts() {
        assert!(
            Cli::try_parse_from(["agent-tui", "press", "Shift", "--hold", "--release"]).is_err()
        );
    }

    #[test]
    fn test_type_command() {
        let cli = Cli::parse_from(["agent-tui", "type", "hello"]);
        let Commands::Type { text } = cli.command else {
            panic!("Expected Type command, got {:?}", cli.command);
        };
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_type_allows_hyphen_text() {
        let cli = Cli::parse_from(["agent-tui", "type", "-n"]);
        let Commands::Type { text } = cli.command else {
            panic!("Expected Type command, got {:?}", cli.command);
        };
        assert_eq!(text, "-n");
    }

    #[test]
    fn test_type_command_with_spaces() {
        let cli = Cli::parse_from(["agent-tui", "type", "Hello, World!"]);
        let Commands::Type { text } = cli.command else {
            panic!("Expected Type command, got {:?}", cli.command);
        };
        assert_eq!(text, "Hello, World!");
    }

    #[test]
    fn test_cli_long_help_renders_without_color() {
        let mut cmd = Cli::command();
        cmd = cmd.color(ColorChoice::Never);
        let _ = cmd.render_long_help().to_string();
    }
}
