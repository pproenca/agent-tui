//! CLI command parsing and configuration.

use clap::ArgGroup;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use clap::ValueHint;
use std::collections::HashMap;
use std::path::PathBuf;

pub use crate::adapters::presenter::OutputFormat;
use crate::app::attach::DetachKeys;
use crate::domain::TerminalSize;

const AFTER_HELP: &str =
    "Use --help for full details and examples. Use --format json for machine-readable output.";

const LONG_ABOUT: &str = "\
Drive TUI (text UI) applications programmatically or interactively.\n\
\n\
Common flow: run -> screenshot -> press/type/scroll -> wait -> kill.\n\
Use --format json for automation-friendly output.\n\
\n\
Supported platforms: Unix-like systems only (Linux, macOS, and environments\n\
with PTYs, Unix domain sockets, and POSIX signals).";

const AFTER_LONG_HELP: &str = r#"WORKFLOW:
    1. Run a TUI application
    2. View the screenshot
    3. Interact with keys/text or scroll
    4. Wait for UI changes
    5. Kill the session when done

OUTPUT:
    --format json  Machine-readable JSON (recommended for automation)
    --format text  Human-readable text (default)

CONFIGURATION:
    AGENT_TUI_NO_INPUT          Disable prompts and interactive TTY behavior (default: false)
    AGENT_TUI_TRANSPORT         IPC transport (unix or ws; default: unix)
    AGENT_TUI_WS_ADDR           Remote WS-RPC target when transport is ws (e.g. ws://host:port/ws)
    AGENT_TUI_DETACH_KEYS       Detach keys for `sessions attach` (default: Ctrl-P Ctrl-B)
    AGENT_TUI_WS_LISTEN         Daemon WS bind address (default: 127.0.0.1:0)
    AGENT_TUI_WS_ALLOW_REMOTE   Allow non-loopback WS bind (default: false)
    AGENT_TUI_WS_STATE          Daemon WS state file path (default: ~/.agent-tui/api.json)
    AGENT_TUI_WS_DISABLED       Disable daemon WS server (default: false)
    AGENT_TUI_WS_MAX_CONNECTIONS  Max WebSocket connections (default: 32)
    AGENT_TUI_WS_QUEUE          WS outbound queue size (default: 128)
    AGENT_TUI_SESSION_STORE     Session metadata log path (default: ~/.agent-tui/sessions.jsonl)
    AGENT_TUI_LOG               Log file path (optional)
    AGENT_TUI_LOG_FORMAT        Log format (text or json; default: text)
    AGENT_TUI_LOG_STREAM        Log output stream (stderr or stdout; default: stderr)
    AGENT_TUI_UI_URL            Same-origin UI URL or path override for live preview opening (optional)
    AGENT_TUI_UI_MODE           UI mode override (optional)
    AGENT_TUI_UI_PORT           UI port override (optional)
    AGENT_TUI_UI_ROOT           UI root path override (optional)
    AGENT_TUI_UI_STATE          UI state file path (optional)

EXAMPLES:
    # Start and interact with a TUI app
    agent-tui run "npx create-next-app"
    agent-tui screenshot
    agent-tui type "my-project"         # Type text
    agent-tui press Ctrl+M              # Submit the current input
    agent-tui wait "success"
    agent-tui kill

    # Navigate menus efficiently
    agent-tui run htop
    agent-tui press F10
    agent-tui press ArrowDown ArrowDown Enter

    # Scroll using directional terminal input
    agent-tui scroll down
    agent-tui scroll up 5

PLATFORM SUPPORT:
    Supported: Linux, macOS, and other Unix-like systems with PTYs,
    Unix domain sockets, and POSIX signals.
    Unsupported: Windows and non-Unix runtimes.

    "#;

fn parse_terminal_cols(value: &str) -> Result<u16, String> {
    let cols = value
        .parse::<u16>()
        .map_err(|err| format!("invalid column count '{value}': {err}"))?;

    if cols < TerminalSize::MIN_COLS {
        return Err(format!(
            "columns must be at least {}",
            TerminalSize::MIN_COLS
        ));
    }
    if cols > TerminalSize::MAX_COLS {
        return Err(format!(
            "columns must be at most {}",
            TerminalSize::MAX_COLS
        ));
    }

    Ok(cols)
}

fn parse_terminal_rows(value: &str) -> Result<u16, String> {
    let rows = value
        .parse::<u16>()
        .map_err(|err| format!("invalid row count '{value}': {err}"))?;

    if rows < TerminalSize::MIN_ROWS {
        return Err(format!("rows must be at least {}", TerminalSize::MIN_ROWS));
    }
    if rows > TerminalSize::MAX_ROWS {
        return Err(format!("rows must be at most {}", TerminalSize::MAX_ROWS));
    }

    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvAssignment {
    pub key: String,
    pub value: String,
}

fn parse_env_assignment(value: &str) -> Result<EnvAssignment, String> {
    let Some((key, env_value)) = value.split_once('=') else {
        return Err("expected KEY=VALUE".to_string());
    };
    if key.is_empty() {
        return Err("environment variable name cannot be empty".to_string());
    }
    if key.contains('\0') || env_value.contains('\0') {
        return Err("environment variables cannot contain NUL bytes".to_string());
    }

    Ok(EnvAssignment {
        key: key.to_string(),
        value: env_value.to_string(),
    })
}

pub(crate) fn env_assignments_to_map(
    assignments: Vec<EnvAssignment>,
) -> Option<HashMap<String, String>> {
    if assignments.is_empty() {
        return None;
    }

    Some(
        assignments
            .into_iter()
            .map(|assignment| (assignment.key, assignment.value))
            .collect(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Elvish,
}

impl CompletionShell {
    pub const fn clap_shell(self) -> clap_complete::Shell {
        match self {
            Self::Bash => clap_complete::Shell::Bash,
            Self::Zsh => clap_complete::Shell::Zsh,
            Self::Fish => clap_complete::Shell::Fish,
            Self::Elvish => clap_complete::Shell::Elvish,
        }
    }
}

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

    /// Disable prompts and interactive TTY behavior; require explicit flags instead
    #[arg(
        long,
        global = true,
        env = "AGENT_TUI_NO_INPUT",
        value_parser = clap::builder::BoolishValueParser::new(),
        help_heading = "Interaction Options"
    )]
    pub no_input: bool,
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
    agent-tui run --env FOO=bar --env BAZ=qux bash
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

        /// Environment variable override for the spawned session (repeatable)
        #[arg(
            long = "env",
            value_name = "KEY=VALUE",
            value_parser = parse_env_assignment,
            help_heading = "Environment"
        )]
        env: Vec<EnvAssignment>,

        /// Terminal columns (default: 120)
        #[arg(
            long,
            default_value_t = 120,
            value_name = "COLS",
            value_parser = parse_terminal_cols,
            help_heading = "Terminal Size"
        )]
        cols: u16,

        /// Terminal rows (default: 40)
        #[arg(
            long,
            default_value_t = 40,
            value_name = "ROWS",
            value_parser = parse_terminal_rows,
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
    agent-tui screenshot               # Screenshot with terminal colors/styles
    agent-tui screenshot --retain-ansi # Explicitly preserve terminal colors/styles
    agent-tui screenshot --strip-ansi  # Plain text without colors

