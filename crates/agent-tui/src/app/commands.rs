use clap::ArgGroup;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use clap::ValueHint;
pub use clap_complete::Shell;
use std::path::PathBuf;

const AFTER_LONG_HELP: &str = r#"WORKFLOW:
    1. Run a TUI application
    2. View the screen and detect elements
    3. Interact with elements or press keys
    4. Wait for UI changes
    5. Kill the session when done

SELECTORS:
    @e1, @e2, @e3  - Element refs (from 'screen -e' output)
    @"Submit"      - Find element by exact text
    :Submit        - Find element by partial text (contains)

EXAMPLES:
    # Start and interact with a TUI app
    agent-tui run "npx create-next-app"
    agent-tui screen -e
    agent-tui @e1 "my-project"           # Fill input with value
    agent-tui press Enter                 # Press Enter key
    agent-tui wait "success"
    agent-tui kill

    # Navigate menus efficiently
    agent-tui run htop
    agent-tui press F10
    agent-tui screen -e
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
#[command(after_long_help = AFTER_LONG_HELP)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, value_name = "ID")]
    pub session: Option<String>,

    #[arg(
        short,
        long,
        global = true,
        value_enum,
        value_name = "FORMAT",
        default_value_t = OutputFormat::Text
    )]
    pub format: OutputFormat,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    #[arg(short, long, global = true)]
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
    #[command(visible_alias = "spawn")]
    #[command(long_about = "\
Run a new TUI application in a virtual terminal.

Creates a new PTY session with the specified command and returns a session ID.
The session runs in the background and can be interacted with using other commands.")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui run bash
    agent-tui run htop
    agent-tui run \"npx create-next-app\"
    agent-tui run vim -- file.txt
    agent-tui run --cols 80 --rows 24 nano")]
    Run {
        #[arg(value_name = "COMMAND", value_hint = ValueHint::CommandName)]
        command: String,

        #[arg(trailing_var_arg = true, value_name = "ARGS")]
        args: Vec<String>,

        #[arg(short = 'd', long, value_name = "DIR", value_hint = ValueHint::DirPath)]
        cwd: Option<PathBuf>,

        #[arg(long, default_value_t = 120)]
        cols: u16,

        #[arg(long, default_value_t = 40)]
        rows: u16,
    },

    /// View screen content and detect UI elements
    #[command(visible_alias = "snapshot")]
    #[command(long_about = "\
View the current screen state.

Returns the current terminal screen content and optionally detects
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
    agent-tui screen              # Just the screen
    agent-tui screen -e           # Screen + detected elements
    agent-tui screen -a           # Accessibility tree format
    agent-tui screen -a --interactive-only  # Only interactive elements
    agent-tui screen --strip-ansi # Plain text without colors")]
    Screen {
        #[arg(short = 'i', long)]
        elements: bool,

        #[arg(short = 'a', long)]
        accessibility: bool,

        #[arg(long, requires = "accessibility")]
        interactive_only: bool,

        #[arg(long, value_name = "REGION")]
        region: Option<String>,

        #[arg(long)]
        strip_ansi: bool,

        #[arg(long)]
        include_cursor: bool,
    },

    /// Perform an action on an element by reference
    #[command(visible_alias = "click")]
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
        #[arg(value_name = "REF")]
        element_ref: String,

        #[command(subcommand)]
        operation: Option<ActionOperation>,
    },

    /// Send key press(es) to the terminal
    #[command(visible_alias = "keystroke")]
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
    Type {
        /// Text to type
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
        #[arg(value_name = "KEY|TEXT")]
        value: String,

        #[arg(long, conflicts_with = "release")]
        hold: bool,

        #[arg(long, conflicts_with = "hold")]
        release: bool,
    },

    /// Wait for text, element, or screen stability
    #[command(long_about = "\
Wait for a condition to be met before continuing.

Waits for text to appear, elements to change, or the screen to stabilize.
Returns success if the condition is met within the timeout period.

WAIT CONDITIONS:
    <text>              Wait for text to appear on screen
    -e, --element <ref> Wait for element to appear
    --focused <ref>     Wait for element to be focused
    --stable            Wait for screen to stop changing
    --value <ref>=<val> Wait for input to have specific value
    -g, --gone          Modifier: wait for element/text to disappear

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 1 if timeout")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui wait \"Continue\"           # Wait for text
    agent-tui wait -e @btn1             # Wait for element
    agent-tui wait -e @spinner --gone   # Wait for element to disappear
    agent-tui wait --stable             # Wait for screen stability
    agent-tui wait -t 5000 \"Done\"       # 5 second timeout")]
    Wait {
        #[command(flatten)]
        params: WaitParams,
    },

    /// Kill the current session
    Kill,

    /// List and manage sessions
    #[command(long_about = "\
