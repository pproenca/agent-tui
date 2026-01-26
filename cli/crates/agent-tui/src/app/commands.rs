use clap::ArgGroup;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use clap::ValueHint;
pub use clap_complete::Shell;
use std::path::PathBuf;

use crate::app::attach::DetachKeys;

const AFTER_HELP: &str =
    "Use --help for full details and examples. Use --format json for machine-readable output.";

const LONG_ABOUT: &str = "\
Drive TUI (text UI) applications programmatically or interactively.\n\
\n\
Common flow: run -> screenshot -> action/press/input -> wait -> kill.\n\
Use --format json for automation-friendly output.";

const AFTER_LONG_HELP: &str = r#"WORKFLOW:
    1. Run a TUI application
    2. View the screenshot and detect elements
    3. Interact with elements or press keys
    4. Wait for UI changes
    5. Kill the session when done

SELECTORS:
    @e1, @e2, @e3  - Element refs (from 'screenshot -e' output)
    @"Submit"      - Find element by exact text
    :Submit        - Find element by partial text (contains)

OUTPUT:
    --format json  Machine-readable JSON (recommended for automation)
    --format text  Human-readable text (default)

EXAMPLES:
    # Start and interact with a TUI app
    agent-tui run "npx create-next-app"
    agent-tui screenshot -e
    agent-tui @e1 "my-project"           # Fill input with value
    agent-tui press Enter                 # Press Enter key
    agent-tui wait "success"
    agent-tui kill

    # Navigate menus efficiently
    agent-tui run htop
    agent-tui press F10
    agent-tui screenshot -e
    agent-tui @e1                         # Activate element (click)
    agent-tui press ArrowDown ArrowDown Enter

    # Use text selectors for readable scripts
    agent-tui @"Yes, proceed"             # Click by exact text
    agent-tui :Submit                     # Click element containing "Submit"

    # Check daemon status
    agent-tui daemon status"#;

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

    /// Shorthand for --format json
    #[arg(long, global = true, help_heading = "Output Options")]
    pub json: bool,

    /// Disable colored output (also respects NO_COLOR)
    #[arg(long, global = true, env = "NO_COLOR", help_heading = "Output Options")]
    pub no_color: bool,

    /// Enable verbose output (shows request timing)
    #[arg(short, long, global = true, help_heading = "Debug Options")]
    pub verbose: bool,
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

    /// Capture a screenshot and detect UI elements
    #[command(long_about = "\
View the current screenshot state.

Returns the current terminal screenshot content and optionally detects
interactive UI elements like buttons, inputs, and menus.

Element detection uses the Visual Object Model (VOM) which identifies
UI components based on visual styling (colors, backgrounds) rather than
text patterns. This provides reliable detection across different TUI frameworks.

ACCESSIBILITY TREE FORMAT (-a):
    Returns an agent-browser style accessibility tree with refs for elements:
    - button \"Submit\" [ref=e1]
    - textbox \"Search\" [ref=e2]")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui screenshot              # Just the screenshot
    agent-tui screenshot -e           # Screenshot + detected elements
    agent-tui screenshot -a           # Accessibility tree format
    agent-tui screenshot -a --interactive-only  # Only interactive elements
    agent-tui screenshot --strip-ansi # Plain text without colors")]
    Screenshot {
        /// Detect interactive elements and include element refs
        #[arg(short = 'e', long, help_heading = "Element Detection")]
        elements: bool,

        /// Output in accessibility-tree format (agent-browser style)
        #[arg(short = 'a', long, help_heading = "Element Detection")]
        accessibility: bool,

        /// Only include interactive elements (requires --accessibility)
        #[arg(long, requires = "accessibility", help_heading = "Element Detection")]
        interactive_only: bool,

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

    /// Perform an action on an element by reference
    #[command(long_about = "\
Perform an action on an element by reference.

All element interactions are done through this command. Specify the element
reference and the operation to perform. If no operation is specified,
defaults to click.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui action @e1
    agent-tui action @e1 click
    agent-tui action @e1 fill \"my-project\"
    agent-tui action @sel1 select \"Option 2\"
    agent-tui action @cb1 toggle on
    agent-tui action @e1 scroll up 10")]
    Action {
        /// Element ref to target (e.g., @e1)
        #[arg(value_name = "REF")]
        element_ref: String,

        #[command(subcommand)]
        operation: Option<ActionOperation>,
    },

    /// Send key press(es) to the terminal
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui press Enter
    agent-tui press Ctrl+C
    agent-tui press ArrowDown ArrowDown Enter")]
    Press {
        /// Keys to press (e.g., Enter, Ctrl+C, ArrowDown)
        #[arg(required = true, value_name = "KEY")]
        keys: Vec<String>,
    },

    /// Type literal text character by character
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui type \"hello world\"
    agent-tui type \"user@example.com\"")]
    Type {
        /// Text to type
        #[arg(value_name = "TEXT")]
        text: String,
    },

    /// Send keyboard input (keys or text)
    #[command(long_about = "\
