use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
pub use clap_complete::Shell;

const LONG_ABOUT: &str = r#"agent-tui enables AI agents to interact with TUI (Text User Interface) applications.

WORKFLOW:
    1. Run a TUI application
    2. View the screen and detect elements
    3. Interact using action and input commands
    4. Wait for UI changes
    5. Kill the session when done

ELEMENT REFS:
    Element refs are simple sequential identifiers like @e1, @e2, @e3 that
    you can use to interact with detected UI elements. Run 'agent-tui screen -e'
    to see available elements and their refs.

    @e1, @e2, @e3, ...  - Elements in document order (top-to-bottom, left-to-right)

    Refs reset on each screen view. Always use the latest screen's refs.

EXAMPLES:
    # Start a new Next.js project wizard
    agent-tui run "npx create-next-app"
    agent-tui screen -e
    agent-tui action @e1 fill "my-project"
    agent-tui input Enter
    agent-tui wait "success"
    agent-tui kill

    # Interactive menu navigation
    agent-tui run htop
    agent-tui input F10
    agent-tui screen -e
    agent-tui action @e1 click
    agent-tui kill

    # Check daemon status
    agent-tui daemon status"#;

#[derive(Parser)]
#[command(name = "agent-tui")]
#[command(author, version)]
#[command(about = "CLI tool for AI agents to interact with TUI applications")]
#[command(long_about = LONG_ABOUT)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Session ID to use (default: uses the most recent session)
    #[arg(short, long, global = true)]
    pub session: Option<String>,

    /// Output format
    #[arg(short, long, global = true, default_value = "text")]
    pub format: OutputFormat,

    /// Output as JSON (shorthand for --format json)
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable colored output (also respects NO_COLOR env var)
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Enable verbose output (shows request timing)
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

impl Cli {
    /// Returns the effective output format, considering --json shorthand.
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
    /// Run a new TUI application in a virtual terminal
    #[command(name = "run")]
    #[command(long_about = r#"Run a new TUI application in a virtual terminal.

Creates a new PTY session with the specified command and returns a session ID.
The session runs in the background and can be interacted with using other commands.

EXAMPLES:
    agent-tui run bash
    agent-tui run htop
    agent-tui run "npx create-next-app"
    agent-tui run vim -- file.txt
    agent-tui run --cols 80 --rows 24 nano"#)]
    Run {
        /// Command to execute (e.g., bash, htop, vim)
        command: String,

        /// Additional arguments for the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Working directory for the spawned process
        #[arg(short = 'd', long)]
        cwd: Option<String>,

        /// Terminal width in columns
        #[arg(long, default_value = "120")]
        cols: u16,

        /// Terminal height in rows
        #[arg(long, default_value = "40")]
        rows: u16,
    },

    /// View the current screen state
    #[command(name = "screen")]
    #[command(long_about = r#"View the current screen state.

Returns the current terminal screen content and optionally detects
interactive UI elements like buttons, inputs, and menus.

Element detection uses the Visual Object Model (VOM) which identifies
UI components based on visual styling (colors, backgrounds) rather than
text patterns. This provides reliable detection across different TUI frameworks.

ACCESSIBILITY TREE FORMAT (-a):
    Returns an agent-browser style accessibility tree with refs for elements:
    - button "Submit" [ref=e1]
    - textbox "Search" [ref=e2]

EXAMPLES:
    agent-tui screen              # Just the screen
    agent-tui screen -e           # Screen + detected elements
    agent-tui screen -a           # Accessibility tree format
    agent-tui screen -a --interactive-only  # Only interactive elements
    agent-tui screen --strip-ansi # Plain text without colors"#)]
    Screen {
        /// Include detected UI elements in output
        #[arg(short = 'i', long)]
        elements: bool,

        /// Output accessibility tree format (agent-browser style)
        #[arg(short = 'a', long)]
        accessibility: bool,

        /// Filter to interactive elements only (used with -a)
        #[arg(long)]
        interactive_only: bool,

        /// Limit snapshot to a named region
        #[arg(long)]
        region: Option<String>,

        /// Strip ANSI escape codes from output
        #[arg(long)]
        strip_ansi: bool,

        /// Include cursor position in output
        #[arg(long)]
        include_cursor: bool,
    },