Manage sessions - list, cleanup, attach, or show details.

By default, lists all active sessions. Use flags for other operations.

FLAGS:
    <id>           Show details for a specific session
    --cleanup      Remove dead/orphaned sessions
    --cleanup --all    Remove all sessions
    --attach <id>  Attach to a session (interactive mode)
    --status       Include daemon health in output")]
    #[command(after_long_help = "\
EXAMPLES:
    agent-tui sessions                    # List sessions
    agent-tui sessions abc123             # Show session details
    agent-tui sessions --cleanup          # Remove dead sessions
    agent-tui sessions --attach abc123    # Attach interactively")]
    #[command(
        group = ArgGroup::new("sessions_mode")
            .multiple(false)
            .args(&["id", "cleanup", "attach", "status"])
    )]
    Sessions {
        #[arg(name = "id", value_name = "ID")]
        session_id: Option<String>,

        #[arg(long)]
        cleanup: bool,

        #[arg(long, requires = "cleanup")]
        all: bool,

        #[arg(long, value_name = "ID")]
        attach: Option<String>,

        #[arg(long)]
        status: bool,
    },
    #[command(subcommand, hide = true)]
    Debug(DebugCommand),

    #[command(hide = true)]
    RecordStart,

    #[command(hide = true)]
    RecordStop(RecordStopArgs),

    #[command(hide = true)]
    RecordStatus,

    #[command(hide = true)]
    Trace(TraceArgs),

    #[command(hide = true)]
    Console(ConsoleArgs),

    #[command(hide = true)]
    Errors(ErrorsArgs),

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
        #[arg(value_name = "VALUE")]
        value: String,
    },

    /// Select option(s) from a list
    Select {
        #[arg(required = true, value_name = "OPTION")]
        options: Vec<String>,
    },

    /// Toggle checkbox/radio state
    Toggle {
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
        #[arg(value_enum)]
        direction: ScrollDirection,

        #[arg(default_value_t = 5)]
        amount: u16,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ToggleState {
    On,
    Off,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum DebugCommand {
    #[command(subcommand)]
    Record(RecordAction),

    Trace(TraceArgs),

    Console(ConsoleArgs),

    Errors(ErrorsArgs),

    Env,
}

#[derive(Debug, Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum RecordAction {
    Start,

    Stop(RecordStopArgs),

    Status,
}

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
pub enum RecordFormat {
    #[default]
    Json,
    Asciicast,
}

impl RecordFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordFormat::Json => "json",
            RecordFormat::Asciicast => "asciicast",
        }
    }
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

#[derive(Debug, Clone, Args)]
pub struct RecordStopArgs {
    #[arg(short, long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    #[arg(
        long = "record-format",
        value_enum,
        default_value_t = RecordFormat::Json,
        value_name = "FORMAT"
    )]
    pub record_format: RecordFormat,
}

#[derive(Debug, Clone, Args)]
pub struct TraceArgs {
    #[arg(short = 'n', long, default_value_t = 10)]
    pub count: usize,

    #[arg(long)]
    pub start: bool,