Send keyboard input - keys or text.

Unified command for all keyboard input. Automatically detects whether
the input is a key name or text to type.

SUPPORTED KEYS: Enter, Tab, Escape, Backspace, Delete, Arrow keys, Home, End, PageUp, PageDown, F1-F12
MODIFIERS: Ctrl+<key>, Alt+<key>, Shift+<key>

If the input matches a known key name, it's sent as a key press.
Otherwise, it's typed as text character by character.
Use quotes for text that might be mistaken for a key name.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui input Enter              # Press Enter
    agent-tui input Ctrl+C             # Press Ctrl+C
    agent-tui input \"hello\"            # Type text char-by-char
    agent-tui input Shift --hold       # Hold Shift down")]
    Input {
        /// Key name or text to send
        #[arg(value_name = "KEY|TEXT")]
        value: String,

        /// Hold the key down (for modifier sequences)
        #[arg(long, conflicts_with = "release", help_heading = "Modifiers")]
        hold: bool,

        /// Release a held key (for modifier sequences)
        #[arg(long, conflicts_with = "hold", help_heading = "Modifiers")]
        release: bool,
    },

    /// Wait for text, element, or screenshot stability
    #[command(long_about = "\
Wait for a condition to be met before continuing.

Waits for text to appear, elements to change, or the screenshot to stabilize.
Returns success if the condition is met within the timeout period.

WAIT CONDITIONS:
    <text>              Wait for text to appear on screenshot
    -e, --element <ref> Wait for element to appear
    --focused <ref>     Wait for element to be focused
    --stable            Wait for screenshot to stop changing
    --value <ref>=<val> Wait for input to have specific value
    -g, --gone          Modifier: wait for element/text to disappear

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 1 if timeout.
                        Without --assert, always exit 0 (timeout still reported).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui wait \"Continue\"           # Wait for text
    agent-tui wait -e @btn1             # Wait for element
    agent-tui wait -e @spinner --gone   # Wait for element to disappear
    agent-tui wait --stable             # Wait for screenshot stability
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
Manage sessions - list, show details, attach, cleanup, or status.

By default, lists all active sessions.

MODES:
    list              List active sessions (default)
    show <id>         Show details for a session
    attach [id]       Attach with TTY (defaults to --session or active)
    cleanup [--all]   Remove dead/orphaned sessions
    status            Show daemon health")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions                    # List sessions
    agent-tui sessions list               # List sessions (explicit)
    agent-tui sessions show abc123        # Show session details
    agent-tui sessions attach             # Attach to active session (TTY)
    agent-tui sessions attach abc123      # Attach to session by id (TTY)
    agent-tui sessions attach -T abc123   # Attach without TTY (stream output only)
    agent-tui sessions attach --detach-keys 'ctrl-]'  # Custom detach sequence
    agent-tui sessions cleanup            # Remove dead sessions
    agent-tui sessions cleanup --all      # Remove all sessions
    agent-tui sessions status             # Show daemon health")]
    Sessions {
        #[command(subcommand)]
        command: Option<SessionsCommand>,
    },

    /// Live preview gateway for the current session
    #[command(long_about = "\
Start or manage the live preview gateway.

By default, starts a live preview gateway for the active session.
The gateway serves a browser page with a live terminal view using the daemon stream.

REQUIREMENTS:
    The gateway is a Node/Bun script shipped with the repo. If auto-discovery
    fails, set AGENT_TUI_LIVE_GATEWAY to the script path.