LEGACY COMPATIBILITY:
    agent-tui screenshot -e             # Deprecated; returns the standard screenshot
    agent-tui screenshot -a             # Deprecated; returns the standard screenshot
    agent-tui screenshot --interactive-only # Deprecated; returns the standard screenshot")]
    Screenshot {
        /// Reserved for future named regions; currently rejected if provided
        #[arg(long, value_name = "REGION", help_heading = "Filtering")]
        region: Option<String>,

        /// Strip ANSI color codes from output
        #[arg(long, conflicts_with = "retain_ansi", help_heading = "Output Options")]
        strip_ansi: bool,

        /// Preserve ANSI color/style codes in output (default)
        #[arg(long, conflicts_with = "strip_ansi", help_heading = "Output Options")]
        retain_ansi: bool,

        /// Include cursor position in output
        #[arg(long, help_heading = "Output Options")]
        include_cursor: bool,

        /// Deprecated compatibility flag; returns the standard terminal screenshot
        #[arg(short = 'e', help_heading = "Legacy Compatibility")]
        legacy_element: bool,

        /// Deprecated compatibility flag; returns the standard terminal screenshot
        #[arg(short = 'a', help_heading = "Legacy Compatibility")]
        legacy_accessibility: bool,

        /// Deprecated compatibility flag; returns the standard terminal screenshot
        #[arg(long = "interactive-only", help_heading = "Legacy Compatibility")]
        legacy_interactive_only: bool,
    },

    /// Deprecated selector action compatibility command
    #[command(long_about = "\