    /// Perform an action on an element
    #[command(long_about = r#"Perform an action on an element by reference.

All element interactions are done through this command. Specify the element
reference and the operation to perform.

OPERATIONS:
    click           Click the element
    dblclick        Double-click the element
    fill <value>    Set the input value
    select <opt>    Select an option (multiple for multiselect)
    toggle [on|off] Toggle checkbox/radio (optionally force state)
    focus           Set focus to the element
    clear           Clear the input value
    selectall       Select all text in input
    scroll <dir> [n] Scroll viewport (up/down/left/right, default 5)

EXAMPLES:
    agent-tui action @e1 click
    agent-tui action @e1 dblclick
    agent-tui action @e1 fill "my-project"
    agent-tui action @sel1 select "Option 2"
    agent-tui action @list1 select "red" "blue"   # multiselect
    agent-tui action @cb1 toggle
    agent-tui action @cb1 toggle on               # force checked
    agent-tui action @inp1 focus
    agent-tui action @inp1 clear
    agent-tui action @inp1 selectall
    agent-tui action @e1 scroll up 10"#)]
    Action {
        /// Element reference (e.g., @e1, @btn1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Operation to perform
        #[command(subcommand)]
        operation: ActionOperation,
    },

    /// Send keyboard input (keys or text)
    #[command(name = "input")]
    #[command(long_about = r#"Send keyboard input - keys or text.

Unified command for all keyboard input. Automatically detects whether
the input is a key name or text to type.

SUPPORTED KEYS:
    Enter, Tab, Escape, Backspace, Delete
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight
    Home, End, PageUp, PageDown
    F1-F12

MODIFIERS:
    Ctrl+<key>   - Control modifier (e.g., Ctrl+C, Ctrl+A)
    Alt+<key>    - Alt modifier (e.g., Alt+F4)
    Shift+<key>  - Shift modifier

AUTO-DETECTION:
    If the input matches a known key name, it's sent as a key press.
    Otherwise, it's typed as text character by character.
    Use quotes for text that might be mistaken for a key name.

EXAMPLES:
    agent-tui input Enter              # Press Enter
    agent-tui input Ctrl+C             # Press Ctrl+C
    agent-tui input "hello"            # Type text char-by-char
    agent-tui input "Enter"            # Type literal "Enter" text
    agent-tui input Shift --hold       # Hold Shift down
    agent-tui input Shift --release    # Release Shift"#)]
    Input {
        /// Key name, combination, or text to type
        #[arg(required_unless_present_any = ["hold", "release"])]
        value: Option<String>,

        /// Hold key down for modifier sequences (requires key name)
        #[arg(long, conflicts_with = "release")]
        hold: bool,

        /// Release a held key (requires key name)
        #[arg(long, conflicts_with = "hold")]
        release: bool,
    },

    /// Wait for a condition to be met
    #[command(long_about = r#"Wait for a condition to be met before continuing.

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
    --assert            Exit with code 0 if condition met, 1 if timeout
                        Useful for scripting/testing without error messages

EXAMPLES:
    agent-tui wait "Continue"           # Wait for text
    agent-tui wait -e @btn1             # Wait for element
    agent-tui wait -e @spinner --gone   # Wait for element to disappear
    agent-tui wait "Loading" --gone     # Wait for text to disappear
    agent-tui wait --stable             # Wait for screen stability
    agent-tui wait --focused @inp1      # Wait for focus
    agent-tui wait -t 5000 "Done"       # 5 second timeout
    agent-tui wait --assert "Success"   # Exit 0 if found, 1 if not"#)]
    Wait {
        #[command(flatten)]
        params: WaitParams,
    },

    /// Terminate the current session
    Kill,