SECURITY:
    Only binds to loopback by default. Use --allow-remote to bind to
    non-loopback addresses.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui live start
    agent-tui live start -l 127.0.0.1:0
    agent-tui live start -l 0.0.0.0:9999 --allow-remote
    agent-tui live status
    agent-tui live stop")]
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
Useful for debugging and ensuring CLI/daemon compatibility.")]
    Version,

    /// Show environment diagnostics
    #[command(long_about = "\
Show environment diagnostics.

Displays all environment variables and configuration that affect
agent-tui behavior. Useful for debugging connection issues.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui env
    agent-tui --format json env")]
    Env,

    /// Catch-all for element refs (@e1) and text selectors (:Submit)
    #[command(external_subcommand)]
    External(Vec<String>),

    /// Generate shell completions
    #[command(long_about = "\
Generate shell completion scripts for bash, zsh, fish, powershell, or elvish.")]
    #[command(after_long_help = "\
INSTALLATION:
    # Bash - add to ~/.bashrc
    source <(agent-tui completions bash)

    # Zsh - add to ~/.zshrc
    source <(agent-tui completions zsh)

    # Fish - run once
    agent-tui completions fish > ~/.config/fish/completions/agent-tui.fish

    # PowerShell - add to $PROFILE
    agent-tui completions powershell | Out-String | Invoke-Expression")]
    Completions {
        #[arg(value_enum, value_name = "SHELL")]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommand {
    /// List active sessions
    List,

    /// Show details for a specific session
    Show {
        #[arg(value_name = "ID")]
        session_id: String,
    },

    /// Attach to a session (TTY by default; detach with Ctrl-P Ctrl-Q or --detach-keys)
    Attach {
        #[arg(value_name = "ID")]
        session_id: Option<String>,
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

    /// Remove dead/orphaned sessions
    Cleanup {
        /// Remove all sessions (including active)
        #[arg(long)]
        all: bool,
    },

    /// Show daemon health status
    Status,
}

#[derive(Debug, Subcommand)]
pub enum LiveCommand {
    /// Start the live preview gateway
    Start(LiveStartArgs),

    /// Stop the live preview gateway
    Stop,

    /// Show live preview status
    Status,
}

#[derive(Debug, Clone, Default, Args)]
pub struct LiveStartArgs {
    /// Listen address for the live preview gateway (default: 127.0.0.1:0)
    #[arg(
        short = 'l',
        long,
        value_name = "ADDR",
        num_args = 0..=1,
        default_missing_value = "127.0.0.1:0"
    )]
    pub listen: Option<String>,

    /// Allow binding to non-loopback addresses (e.g., 0.0.0.0)
    #[arg(long)]
    pub allow_remote: bool,

    /// Open the preview URL in a browser
    #[arg(long)]
    pub open: bool,

    /// Browser command to use (overrides $BROWSER)
    #[arg(long, value_name = "CMD")]
    pub browser: Option<String>,

    /// Maximum concurrent live preview viewers (default: 3)
    #[arg(long, value_name = "COUNT", env = "AGENT_TUI_LIVE_MAX_VIEWERS")]
    pub max_viewers: Option<u16>,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum DaemonCommand {
    /// Start the daemon process
    #[command(long_about = "\
Start the daemon process.

By default, starts the daemon in the background. Use --foreground to run
in the current terminal (useful for debugging).")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui daemon start              # Start in background
    agent-tui daemon start --foreground # Run in foreground")]
    Start {
        /// Run in the foreground (debugging)
        #[arg(long)]
        foreground: bool,
    },

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

    /// Show daemon status and version
    #[command(long_about = "\
Show daemon status and version information.

Displays whether the daemon is running, its PID, uptime, and version.
Also checks for version mismatch between CLI and daemon.

EXIT CODES (following LSB init script conventions):
    0 - Daemon is running and healthy
    3 - Daemon is not running")]
    Status,

    /// Restart the daemon
    #[command(long_about = "\
Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.")]
    Restart,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ActionOperation {
    /// Click the element
    Click,

    /// Double-click the element
    #[command(name = "dblclick")]
    DblClick,

    /// Set the input value
    Fill {
        /// Text to fill into the element
        #[arg(value_name = "VALUE")]
        value: String,
    },

    /// Select option(s) from a list
    Select {
        /// Options to select
        #[arg(required = true, value_name = "OPTION")]
        options: Vec<String>,
    },

    /// Toggle checkbox/radio state
    Toggle {
        /// Desired state (on/off). If omitted, toggle current state.
        #[arg(value_enum)]
        state: Option<ToggleState>,
    },

    /// Set focus to the element
    Focus,

    /// Clear the input value
    Clear,

    /// Select all text in input
    #[command(name = "selectall")]
    SelectAll,

    /// Scroll viewport in a direction
    Scroll {
        /// Direction to scroll
        #[arg(value_enum)]
        direction: ScrollDirection,

        /// Number of lines/rows to scroll
        #[arg(default_value_t = 5, value_name = "AMOUNT")]
        amount: u16,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ToggleState {
    On,
    Off,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            ScrollDirection::Up => "up",
            ScrollDirection::Down => "down",
            ScrollDirection::Left => "left",
            ScrollDirection::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Default, Args)]
#[command(
    group = ArgGroup::new("wait_condition")
        .multiple(false)
        .required(true)
        .args(&["text", "element", "focused", "stable", "value"])
)]
#[command(group = ArgGroup::new("gone_target").args(&["text", "element"]))]
pub struct WaitParams {
    /// Text to wait for (positional)
    #[arg(value_name = "TEXT")]
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