Deprecated compatibility command for old selector-based action workflows.

Use current terminal commands (`press`, `type`, and `scroll`) for new scripts.")]
    #[command(after_long_help = "\
SUPPORTED COMPATIBILITY FORMS:
    agent-tui action <selector> click        # Sends Enter with `agent-tui press Enter`
    agent-tui action <selector> fill <text>  # Types text with `agent-tui type <text>`

Unsupported selector actions return a compatibility error with migration guidance.")]
    Action {
        /// Legacy selector/action form
        #[arg(value_name = "FORM", required = true, num_args = 1.., allow_hyphen_values = true)]
        form: Vec<String>,
    },
    /// Resize the session terminal
    #[command(long_about = "\
Resize the current session terminal.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui resize --cols 120 --rows 40")]
    Resize {
        /// Terminal columns
        #[arg(long, value_name = "COLS", value_parser = parse_terminal_cols)]
        cols: u16,

        /// Terminal rows
        #[arg(long, value_name = "ROWS", value_parser = parse_terminal_rows)]
        rows: u16,
    },

    /// Restart the current session
    #[command(long_about = "\
Restart the current session command, creating a new session.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui restart --yes
    agent-tui --session abc123 restart --dry-run")]
    Restart {
        /// Preview the restart without changing the session
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
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
    #[command(long_about = "\
Type literal text character by character.

Pass `-` to read the text payload from stdin in non-interactive pipelines.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui type \"hello world\"
    agent-tui type \"user@example.com\"
    printf 'project-name' | agent-tui type -")]
    Type {
        /// Text to type
        #[arg(value_name = "TEXT", allow_hyphen_values = true)]
        text: String,
    },

    /// Legacy alias for `type`
    #[command(long_about = "\
Legacy alias for `agent-tui type`.

Use `agent-tui type` for new scripts.")]
    Input {
        /// Text to type
        #[arg(value_name = "TEXT", allow_hyphen_values = true)]
        text: String,
    },

    /// Scroll using repeated directional terminal input
    #[command(long_about = "\
Send repeated directional input to the terminal.

This is a thin convenience wrapper over terminal keys:
    up    -> ArrowUp
    down  -> ArrowDown
    left  -> ArrowLeft
    right -> ArrowRight")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui scroll down
    agent-tui scroll up 10
    agent-tui scroll right 3")]
    Scroll {
        /// Direction to move
        #[arg(value_enum, value_name = "DIRECTION")]
        direction: ScrollDirection,

        /// Number of steps to send
        #[arg(default_value_t = 1, value_name = "AMOUNT")]
        amount: u16,
    },

    /// Deprecated element scroll compatibility command
    #[command(long_about = "\
Deprecated compatibility command for old element scroll workflows.

The current CLI has no element selector engine. This command does not send terminal input; use `scroll` or `press` for new scripts.")]
    #[command(after_long_help = "\
SUPPORTED COMPATIBILITY FORMS:
    agent-tui scroll-into-view <selector>  # No-op compatibility success

Unsupported selector options return a compatibility error with migration guidance.")]
    ScrollIntoView {
        /// Legacy selector form
        #[arg(value_name = "FORM", required = true, num_args = 1.., allow_hyphen_values = true)]
        form: Vec<String>,
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
    -e <ref>     Deprecated: treats element ref as literal text

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 75 if timeout.
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
    agent-tui kill --yes
    agent-tui --session abc123 kill --dry-run")]
    Kill {
        /// Preview the kill without changing the session
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List and manage sessions
    #[command(long_about = "\
Manage sessions - list, show details, attach, switch active, or cleanup.

By default, lists all active sessions.

MODES:
    list              List active sessions (default)
    show <id>         Show details for a session
    attach            Attach with TTY (defaults to --session or active)
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
    agent-tui sessions attach --detach-keys 'ctrl-]'  # Custom detach sequence
    agent-tui sessions cleanup --yes            # Remove dead sessions
    agent-tui sessions cleanup --all --dry-run  # Preview removing all sessions")]
    #[command(after_help = "Default action: list (same as `sessions list`).")]
    Sessions {
        #[command(subcommand)]
        command: Option<SessionsCommand>,
    },

    /// Live preview API exposed by the local daemon
    #[command(long_about = "\
Show the local daemon's live preview WebSocket endpoints.

The daemon serves a built-in web UI at /ui and exposes JSON-RPC over WebSocket at /ws.
Use this command to print WS/UI URLs so external frontends can connect.
This command always inspects the local daemon and does not use AGENT_TUI_TRANSPORT.

CONFIGURATION:
    AGENT_TUI_WS_LISTEN          Bind address (default: 127.0.0.1:0)
    AGENT_TUI_WS_ALLOW_REMOTE    Allow non-loopback bind (default: false)
    AGENT_TUI_WS_STATE           State file path (default: ~/.agent-tui/api.json)
    AGENT_TUI_UI_URL             Same-origin UI URL or path to open with --open (CLI appends ws/session/auto)

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
    #[command(long_about = "\
Manage the background daemon lifecycle.

Use `daemon start` to launch in the background, `daemon run` for foreground
debugging, and `daemon status` to inspect the local daemon state.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon start
    agent-tui daemon status
    agent-tui daemon stop --yes")]
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Show version information
    #[command(long_about = "\
Show detailed version information.

Shows version info for both the CLI binary and the local running daemon.
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
Generate or install shell completions for bash, zsh, fish, or elvish.

Runs an interactive setup by default (auto-detects your shell) and checks
whether your installed completions are up-to-date. Use --print to output the
raw completion script for scripting or redirection.

Use --no-input to disable prompts and require explicit shell selection.")]
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

    # Elvish - run once
    agent-tui completions elvish --print > ~/.elvish/lib/agent-tui.elv")]
    Completions {
        #[arg(value_enum, value_name = "SHELL")]
        shell: Option<CompletionShell>,
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
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions list
    agent-tui --json sessions list")]
    List,

    /// Show details for a specific session
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions show abc123
    agent-tui --json sessions show abc123")]
    Show {
        #[arg(value_name = "ID")]
        session_id: String,
    },

    /// Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-B or --detach-keys)
    #[command(after_long_help = "\
NOTES:
    --no-input implies --no-tty for automation-safe streaming.

EXAMPLES:
    agent-tui sessions attach
    agent-tui -s abc123 sessions attach --no-tty
    agent-tui --no-input sessions attach")]
    Attach {
        /// Disable TTY mode (stream output only)
        #[arg(short = 'T', long = "no-tty")]
        no_tty: bool,
        /// Detach key sequence (docker-style, e.g. "ctrl-p,ctrl-b"; use "none" to disable)
        #[arg(
            long = "detach-keys",
            value_name = "KEYS",
            env = "AGENT_TUI_DETACH_KEYS"
        )]
        detach_keys: Option<DetachKeys>,
    },

    /// Set the active session without attaching
    #[command(alias = "select")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions switch abc123
    agent-tui sessions select abc123")]
    Switch {
        #[arg(value_name = "ID")]
        session_id: String,
    },

    /// Remove dead/orphaned sessions
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions cleanup --yes
    agent-tui sessions cleanup --all --dry-run")]
    Cleanup {
        /// Remove all sessions (including active)
        #[arg(long)]
        all: bool,

        /// Preview which sessions would be cleaned without killing them
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum LiveCommand {
    /// Show the live preview API details
    #[command(alias = "info")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui live start
    agent-tui live start --open")]
    Start(LiveStartArgs),

    /// Stop any managed UI server and show how to stop daemon-backed live preview
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui live stop
    agent-tui daemon stop --yes   # Stop daemon-backed live preview")]
    Stop,

    /// Show live preview API status
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui live status
    agent-tui --json live status")]
    Status,
}

#[derive(Debug, Clone, Default, Args)]
pub struct LiveStartArgs {
    /// Open the preview URL in a browser (uses AGENT_TUI_UI_URL if set)
    #[arg(long)]
    pub open: bool,

    /// Browser command to use (overrides $BROWSER)
    #[arg(long, value_name = "CMD", value_hint = ValueHint::CommandName)]
    pub browser: Option<String>,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum DaemonCommand {
    /// Start the daemon process
    #[command(long_about = "\
Start the daemon process.

Starts the daemon in the background. Use `daemon run` to keep it in the
foreground.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon start              # Start in background")]
    Start {},

    /// Run the daemon in the foreground
    #[command(long_about = "\
Run the daemon in the foreground.

This is the UNIX-style form for supervisors and local debugging when you want
the daemon attached to the current process instead of forking to the
background.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon run
    AGENT_TUI_WS_LISTEN=0.0.0.0:8080 agent-tui daemon run")]
    Run,

    /// Stop the running daemon
    #[command(long_about = "\
Stop the running daemon.

Sends SIGTERM to gracefully stop the daemon, allowing it to clean up
sessions and resources. Use --force to send SIGKILL for immediate
termination (not recommended unless daemon is unresponsive).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon stop --yes          # Graceful stop
    agent-tui daemon stop --force --yes  # Force kill")]
    Stop {
        /// Force kill the daemon (SIGKILL)
        #[arg(long)]
        force: bool,

        /// Preview the stop without changing daemon state
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Show daemon status
    #[command(long_about = "\
Show daemon status.

Reports whether the daemon is running, its PID, versions, and any discovered
WS/UI endpoints.

EXIT CODES (LSB init script conventions):
    0 - Daemon is running
    3 - Daemon is not running")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon status
    agent-tui --json daemon status")]
    Status,

    /// Restart the daemon
    #[command(long_about = "\
Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon restart --yes
    agent-tui daemon restart --dry-run")]
    Restart {
        /// Preview the restart without changing daemon state
        #[arg(long)]
        dry_run: bool,

        /// Skip interactive confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Default, Args)]
#[command(
    group = ArgGroup::new("wait_condition")
        .multiple(false)
        .required(true)
        .args(&["text", "stable", "legacy_element"]),
    group = ArgGroup::new("wait_text_condition")
        .multiple(false)
        .required(false)
        .args(&["text", "legacy_element"])
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
    #[arg(
        short = 'g',
        long,
        requires = "wait_text_condition",
        help_heading = "Wait Condition"
    )]
    pub gone: bool,

    /// Deprecated compatibility flag; treats the element ref as literal text
    #[arg(
        short = 'e',
        value_name = "REF",
        allow_hyphen_values = true,
        group = "wait_condition",
        help_heading = "Legacy Compatibility"
    )]
    pub legacy_element: Option<String>,

    /// Exit with status 0 if met, 75 on timeout
    #[arg(long, help_heading = "Behavior")]
    pub assert: bool,
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