    /// Manage sessions
    #[command(name = "sessions")]
    #[command(
        long_about = r#"Manage sessions - list, cleanup, attach, or show details.

By default, lists all active sessions. Use flags for other operations.

OPERATIONS:
    sessions              List all active sessions
    sessions <id>         Show details for a specific session
    sessions --cleanup    Remove dead/orphaned sessions
    sessions --cleanup --all  Remove all sessions
    sessions --attach <id>    Attach to a session (interactive mode)
    sessions --status     Include daemon health in output

EXAMPLES:
    agent-tui sessions                    # List sessions
    agent-tui sessions abc123             # Show session details
    agent-tui sessions --cleanup          # Remove dead sessions
    agent-tui sessions --attach abc123    # Attach interactively"#
    )]
    Sessions {
        /// Session ID to show details for
        #[arg(name = "id")]
        session_id: Option<String>,

        /// Remove dead/orphaned sessions
        #[arg(long)]
        cleanup: bool,

        /// With --cleanup: remove all sessions (not just dead ones)
        #[arg(long, requires = "cleanup")]
        all: bool,

        /// Attach to a session interactively
        #[arg(long, value_name = "ID", conflicts_with_all = ["cleanup", "id"])]
        attach: Option<String>,

        /// Include daemon status information
        #[arg(long)]
        status: bool,
    },
    /// Debugging subcommands for development and troubleshooting
    #[command(subcommand)]
    Debug(DebugCommand),

    /// Start recording screen activity
    #[command(name = "record-start")]
    #[command(hide = true)]
    RecordStart,

    /// Stop recording and save output
    #[command(name = "record-stop")]
    #[command(hide = true)]
    RecordStop {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Recording format
        #[arg(long, value_enum, default_value = "json")]
        record_format: RecordFormat,
    },

    /// Check recording status
    #[command(name = "record-status")]
    #[command(hide = true)]
    RecordStatus,

    /// View performance trace data
    #[command(hide = true)]
    Trace {
        /// Number of trace entries to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Start performance tracing
        #[arg(long)]
        start: bool,

        /// Stop performance tracing
        #[arg(long)]
        stop: bool,
    },

    /// View console output
    #[command(hide = true)]
    Console {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,

        /// Clear the console buffer
        #[arg(long)]
        clear: bool,
    },

    /// View captured errors
    #[command(hide = true)]
    Errors {
        /// Number of errors to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Clear the error buffer
        #[arg(long)]
        clear: bool,
    },

    /// Daemon lifecycle management
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Show version information for CLI and daemon
    #[command(long_about = r#"Show detailed version information.

Shows version info for both the CLI binary and the running daemon.
Useful for debugging and ensuring CLI/daemon compatibility.

EXAMPLES:
    agent-tui version
    agent-tui version -f json"#)]
    Version,

    /// Show environment diagnostics
    #[command(long_about = r#"Show environment diagnostics.

Displays all environment variables and configuration that affect
agent-tui behavior. Useful for debugging connection issues.

EXAMPLES:
    agent-tui env
    agent-tui env -f json"#)]
    Env,

    /// Generate shell completion scripts
    #[command(
        long_about = r#"Generate shell completion scripts for bash, zsh, fish, powershell, or elvish.

INSTALLATION:
    # Bash - add to ~/.bashrc
    source <(agent-tui completions bash)

    # Zsh - add to ~/.zshrc
    source <(agent-tui completions zsh)

    # Fish - run once
    agent-tui completions fish > ~/.config/fish/completions/agent-tui.fish

    # PowerShell - add to $PROFILE
    agent-tui completions powershell | Out-String | Invoke-Expression

EXAMPLES:
    agent-tui completions bash
    agent-tui completions zsh > /usr/local/share/zsh/site-functions/_agent-tui"#
    )]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

/// Daemon lifecycle management subcommands
#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Start the daemon
    #[command(long_about = r#"Start the daemon process.

By default, starts the daemon in the background. Use --foreground to run
in the current terminal (useful for debugging).

EXAMPLES:
    agent-tui daemon start              # Start in background
    agent-tui daemon start --foreground # Run in foreground (blocks)"#)]
    Start {
        /// Run in foreground instead of background
        #[arg(long)]
        foreground: bool,
    },

    /// Stop the running daemon gracefully
    #[command(long_about = r#"Stop the running daemon.

Sends SIGTERM to gracefully stop the daemon, allowing it to clean up
sessions and resources. Use --force to send SIGKILL for immediate
termination (not recommended unless daemon is unresponsive).

EXAMPLES:
    agent-tui daemon stop          # Graceful stop
    agent-tui daemon stop --force  # Force kill (SIGKILL)"#)]
    Stop {
        /// Force immediate termination (SIGKILL instead of SIGTERM)
        #[arg(long)]
        force: bool,
    },

    /// Show daemon status with version info
    #[command(long_about = r#"Show daemon status and version information.

Displays whether the daemon is running, its PID, uptime, and version.
Also checks for version mismatch between CLI and daemon.

EXAMPLES:
    agent-tui daemon status"#)]
    Status,

    /// Restart the daemon
    #[command(long_about = r#"Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.

EXAMPLES:
    agent-tui daemon restart"#)]
    Restart,
}

/// Action operations for element interactions
#[derive(Debug, Subcommand, Clone)]
pub enum ActionOperation {
    /// Click the element
    Click,