    /// Wait for an element ref to appear
    #[arg(
        short = 'e',
        long,
        group = "wait_condition",
        value_name = "REF",
        help_heading = "Wait Condition"
    )]
    pub element: Option<String>,

    /// Wait for an element to be focused
    #[arg(
        long,
        group = "wait_condition",
        value_name = "REF",
        help_heading = "Wait Condition"
    )]
    pub focused: Option<String>,

    /// Wait for the screenshot to stop changing
    #[arg(long, group = "wait_condition", help_heading = "Wait Condition")]
    pub stable: bool,

    /// Wait for an input to have a specific value
    #[arg(
        long,
        group = "wait_condition",
        value_name = "REF=VALUE",
        help_heading = "Wait Condition"
    )]
    pub value: Option<String>,

    /// Wait for the target to disappear (text or element)
    #[arg(
        short = 'g',
        long,
        requires = "gone_target",
        help_heading = "Wait Condition"
    )]
    pub gone: bool,

    /// Exit with status 0 if met, 1 on timeout
    #[arg(long, help_heading = "Behavior")]
    pub assert: bool,
}

#[derive(Debug, Clone, Default, Args)]
pub struct FindParams {
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,

    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,

    #[arg(long, value_name = "TEXT")]
    pub placeholder: Option<String>,

    #[arg(long)]
    pub focused: bool,

    #[arg(long, value_name = "N")]
    pub nth: Option<usize>,