    #[arg(long)]
    pub stop: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ConsoleArgs {
    #[arg(short = 'n', long, default_value_t = 100)]
    pub lines: usize,

    #[arg(long)]
    pub clear: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ErrorsArgs {
    #[arg(short = 'n', long, default_value_t = 10)]
    pub count: usize,

    #[arg(long)]
    pub clear: bool,
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
    #[arg(value_name = "TEXT")]
    pub text: Option<String>,

    #[arg(short, long, default_value_t = 30_000, value_name = "MILLIS")]
    pub timeout: u64,

    #[arg(short = 'e', long, group = "wait_condition", value_name = "REF")]
    pub element: Option<String>,

    #[arg(long, group = "wait_condition", value_name = "REF")]
    pub focused: Option<String>,

    #[arg(long, group = "wait_condition")]
    pub stable: bool,

    #[arg(long, group = "wait_condition", value_name = "REF=VALUE")]
    pub value: Option<String>,

    #[arg(short = 'g', long, requires = "gone_target")]
    pub gone: bool,

    #[arg(long)]
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
    fn test_screen_flags() {
        let cli = Cli::parse_from(["agent-tui", "screen", "-i"]);
        let Commands::Screen {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Screen command, got {:?}", cli.command);
        };
        assert!(elements, "-i should enable elements");
        assert!(region.is_none());
        assert!(!strip_ansi);
        assert!(!include_cursor);
    }

    #[test]
    fn test_screen_all_flags() {
        let cli = Cli::parse_from([
            "agent-tui",
            "screen",
            "-i",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ]);
        let Commands::Screen {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Screen command, got {:?}", cli.command);
        };
        assert!(elements);
        assert_eq!(region, Some("modal".to_string()));
        assert!(strip_ansi);
        assert!(include_cursor);
    }

    #[test]
    fn test_screen_accessibility_flag() {
        let cli = Cli::parse_from(["agent-tui", "screen", "-a"]);
        let Commands::Screen {
            accessibility,
            elements,
            ..
        } = cli.command
        else {
            panic!("Expected Screen command, got {:?}", cli.command);
        };
        assert!(accessibility, "-a should enable accessibility tree format");
        assert!(!elements, "elements should be false by default");
    }

    #[test]
    fn test_screen_accessibility_interactive_only() {
        let cli = Cli::parse_from(["agent-tui", "screen", "-a", "--interactive-only"]);
        let Commands::Screen {
            accessibility,
            interactive_only,
            ..
        } = cli.command
        else {
            panic!("Expected Screen command, got {:?}", cli.command);
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
    fn test_trace_defaults() {
        let cli = Cli::parse_from(["agent-tui", "trace"]);
        let Commands::Trace(args) = cli.command else {
            panic!("Expected Trace command, got {:?}", cli.command);
        };

        assert_eq!(args.count, 10, "Default trace count should be 10");
        assert!(!args.start);
        assert!(!args.stop);
    }

    #[test]
    fn test_console_defaults() {
        let cli = Cli::parse_from(["agent-tui", "console"]);
        let Commands::Console(args) = cli.command else {
            panic!("Expected Console command, got {:?}", cli.command);
        };

        assert_eq!(args.lines, 100, "Default console lines should be 100");
        assert!(!args.clear, "Default clear should be false");
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
    fn test_errors_command_with_count() {
        let cli = Cli::parse_from(["agent-tui", "errors", "-n", "25"]);
        let Commands::Errors(args) = cli.command else {
            panic!("Expected Errors command, got {:?}", cli.command);
        };
        assert_eq!(args.count, 25);
        assert!(!args.clear);
    }

    #[test]
    fn test_errors_command_with_clear() {
        let cli = Cli::parse_from(["agent-tui", "errors", "--clear"]);
        let Commands::Errors(args) = cli.command else {
            panic!("Expected Errors command, got {:?}", cli.command);
        };
        assert!(args.clear);
    }

    #[test]
    fn test_sessions_list() {
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        let Commands::Sessions {
            session_id,
            cleanup,
            all,
            attach,
            status,
        } = cli.command
        else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(session_id.is_none());
        assert!(!cleanup);
        assert!(!all);
        assert!(attach.is_none());
        assert!(!status);
    }

    #[test]
    fn test_sessions_attach() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--attach", "my-session"]);
        let Commands::Sessions { attach, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert_eq!(attach, Some("my-session".to_string()));
    }

    #[test]
    fn test_sessions_cleanup() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--cleanup"]);
        let Commands::Sessions { cleanup, all, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(cleanup);
        assert!(!all);
    }

    #[test]
    fn test_sessions_cleanup_all() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--cleanup", "--all"]);
        let Commands::Sessions { cleanup, all, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(cleanup);
        assert!(all);
    }

    #[test]
    fn test_sessions_status() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--status"]);
        let Commands::Sessions { status, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(status);
    }

    #[test]
    fn test_sessions_with_id() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "abc123"]);
        let Commands::Sessions { session_id, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert_eq!(session_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_record_start_command() {
        let cli = Cli::parse_from(["agent-tui", "record-start"]);
        assert!(matches!(cli.command, Commands::RecordStart));
    }

    #[test]
    fn test_record_stop_command() {
        let cli = Cli::parse_from(["agent-tui", "record-stop"]);
        let Commands::RecordStop(args) = cli.command else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert!(args.output.is_none());
        assert!(matches!(args.record_format, RecordFormat::Json));
    }

    #[test]
    fn test_record_stop_with_output() {
        let cli = Cli::parse_from(["agent-tui", "record-stop", "-o", "recording.json"]);
        let Commands::RecordStop(args) = cli.command else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert_eq!(args.output, Some(PathBuf::from("recording.json")));
    }

    #[test]
    fn test_record_stop_asciicast_format() {
        let cli = Cli::parse_from(["agent-tui", "record-stop", "--record-format", "asciicast"]);
        let Commands::RecordStop(args) = cli.command else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert!(matches!(args.record_format, RecordFormat::Asciicast));
    }

    #[test]
    fn test_record_status_command() {
        let cli = Cli::parse_from(["agent-tui", "record-status"]);
        assert!(matches!(cli.command, Commands::RecordStatus));
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