    /// Double-click the element
    #[command(name = "dblclick")]
    DblClick,

    /// Set the input value
    Fill {
        /// Value to set
        value: String,
    },

    /// Select option(s) from a list or dropdown
    Select {
        /// Option(s) to select (multiple for multiselect)
        #[arg(required = true)]
        options: Vec<String>,
    },

    /// Toggle checkbox or radio button
    Toggle {
        /// Force state: on (checked) or off (unchecked)
        #[arg(value_enum)]
        state: Option<ToggleState>,
    },

    /// Set focus to the element
    Focus,

    /// Clear the input value
    Clear,

    /// Select all text in the input
    #[command(name = "selectall")]
    SelectAll,

    /// Scroll the viewport
    Scroll {
        /// Scroll direction
        #[arg(value_enum)]
        direction: ScrollDirection,

        /// Number of lines/columns to scroll
        #[arg(default_value = "5")]
        amount: u16,
    },
}

/// Toggle state for checkbox/radio operations
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ToggleState {
    On,
    Off,
}

/// Debugging subcommands
#[derive(Debug, Subcommand)]
pub enum DebugCommand {
    /// Recording subcommands
    #[command(subcommand)]
    Record(RecordAction),

    /// View performance trace data
    Trace {
        /// Number of trace entries to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Start performance tracing
        #[arg(long)]
        start: bool,

        /// Stop performance tracing
        #[arg(long)]
        stop: bool,
    },

    /// View console output
    Console {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,

        /// Clear the console buffer
        #[arg(long)]
        clear: bool,
    },

    /// View captured errors
    Errors {
        /// Number of errors to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Clear the error buffer
        #[arg(long)]
        clear: bool,
    },

    /// Show environment diagnostics
    Env,
}

/// Recording subcommands
#[derive(Debug, Subcommand)]
pub enum RecordAction {
    /// Start recording screen activity
    Start,

    /// Stop recording and save output
    Stop {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Recording format
        #[arg(long, value_enum, default_value = "json")]
        format: RecordFormat,
    },

    /// Check recording status
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

/// Parameters for the wait command
#[derive(Debug, Clone, Default, Parser)]
#[command(group = clap::ArgGroup::new("wait_condition").multiple(false).args(&["element", "focused", "stable", "value"]))]
pub struct WaitParams {
    /// Text to wait for on screen
    pub text: Option<String>,

    /// Timeout in milliseconds (default: 30000)
    #[arg(short, long, default_value = "30000")]
    pub timeout: u64,

    /// Wait for element to appear (or disappear with --gone)
    #[arg(short = 'e', long, group = "wait_condition")]
    pub element: Option<String>,

    /// Wait for element to be focused
    #[arg(long, group = "wait_condition")]
    pub focused: Option<String>,

    /// Wait for screen to stabilize
    #[arg(long, group = "wait_condition")]
    pub stable: bool,

    /// Wait for input to have specific value (format: @ref=value)
    #[arg(long, group = "wait_condition")]
    pub value: Option<String>,

    /// Wait for element/text to disappear (use with -e or text argument)
    #[arg(short = 'g', long)]
    pub gone: bool,

    /// Exit with code 0/1 instead of success/error (for scripting)
    #[arg(long)]
    pub assert: bool,
}

impl WaitParams {
    /// Resolve wait condition from WaitParams to (condition_type, target) tuple
    pub fn resolve_condition(&self) -> (Option<String>, Option<String>) {
        if self.stable {
            return (Some("stable".to_string()), None);
        }

        // Handle element with optional --gone modifier
        if let Some(ref elem) = self.element {
            let condition = if self.gone { "not_visible" } else { "element" };
            return (Some(condition.to_string()), Some(elem.clone()));
        }

        if let Some(ref elem) = self.focused {
            return (Some("focused".to_string()), Some(elem.clone()));
        }

        if let Some(ref val) = self.value {
            return (Some("value".to_string()), Some(val.clone()));
        }

        // Handle text with optional --gone modifier
        if let Some(ref txt) = self.text {
            if self.gone {
                return (Some("text_gone".to_string()), Some(txt.clone()));
            }
            // Text without --gone is handled as default (text condition)
        }

        (None, None)
    }
}

/// Parameters for the find command
#[derive(Debug, Clone, Default, Parser)]
pub struct FindParams {
    /// Element role to find (button, input, checkbox, etc.)
    #[arg(long)]
    pub role: Option<String>,