    #[arg(long)]
    pub exact: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum, Default, PartialEq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_defaults() {
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        assert!(cli.session.is_none());
        assert_eq!(cli.format, OutputFormat::Text);
        assert!(!cli.no_color);
        assert!(!cli.verbose);
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
            "--verbose",
            "sessions",
        ]);
        assert_eq!(cli.session, Some("my-session".to_string()));
        assert_eq!(cli.format, OutputFormat::Json);
        assert!(cli.no_color);
        assert!(cli.verbose);
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
        let cli = Cli::parse_from(["agent-tui", "screenshot", "-e"]);
        let Commands::Screenshot {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Screenshot command, got {:?}", cli.command);
        };
        assert!(elements, "-i should enable elements");
        assert!(region.is_none());
        assert!(!strip_ansi);
        assert!(!include_cursor);
    }

    #[test]
    fn test_screenshot_all_flags() {
        let cli = Cli::parse_from([
            "agent-tui",
            "screenshot",
            "-e",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ]);
        let Commands::Screenshot {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Screenshot command, got {:?}", cli.command);
        };
        assert!(elements);
        assert_eq!(region, Some("modal".to_string()));
        assert!(strip_ansi);
        assert!(include_cursor);
    }

    #[test]
    fn test_screenshot_accessibility_flag() {
        let cli = Cli::parse_from(["agent-tui", "screenshot", "-a"]);
        let Commands::Screenshot {
            accessibility,
            elements,
            ..
        } = cli.command
        else {
            panic!("Expected Screenshot command, got {:?}", cli.command);
        };
        assert!(accessibility, "-a should enable accessibility tree format");
        assert!(!elements, "elements should be false by default");
    }

    #[test]
    fn test_screenshot_accessibility_interactive_only() {
        let cli = Cli::parse_from(["agent-tui", "screenshot", "-a", "--interactive-only"]);
        let Commands::Screenshot {
            accessibility,
            interactive_only,
            ..
        } = cli.command
        else {
            panic!("Expected Screenshot command, got {:?}", cli.command);
        };
        assert!(accessibility, "--accessibility should be set");
        assert!(
            interactive_only,
            "--interactive-only should filter to interactive elements"
        );
    }

    #[test]
    fn test_action_click() {
        let cli = Cli::parse_from(["agent-tui", "action", "@btn1", "click"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
        assert!(matches!(operation, Some(ActionOperation::Click)));
    }

    #[test]
    fn test_action_dblclick() {
        let cli = Cli::parse_from(["agent-tui", "action", "@btn1", "dblclick"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
        assert!(matches!(operation, Some(ActionOperation::DblClick)));
    }

    #[test]
    fn test_action_fill() {
        let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "fill", "test value"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
        let Some(ActionOperation::Fill { value }) = operation else {
            panic!("Expected Fill operation, got {:?}", operation);
        };
        assert_eq!(value, "test value");
    }

    #[test]
    fn test_action_select_single() {
        let cli = Cli::parse_from(["agent-tui", "action", "@sel1", "select", "Option 1"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@sel1");
        let Some(ActionOperation::Select { options }) = operation else {
            panic!("Expected Select operation, got {:?}", operation);
        };
        assert_eq!(options, vec!["Option 1"]);
    }

    #[test]
    fn test_action_select_multiple() {
        let cli = Cli::parse_from([
            "agent-tui",
            "action",
            "@list1",
            "select",
            "red",
            "blue",
            "green",
        ]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@list1");
        let Some(ActionOperation::Select { options }) = operation else {
            panic!("Expected Select operation, got {:?}", operation);
        };
        assert_eq!(options, vec!["red", "blue", "green"]);
    }

    #[test]
    fn test_action_toggle() {
        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        let Some(ActionOperation::Toggle { state }) = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(state.is_none());

        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle", "on"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let Some(ActionOperation::Toggle { state }) = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(matches!(state, Some(ToggleState::On)));

        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle", "off"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let Some(ActionOperation::Toggle { state }) = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(matches!(state, Some(ToggleState::Off)));
    }

    #[test]
    fn test_action_focus() {
        let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "focus"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
        assert!(matches!(operation, Some(ActionOperation::Focus)));
    }

    #[test]
    fn test_action_clear() {
        let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "clear"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
        assert!(matches!(operation, Some(ActionOperation::Clear)));
    }

    #[test]
    fn test_action_selectall() {
        let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "selectall"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
        assert!(matches!(operation, Some(ActionOperation::SelectAll)));
    }

    #[test]
    fn test_action_scroll() {
        let cli = Cli::parse_from(["agent-tui", "action", "@e1", "scroll", "up"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@e1");
        let Some(ActionOperation::Scroll { direction, amount }) = operation else {
            panic!("Expected Scroll operation, got {:?}", operation);
        };
        assert!(matches!(direction, ScrollDirection::Up));
        assert_eq!(amount, 5);

        let cli = Cli::parse_from(["agent-tui", "action", "@e1", "scroll", "down", "10"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let Some(ActionOperation::Scroll { direction, amount }) = operation else {
            panic!("Expected Scroll operation, got {:?}", operation);
        };
        assert!(matches!(direction, ScrollDirection::Down));
        assert_eq!(amount, 10);
    }

    #[test]
    fn test_input_command_keys() {
        let test_cases = vec![
            "Enter",
            "Tab",
            "Escape",
            "Backspace",
            "Delete",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "F1",
            "F10",
            "F12",
            "Ctrl+C",
            "Alt+F4",
            "Shift+Tab",
        ];

        for k in test_cases {
            let cli = Cli::parse_from(["agent-tui", "input", k]);
            let Commands::Input {
                value,
                hold,
                release,
            } = cli.command
            else {
                panic!("Expected Input command for: {k}, got {:?}", cli.command);
            };
            assert_eq!(value, k.to_string());
            assert!(!hold);
            assert!(!release);
        }
    }

    #[test]
    fn test_input_command_text() {
        let cli = Cli::parse_from(["agent-tui", "input", "Hello, World!"]);
        let Commands::Input { value, .. } = cli.command else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, "Hello, World!".to_string());

        let cli = Cli::parse_from(["agent-tui", "input", "hello"]);
        let Commands::Input { value, .. } = cli.command else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, "hello".to_string());
    }

    #[test]
    fn test_input_hold_command() {
        let cli = Cli::parse_from(["agent-tui", "input", "Shift", "--hold"]);
        let Commands::Input {
            value,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, "Shift".to_string());
        assert!(hold);
        assert!(!release);
    }

    #[test]
    fn test_input_release_command() {
        let cli = Cli::parse_from(["agent-tui", "input", "Shift", "--release"]);
        let Commands::Input {
            value,
            hold,
            release,
        } = cli.command
        else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, "Shift".to_string());
        assert!(!hold);
        assert!(release);
    }

    #[test]
    fn test_input_flag_conflicts() {
        assert!(
            Cli::try_parse_from(["agent-tui", "input", "Shift", "--hold", "--release"]).is_err()
        );
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
    fn test_wait_stable() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--stable"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(params.stable);
        assert!(params.text.is_none());
    }

    #[test]
    fn test_wait_element() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--element", "@btn1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@btn1".to_string()));
        assert!(params.text.is_none());
    }

    #[test]
    fn test_wait_element_short_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@btn1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@btn1".to_string()));
        assert!(!params.gone);
    }

    #[test]
    fn test_wait_focused() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--focused", "@inp1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.focused, Some("@inp1".to_string()));
        assert!(params.text.is_none());
    }

    #[test]
    fn test_wait_element_gone() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@spinner", "--gone"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@spinner".to_string()));
        assert!(params.gone);
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
    fn test_wait_gone_short_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@spinner", "-g"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@spinner".to_string()));
        assert!(params.gone);
    }

    #[test]
    fn test_wait_value() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--value", "@inp1=hello"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.value, Some("@inp1=hello".to_string()));
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
        assert!(Cli::try_parse_from(["agent-tui", "action"]).is_err());
        let cli = Cli::parse_from(["agent-tui", "action", "@e1"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@e1");
        assert!(operation.is_none());

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
    fn test_sessions_list_explicit() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "list"]);
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
                session_id: None,
                no_tty: false,
                detach_keys: None
            })
        ));
    }

    #[test]
    fn test_sessions_attach_with_id() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "attach", "my-session"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Attach {
                session_id: Some(_),
                no_tty: false,
                detach_keys: None
            })
        ));
    }

    #[test]
    fn test_sessions_attach_no_tty() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "attach", "-T", "my-session"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(
            command,
            Some(SessionsCommand::Attach {
                session_id: Some(_),
                no_tty: true,
                detach_keys: None
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
    fn test_sessions_status() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "status"]);
        let Commands::Sessions { command } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(matches!(command, Some(SessionsCommand::Status)));
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
        let Commands::Completions { shell } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Shell::Bash));
    }

    #[test]
    fn test_completions_fish() {
        let cli = Cli::parse_from(["agent-tui", "completions", "fish"]);
        let Commands::Completions { shell } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Shell::Fish));
    }

    #[test]
    fn test_daemon_start_default() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "start"]);
        let Commands::Daemon(DaemonCommand::Start { foreground }) = cli.command else {
            panic!("Expected Daemon Start command, got {:?}", cli.command);
        };
        assert!(!foreground, "Default should be background mode");
    }

    #[test]
    fn test_daemon_start_foreground() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "start", "--foreground"]);
        let Commands::Daemon(DaemonCommand::Start { foreground }) = cli.command else {
            panic!("Expected Daemon Start command, got {:?}", cli.command);
        };
        assert!(foreground, "Should be foreground mode");
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
    fn test_daemon_status() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "status"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommand::Status)
        ));
    }

    #[test]
    fn test_daemon_restart() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "restart"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommand::Restart)
        ));
    }

    // Note: With external_subcommand, unknown commands are captured as External.
    // These "removed" commands will be caught at runtime with validation errors.
    #[test]
    fn test_count_command_becomes_external() {
        let cli = Cli::parse_from(["agent-tui", "count", "--role", "button"]);
        assert!(
            matches!(cli.command, Commands::External(_)),
            "Unknown command should be captured as External"
        );
    }

    #[test]
    fn test_find_command_becomes_external() {
        let cli = Cli::parse_from(["agent-tui", "find", "--role", "button"]);
        assert!(
            matches!(cli.command, Commands::External(_)),
            "Unknown command should be captured as External"
        );
    }

    #[test]
    fn test_restart_command_becomes_external() {
        let cli = Cli::parse_from(["agent-tui", "restart"]);
        assert!(
            matches!(cli.command, Commands::External(_)),
            "Unknown command should be captured as External"
        );
    }

    #[test]
    fn test_resize_command_becomes_external() {
        let cli = Cli::parse_from(["agent-tui", "resize", "--cols", "80"]);
        assert!(
            matches!(cli.command, Commands::External(_)),
            "Unknown command should be captured as External"
        );
    }

    // Phase 1: Press and Type commands
    #[test]
    fn test_press_enter_command() {
        let cli = Cli::parse_from(["agent-tui", "press", "Enter"]);
        let Commands::Press { keys } = cli.command else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "Enter");
    }

    #[test]
    fn test_press_key_sequence() {
        let cli = Cli::parse_from(["agent-tui", "press", "ArrowDown", "ArrowDown", "Enter"]);
        let Commands::Press { keys } = cli.command else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "ArrowDown");
        assert_eq!(keys[1], "ArrowDown");
        assert_eq!(keys[2], "Enter");
    }

    #[test]
    fn test_press_with_modifier() {
        let cli = Cli::parse_from(["agent-tui", "press", "Ctrl+C"]);
        let Commands::Press { keys } = cli.command else {
            panic!("Expected Press command, got {:?}", cli.command);
        };
        assert_eq!(keys[0], "Ctrl+C");
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
    fn test_type_command_with_spaces() {
        let cli = Cli::parse_from(["agent-tui", "type", "Hello, World!"]);
        let Commands::Type { text } = cli.command else {
            panic!("Expected Type command, got {:?}", cli.command);
        };
        assert_eq!(text, "Hello, World!");
    }

    // Phase 2: Element ref as command (@e1)
    #[test]
    fn test_element_ref_activate() {
        let cli = Cli::parse_from(["agent-tui", "@e1"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], "@e1");
    }

    #[test]
    fn test_element_ref_fill() {
        let cli = Cli::parse_from(["agent-tui", "@e1", "my-project"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "@e1");
        assert_eq!(args[1], "my-project");
    }

    #[test]
    fn test_element_ref_toggle() {
        let cli = Cli::parse_from(["agent-tui", "@e1", "toggle"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "@e1");
        assert_eq!(args[1], "toggle");
    }

    #[test]
    fn test_element_ref_toggle_on() {
        let cli = Cli::parse_from(["agent-tui", "@e1", "toggle", "on"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args.len(), 3);
        assert_eq!(args[2], "on");
    }

    #[test]
    fn test_element_ref_choose() {
        let cli = Cli::parse_from(["agent-tui", "@e1", "choose", "Option 1"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "@e1");
        assert_eq!(args[1], "choose");
        assert_eq!(args[2], "Option 1");
    }

    // Phase 3: Text selectors
    #[test]
    fn test_text_selector_exact() {
        let cli = Cli::parse_from(["agent-tui", "@Yes, proceed"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args[0], "@Yes, proceed");
    }

    #[test]
    fn test_text_selector_partial() {
        let cli = Cli::parse_from(["agent-tui", ":Submit"]);
        let Commands::External(args) = cli.command else {
            panic!("Expected External command, got {:?}", cli.command);
        };
        assert_eq!(args[0], ":Submit");
    }
}