    /// Element name/label to match (supports regex)
    #[arg(long)]
    pub name: Option<String>,

    /// Text content to find (searches label and value)
    #[arg(long)]
    pub text: Option<String>,

    /// Placeholder text to match (for inputs)
    #[arg(long)]
    pub placeholder: Option<String>,

    /// Find the currently focused element
    #[arg(long)]
    pub focused: bool,

    /// Select the nth matching element (0-indexed)
    #[arg(long)]
    pub nth: Option<usize>,

    /// Use exact string matching instead of substring matching
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

    /// Test that the CLI can be constructed with default values
    #[test]
    fn test_cli_defaults() {
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        assert!(cli.session.is_none());
        assert_eq!(cli.format, OutputFormat::Text);
        assert!(!cli.no_color);
        assert!(!cli.verbose);
    }

    /// Test global arguments are parsed correctly
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

    /// Test run command default values match documentation
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

    /// Test spawn with custom dimensions
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

    /// Test spawn with trailing arguments
    #[test]
    fn test_run_with_args() {
        let cli = Cli::parse_from(["agent-tui", "run", "vim", "--", "file.txt", "-n"]);
        let Commands::Run { command, args, .. } = cli.command else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(command, "vim");
        assert_eq!(args, vec!["file.txt".to_string(), "-n".to_string()]);
    }

    /// Test screen command flags
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

    /// Test screen with all flags
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

    /// Test screen accessibility tree format flag
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

    /// Test screen accessibility with interactive-only filter
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

    /// Test action click command
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
        assert!(matches!(operation, ActionOperation::Click));
    }

    /// Test action dblclick command
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
        assert!(matches!(operation, ActionOperation::DblClick));
    }

    /// Test action fill command
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
        let ActionOperation::Fill { value } = operation else {
            panic!("Expected Fill operation, got {:?}", operation);
        };
        assert_eq!(value, "test value");
    }

    /// Test action select command (single option)
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
        let ActionOperation::Select { options } = operation else {
            panic!("Expected Select operation, got {:?}", operation);
        };
        assert_eq!(options, vec!["Option 1"]);
    }

    /// Test action select command (multiple options for multiselect)
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
        let ActionOperation::Select { options } = operation else {
            panic!("Expected Select operation, got {:?}", operation);
        };
        assert_eq!(options, vec!["red", "blue", "green"]);
    }

    /// Test action toggle command
    #[test]
    fn test_action_toggle() {
        // Toggle without state (invert)
        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle"]);
        let Commands::Action {
            element_ref,
            operation,
        } = cli.command
        else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        let ActionOperation::Toggle { state } = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(state.is_none());

        // Toggle with on state
        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle", "on"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let ActionOperation::Toggle { state } = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(matches!(state, Some(ToggleState::On)));

        // Toggle with off state
        let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle", "off"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let ActionOperation::Toggle { state } = operation else {
            panic!("Expected Toggle operation, got {:?}", operation);
        };
        assert!(matches!(state, Some(ToggleState::Off)));
    }

    /// Test action focus command
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
        assert!(matches!(operation, ActionOperation::Focus));
    }

    /// Test action clear command
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
        assert!(matches!(operation, ActionOperation::Clear));
    }

    /// Test action selectall command
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
        assert!(matches!(operation, ActionOperation::SelectAll));
    }

    /// Test action scroll command
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
        let ActionOperation::Scroll { direction, amount } = operation else {
            panic!("Expected Scroll operation, got {:?}", operation);
        };
        assert!(matches!(direction, ScrollDirection::Up));
        assert_eq!(amount, 5); // default

        // With custom amount
        let cli = Cli::parse_from(["agent-tui", "action", "@e1", "scroll", "down", "10"]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let ActionOperation::Scroll { direction, amount } = operation else {
            panic!("Expected Scroll operation, got {:?}", operation);
        };
        assert!(matches!(direction, ScrollDirection::Down));
        assert_eq!(amount, 10);
    }

    /// Test input command with keys
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
            assert_eq!(value, Some(k.to_string()));
            assert!(!hold);
            assert!(!release);
        }
    }

    /// Test input command with text (auto-detected)
    #[test]
    fn test_input_command_text() {
        let cli = Cli::parse_from(["agent-tui", "input", "Hello, World!"]);
        let Commands::Input { value, .. } = cli.command else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, Some("Hello, World!".to_string()));

        // Text that could be mistaken for a key name should be quoted
        let cli = Cli::parse_from(["agent-tui", "input", "hello"]);
        let Commands::Input { value, .. } = cli.command else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
        assert_eq!(value, Some("hello".to_string()));
    }

    /// Test input --hold command
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
        assert_eq!(value, Some("Shift".to_string()));
        assert!(hold);
        assert!(!release);
    }

    /// Test input --release command
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
        assert_eq!(value, Some("Shift".to_string()));
        assert!(!hold);
        assert!(release);
    }

    /// Test input command flag conflicts
    #[test]
    fn test_input_flag_conflicts() {
        // --hold and --release conflict
        assert!(
            Cli::try_parse_from(["agent-tui", "input", "Shift", "--hold", "--release"]).is_err()
        );
    }

    /// Test wait command defaults
    #[test]
    fn test_wait_defaults() {
        let cli = Cli::parse_from(["agent-tui", "wait", "Loading"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Loading".to_string()));

        assert_eq!(params.timeout, 30000, "Default timeout should be 30000ms");
    }

    /// Test wait with custom timeout
    #[test]
    fn test_wait_custom_timeout() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-t", "5000", "Done"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Done".to_string()));
        assert_eq!(params.timeout, 5000);
    }

    /// Test wait with --stable flag
    #[test]
    fn test_wait_stable() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--stable"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(params.stable);
        assert!(params.text.is_none());
    }

    /// Test wait with --element flag
    #[test]
    fn test_wait_element() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--element", "@btn1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@btn1".to_string()));
        assert!(params.text.is_none());
    }

    /// Test wait with -e short flag for element
    #[test]
    fn test_wait_element_short_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@btn1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@btn1".to_string()));
        assert!(!params.gone);
    }

    /// Test wait with --focused flag
    #[test]
    fn test_wait_focused() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--focused", "@inp1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.focused, Some("@inp1".to_string()));
        assert!(params.text.is_none());
    }

    /// Test wait with --gone flag for element disappearing
    #[test]
    fn test_wait_element_gone() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@spinner", "--gone"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@spinner".to_string()));
        assert!(params.gone);
    }

    /// Test wait with --gone flag for text disappearing
    #[test]
    fn test_wait_text_gone() {
        let cli = Cli::parse_from(["agent-tui", "wait", "Loading...", "--gone"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Loading...".to_string()));
        assert!(params.gone);
    }

    /// Test wait with -g short flag for gone
    #[test]
    fn test_wait_gone_short_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@spinner", "-g"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.element, Some("@spinner".to_string()));
        assert!(params.gone);
    }

    /// Test wait with --value flag
    #[test]
    fn test_wait_value() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--value", "@inp1=hello"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.value, Some("@inp1=hello".to_string()));
    }

    /// Test trace defaults
    #[test]
    fn test_trace_defaults() {
        let cli = Cli::parse_from(["agent-tui", "trace"]);
        let Commands::Trace { count, start, stop } = cli.command else {
            panic!("Expected Trace command, got {:?}", cli.command);
        };

        assert_eq!(count, 10, "Default trace count should be 10");
        assert!(!start);
        assert!(!stop);
    }

    /// Test console defaults
    #[test]
    fn test_console_defaults() {
        let cli = Cli::parse_from(["agent-tui", "console"]);
        let Commands::Console { lines, clear } = cli.command else {
            panic!("Expected Console command, got {:?}", cli.command);
        };

        assert_eq!(lines, 100, "Default console lines should be 100");
        assert!(!clear, "Default clear should be false");
    }

    /// Test wait --assert flag
    #[test]
    fn test_wait_assert_flag() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--assert", "Success"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(params.assert);
        assert_eq!(params.text, Some("Success".to_string()));
    }

    /// Test that missing required arguments fail
    #[test]
    fn test_missing_required_args() {
        // action requires both element_ref and operation
        assert!(Cli::try_parse_from(["agent-tui", "action"]).is_err());
        assert!(Cli::try_parse_from(["agent-tui", "action", "@e1"]).is_err());

        // run requires command
        assert!(Cli::try_parse_from(["agent-tui", "run"]).is_err());
    }

    /// Test output format enum values
    #[test]
    fn test_output_format_values() {
        let cli = Cli::parse_from(["agent-tui", "-f", "text", "sessions"]);
        assert_eq!(cli.format, OutputFormat::Text);

        let cli = Cli::parse_from(["agent-tui", "-f", "json", "sessions"]);
        assert_eq!(cli.format, OutputFormat::Json);

        assert!(Cli::try_parse_from(["agent-tui", "-f", "xml", "sessions"]).is_err());
    }

    /// Test --json shorthand flag
    #[test]
    fn test_json_shorthand_flag() {
        let cli = Cli::parse_from(["agent-tui", "--json", "sessions"]);
        assert!(cli.json);
    }

    /// Test spawn with cwd argument
    #[test]
    fn test_run_with_cwd() {
        let cli = Cli::parse_from(["agent-tui", "run", "-d", "/tmp", "bash"]);
        let Commands::Run { command, cwd, .. } = cli.command else {
            panic!("Expected Run command, got {:?}", cli.command);
        };
        assert_eq!(command, "bash");
        assert_eq!(cwd, Some("/tmp".to_string()));
    }

    /// Test errors command with count
    #[test]
    fn test_errors_command_with_count() {
        let cli = Cli::parse_from(["agent-tui", "errors", "-n", "25"]);
        let Commands::Errors { count, clear } = cli.command else {
            panic!("Expected Errors command, got {:?}", cli.command);
        };
        assert_eq!(count, 25);
        assert!(!clear);
    }

    /// Test errors command with clear
    #[test]
    fn test_errors_command_with_clear() {
        let cli = Cli::parse_from(["agent-tui", "errors", "--clear"]);
        let Commands::Errors { clear, .. } = cli.command else {
            panic!("Expected Errors command, got {:?}", cli.command);
        };
        assert!(clear);
    }

    /// Test sessions command - list sessions
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

    /// Test sessions --attach flag
    #[test]
    fn test_sessions_attach() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--attach", "my-session"]);
        let Commands::Sessions { attach, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert_eq!(attach, Some("my-session".to_string()));
    }

    /// Test sessions --cleanup flag
    #[test]
    fn test_sessions_cleanup() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--cleanup"]);
        let Commands::Sessions { cleanup, all, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(cleanup);
        assert!(!all);
    }

    /// Test sessions --cleanup --all flags
    #[test]
    fn test_sessions_cleanup_all() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--cleanup", "--all"]);
        let Commands::Sessions { cleanup, all, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(cleanup);
        assert!(all);
    }

    /// Test sessions --status flag
    #[test]
    fn test_sessions_status() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "--status"]);
        let Commands::Sessions { status, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert!(status);
    }

    /// Test sessions with session ID
    #[test]
    fn test_sessions_with_id() {
        let cli = Cli::parse_from(["agent-tui", "sessions", "abc123"]);
        let Commands::Sessions { session_id, .. } = cli.command else {
            panic!("Expected Sessions command, got {:?}", cli.command);
        };
        assert_eq!(session_id, Some("abc123".to_string()));
    }

    /// Test record-start command
    #[test]
    fn test_record_start_command() {
        let cli = Cli::parse_from(["agent-tui", "record-start"]);
        assert!(matches!(cli.command, Commands::RecordStart));
    }

    /// Test record-stop command
    #[test]
    fn test_record_stop_command() {
        let cli = Cli::parse_from(["agent-tui", "record-stop"]);
        let Commands::RecordStop {
            output,
            record_format,
        } = cli.command
        else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert!(output.is_none());
        assert!(matches!(record_format, RecordFormat::Json));
    }

    /// Test record-stop with output file
    #[test]
    fn test_record_stop_with_output() {
        let cli = Cli::parse_from(["agent-tui", "record-stop", "-o", "recording.json"]);
        let Commands::RecordStop { output, .. } = cli.command else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert_eq!(output, Some("recording.json".to_string()));
    }

    /// Test record-stop with asciicast format
    #[test]
    fn test_record_stop_asciicast_format() {
        let cli = Cli::parse_from(["agent-tui", "record-stop", "--record-format", "asciicast"]);
        let Commands::RecordStop { record_format, .. } = cli.command else {
            panic!("Expected RecordStop command, got {:?}", cli.command);
        };
        assert!(matches!(record_format, RecordFormat::Asciicast));
    }

    /// Test record-status command
    #[test]
    fn test_record_status_command() {
        let cli = Cli::parse_from(["agent-tui", "record-status"]);
        assert!(matches!(cli.command, Commands::RecordStatus));
    }

    /// Test version command
    #[test]
    fn test_version_command() {
        let cli = Cli::parse_from(["agent-tui", "version"]);
        assert!(matches!(cli.command, Commands::Version));
    }

    /// Test env command
    #[test]
    fn test_env_command() {
        let cli = Cli::parse_from(["agent-tui", "env"]);
        assert!(matches!(cli.command, Commands::Env));
    }

    /// Test kill command
    #[test]
    fn test_kill_command() {
        let cli = Cli::parse_from(["agent-tui", "kill"]);
        assert!(matches!(cli.command, Commands::Kill));
    }

    /// Test completions command
    #[test]
    fn test_completions_command() {
        let cli = Cli::parse_from(["agent-tui", "completions", "bash"]);
        let Commands::Completions { shell } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Shell::Bash));
    }

    /// Test completions with fish shell
    #[test]
    fn test_completions_fish() {
        let cli = Cli::parse_from(["agent-tui", "completions", "fish"]);
        let Commands::Completions { shell } = cli.command else {
            panic!("Expected Completions command, got {:?}", cli.command);
        };
        assert!(matches!(shell, Shell::Fish));
    }

    /// Test daemon start command (default: background)
    #[test]
    fn test_daemon_start_default() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "start"]);
        let Commands::Daemon(DaemonCommand::Start { foreground }) = cli.command else {
            panic!("Expected Daemon Start command, got {:?}", cli.command);
        };
        assert!(!foreground, "Default should be background mode");
    }

    /// Test daemon start --foreground
    #[test]
    fn test_daemon_start_foreground() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "start", "--foreground"]);
        let Commands::Daemon(DaemonCommand::Start { foreground }) = cli.command else {
            panic!("Expected Daemon Start command, got {:?}", cli.command);
        };
        assert!(foreground, "Should be foreground mode");
    }

    /// Test daemon stop command (default: graceful)
    #[test]
    fn test_daemon_stop_default() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "stop"]);
        let Commands::Daemon(DaemonCommand::Stop { force }) = cli.command else {
            panic!("Expected Daemon Stop command, got {:?}", cli.command);
        };
        assert!(!force, "Default should be graceful stop");
    }

    /// Test daemon stop --force
    #[test]
    fn test_daemon_stop_force() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "stop", "--force"]);
        let Commands::Daemon(DaemonCommand::Stop { force }) = cli.command else {
            panic!("Expected Daemon Stop command, got {:?}", cli.command);
        };
        assert!(force, "Should be force stop");
    }

    /// Test daemon status command
    #[test]
    fn test_daemon_status() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "status"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommand::Status)
        ));
    }

    /// Test daemon restart command
    #[test]
    fn test_daemon_restart() {
        let cli = Cli::parse_from(["agent-tui", "daemon", "restart"]);
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommand::Restart)
        ));
    }

    // ==========================================================================
    // Tests for removed commands - verify they are NOT recognized
    // These tests implement the CLI consolidation plan (28 → 8 commands)
    // ==========================================================================

    /// Test that standalone 'count' command is NOT recognized
    /// Use `screen -e` filtering or `find --count` instead
    #[test]
    fn test_count_command_removed() {
        assert!(
            Cli::try_parse_from(["agent-tui", "count", "--role", "button"]).is_err(),
            "The 'count' command should be removed. Use 'screen -e' filtering instead."
        );
    }

    /// Test that standalone 'find' command is NOT recognized
    /// Use `screen -e` with filtering options instead
    #[test]
    fn test_find_command_removed() {
        assert!(
            Cli::try_parse_from(["agent-tui", "find", "--role", "button"]).is_err(),
            "The 'find' command should be removed. Use 'screen -e' filtering instead."
        );
    }

    /// Test that standalone 'restart' command (for sessions) is NOT recognized
    /// Use `kill` followed by `run` instead
    #[test]
    fn test_restart_command_removed() {
        assert!(
            Cli::try_parse_from(["agent-tui", "restart"]).is_err(),
            "The 'restart' command should be removed. Use 'kill' + 'run' instead."
        );
    }

    /// Test that standalone 'resize' command is NOT recognized
    /// Terminal dimensions can be set via `run --cols --rows`
    #[test]
    fn test_resize_command_removed() {
        assert!(
            Cli::try_parse_from(["agent-tui", "resize", "--cols", "80"]).is_err(),
            "The 'resize' command should be removed. Use 'run --cols --rows' instead."
        );
    }
}
