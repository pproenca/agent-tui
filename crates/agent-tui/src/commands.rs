use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
pub use clap_complete::Shell;

const LONG_ABOUT: &str = r#"agent-tui enables AI agents to interact with TUI (Text User Interface) applications.

WORKFLOW:
    1. Run a TUI application
    2. Take snapshots to see the screen and detect elements
    3. Interact using fill, click, key commands
    4. Wait for UI changes
    5. Kill the session when done

ELEMENT REFS:
    Element refs are simple sequential identifiers like @e1, @e2, @e3 that
    you can use to interact with detected UI elements. Run 'agent-tui snap -e'
    to see available elements and their refs.

    @e1, @e2, @e3, ...  - Elements in document order (top-to-bottom, left-to-right)

    Refs reset on each snapshot. Always use the latest snapshot's refs.

EXAMPLES:
    # Start a new Next.js project wizard
    agent-tui run "npx create-next-app"
    agent-tui snap -e
    agent-tui fill @e1 "my-project"
    agent-tui key Enter
    agent-tui wait "success"
    agent-tui kill

    # Interactive menu navigation
    agent-tui run htop
    agent-tui key F10
    agent-tui snap -e
    agent-tui click @e1
    agent-tui kill

    # Check daemon status
    agent-tui status -v"#;

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

    /// Take a snapshot of the current screen state
    #[command(name = "snap")]
    #[command(long_about = r#"Take a snapshot of the current screen state.

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
    agent-tui snap              # Just the screen
    agent-tui snap -e           # Screen + detected elements
    agent-tui snap -a           # Accessibility tree format
    agent-tui snap -a --interactive-only  # Only interactive elements
    agent-tui snap --strip-ansi # Plain text without colors"#)]
    Snap {
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

    /// Click an element by reference
    Click {
        /// Element reference (e.g., @btn1, @e3)
        #[arg(name = "ref")]
        element_ref: String,

        /// Double-click instead of single click
        #[arg(short = '2', long)]
        double: bool,
    },

    /// Set the value of an input element
    Fill {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Value to set in the input
        value: String,
    },

    /// Send keystrokes or type text
    #[command(long_about = r#"Send keystrokes or type text.

Unified command for all keyboard input. Can press a single key, type text,
or hold/release modifier keys.

SUPPORTED KEYS:
    Enter, Tab, Escape, Backspace, Delete
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight
    Home, End, PageUp, PageDown
    F1-F12

MODIFIERS:
    Ctrl+<key>   - Control modifier (e.g., Ctrl+C, Ctrl+A)
    Alt+<key>    - Alt modifier (e.g., Alt+F4)
    Shift+<key>  - Shift modifier

EXAMPLES:
    agent-tui key Enter              # Press Enter
    agent-tui key Ctrl+C             # Press Ctrl+C
    agent-tui key --type "hello"     # Type text char-by-char
    agent-tui key -t "hello"         # Short form for typing
    agent-tui key Shift --hold       # Hold Shift down
    agent-tui key Shift --release    # Release Shift"#)]
    Key {
        /// Key name or combination (e.g., Enter, Ctrl+C) - not required if --type is used
        #[arg(required_unless_present = "text")]
        key: Option<String>,

        /// Type text character by character
        #[arg(short = 't', long = "type", conflicts_with = "key")]
        text: Option<String>,

        /// Hold key down for modifier sequences
        #[arg(long, conflicts_with_all = ["text", "release"])]
        hold: bool,

        /// Release a held key
        #[arg(long, conflicts_with_all = ["text", "hold"])]
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

EXAMPLES:
    agent-tui wait "Continue"           # Wait for text
    agent-tui wait -e @btn1             # Wait for element
    agent-tui wait -e @spinner --gone   # Wait for element to disappear
    agent-tui wait "Loading" --gone     # Wait for text to disappear
    agent-tui wait --stable             # Wait for screen stability
    agent-tui wait --focused @inp1      # Wait for focus
    agent-tui wait -t 5000 "Done"       # 5 second timeout"#)]
    Wait {
        #[command(flatten)]
        params: WaitParams,
    },

    /// Terminate the current session
    Kill,

    /// Restart the TUI application
    #[command(long_about = r#"Restart the TUI application.

Kills the current session and restarts it with the same command.
Equivalent to running 'kill' followed by 'run' with the original command.

This is the TUI equivalent of browser's 'reload' command.

EXAMPLES:
    agent-tui restart                 # Restart current session
    agent-tui restart -s htop-abc123  # Restart specific session"#)]
    Restart,

    /// List all active sessions
    #[command(name = "ls")]
    Ls,

    /// Check daemon status
    #[command(name = "status")]
    Status {
        /// Show connection details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Select an option from a dropdown or list
    Select {
        /// Element reference (e.g., @sel1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Option to select
        option: String,
    },

    /// Select multiple options in a multi-select list
    #[command(long_about = r#"Select multiple options in a multi-select list.

This is the TUI equivalent of browser's multi-select functionality.
Use this for lists where multiple items can be selected simultaneously.

Typical multi-select interaction in TUI:
1. Focus the list element
2. Navigate with arrow keys
3. Press Space to toggle selection for each option

EXAMPLES:
    agent-tui multiselect @e3 "Option 1" "Option 3"
    agent-tui multiselect @list1 red blue green"#)]
    #[command(name = "multiselect")]
    MultiSelect {
        /// Element reference (e.g., @list1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Options to select
        #[arg(required = true)]
        options: Vec<String>,
    },

    /// Scroll the terminal viewport
    Scroll {
        /// Scroll direction (up, down, left, right) - not required if --to is used
        #[arg(value_enum, required_unless_present = "to_ref")]
        direction: Option<ScrollDirection>,

        /// Number of lines/columns to scroll
        #[arg(short, long, default_value = "5")]
        amount: u16,

        /// Target element for scoped scrolling (not yet implemented)
        #[arg(short, long)]
        element: Option<String>,

        /// Scroll until this element is visible
        #[arg(long = "to")]
        to_ref: Option<String>,
    },

    /// Set focus to an element
    Focus {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Clear an input element's value
    Clear {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Select all text in an input element
    #[command(name = "selectall")]
    SelectAll {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Count elements matching criteria
    #[command(long_about = r#"Count elements matching criteria.

This is the TUI equivalent of browser's count command.
Returns the number of elements matching the specified role, name, or text.

EXAMPLES:
    agent-tui count --role button
    agent-tui count --text "Submit"
    agent-tui count --role input --name "Email""#)]
    Count {
        /// Element role to match (button, input, etc.)
        #[arg(long)]
        role: Option<String>,

        /// Element name/label to match
        #[arg(long)]
        name: Option<String>,

        /// Text content to match
        #[arg(long)]
        text: Option<String>,
    },

    /// Toggle a checkbox or radio button
    #[command(long_about = r#"Toggle a checkbox or radio button.

Use this to toggle the checked state of a checkbox or radio button.
By default, it inverts the current state. Use --on or --off to force a specific state.

EXAMPLES:
    agent-tui toggle @e5        # Toggle current state
    agent-tui toggle @e5 --on   # Force checked (idempotent)
    agent-tui toggle @e5 --off  # Force unchecked (idempotent)"#)]
    Toggle {
        /// Element reference (e.g., @cb1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Force checked state (idempotent)
        #[arg(long, conflicts_with = "off")]
        on: bool,

        /// Force unchecked state (idempotent)
        #[arg(long, conflicts_with = "on")]
        off: bool,
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

    /// Resize the terminal window
    #[command(long_about = r#"Resize the terminal window dimensions.

Changes the number of columns and rows for the PTY. This affects how
TUI applications render and can be useful for testing responsive layouts.

EXAMPLES:
    agent-tui resize --cols 120 --rows 40
    agent-tui resize --cols 80 --rows 24"#)]
    Resize {
        /// Number of columns
        #[arg(long, default_value = "120")]
        cols: u16,

        /// Number of rows
        #[arg(long, default_value = "40")]
        rows: u16,
    },

    /// Attach to an existing session
    #[command(long_about = r#"Attach to an existing session by ID.

By default, makes the specified session the active session for subsequent commands.

With --interactive (-i), attaches your terminal directly to the session
for a native terminal experience. Your keystrokes go directly to the app,
and you see its output in real-time. Press Ctrl+\ to detach.

EXAMPLES:
    agent-tui attach abc123          # Set as active session
    agent-tui attach -i abc123       # Interactive mode (native terminal)
    agent-tui ls                     # List session IDs first"#)]
    Attach {
        /// Session ID to attach to
        session_id: String,

        /// Interactive mode: attach terminal directly to the session
        #[arg(short, long)]
        interactive: bool,
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

    /// Assert a condition for testing/scripting
    #[command(long_about = r#"Assert a condition and exit with status code.

Useful for automated testing and scripting. Exits with code 0 if
the condition passes, or code 1 if it fails.

CONDITIONS:
    text:<pattern>        Assert text is visible on screen
    element:<ref>         Assert element exists
    session:<id>          Assert session exists and is running
    exit_code:<expected>  Assert last process exit code (if applicable)

EXAMPLES:
    agent-tui assert text:Success
    agent-tui assert element:@btn1
    agent-tui assert session:main"#)]
    Assert {
        /// Condition to assert (text:pattern, element:ref, session:id)
        condition: String,
    },

    /// Clean up stale sessions
    #[command(long_about = r#"Clean up stale sessions.

Removes sessions that are no longer running or have been idle
for too long. Useful for freeing resources.

EXAMPLES:
    agent-tui cleanup           # Remove dead sessions
    agent-tui cleanup --all     # Remove all sessions"#)]
    Cleanup {
        /// Remove all sessions (not just dead ones)
        #[arg(long)]
        all: bool,
    },

    /// Find elements by semantic properties (role, name, text)
    #[command(long_about = r#"Find elements by semantic properties.

This is a semantic locator that allows finding elements without relying
on refs that may change. Returns matching element refs.

Find modes:
  - By role and name: agent-tui find --role button --name "Submit"
  - By text content: agent-tui find --text "Continue"
  - By focus state: agent-tui find --focused
  - By placeholder: agent-tui find --placeholder "Enter email"
  - Select nth result: agent-tui find --role button --nth 1

MATCHING:
  By default, text matching is case-insensitive and matches substrings.
  Use --exact for exact string matching.

EXAMPLES:
    agent-tui find --role button --name "Submit"
    agent-tui find --text "Continue"
    agent-tui find --focused
    agent-tui find --role input
    agent-tui find --placeholder "Search..."     # Find by placeholder text
    agent-tui find --role button --nth 1        # Get second button (0-indexed)
    agent-tui find --text "Log" --exact         # Exact match only"#)]
    Find {
        #[command(flatten)]
        params: FindParams,
    },

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
        let cli = Cli::parse_from(["agent-tui", "status"]);
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
            "status",
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

    /// Test snap command flags
    #[test]
    fn test_snap_flags() {
        let cli = Cli::parse_from(["agent-tui", "snap", "-i"]);
        let Commands::Snap {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Snap command, got {:?}", cli.command);
        };
        assert!(elements, "-i should enable elements");
        assert!(region.is_none());
        assert!(!strip_ansi);
        assert!(!include_cursor);
    }

    /// Test snap with all flags
    #[test]
    fn test_snap_all_flags() {
        let cli = Cli::parse_from([
            "agent-tui",
            "snap",
            "-i",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ]);
        let Commands::Snap {
            elements,
            region,
            strip_ansi,
            include_cursor,
            ..
        } = cli.command
        else {
            panic!("Expected Snap command, got {:?}", cli.command);
        };
        assert!(elements);
        assert_eq!(region, Some("modal".to_string()));
        assert!(strip_ansi);
        assert!(include_cursor);
    }

    /// Test snap accessibility tree format flag
    #[test]
    fn test_snap_accessibility_flag() {
        let cli = Cli::parse_from(["agent-tui", "snap", "-a"]);
        let Commands::Snap {
            accessibility,
            elements,
            ..
        } = cli.command
        else {
            panic!("Expected Snap command, got {:?}", cli.command);
        };
        assert!(accessibility, "-a should enable accessibility tree format");
        assert!(!elements, "elements should be false by default");
    }

    /// Test snap accessibility with interactive-only filter
    #[test]
    fn test_snap_accessibility_interactive_only() {
        let cli = Cli::parse_from(["agent-tui", "snap", "-a", "--interactive-only"]);
        let Commands::Snap {
            accessibility,
            interactive_only,
            ..
        } = cli.command
        else {
            panic!("Expected Snap command, got {:?}", cli.command);
        };
        assert!(accessibility, "--accessibility should be set");
        assert!(
            interactive_only,
            "--interactive-only should filter to interactive elements"
        );
    }

    /// Test click command requires element ref
    #[test]
    fn test_click_command() {
        let cli = Cli::parse_from(["agent-tui", "click", "@btn1"]);
        let Commands::Click {
            element_ref,
            double,
        } = cli.command
        else {
            panic!("Expected Click command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
        assert!(!double);
    }

    /// Test click command with --double flag
    #[test]
    fn test_click_double_flag() {
        let cli = Cli::parse_from(["agent-tui", "click", "@btn1", "--double"]);
        let Commands::Click {
            element_ref,
            double,
        } = cli.command
        else {
            panic!("Expected Click command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
        assert!(double);

        // Test short flag
        let cli = Cli::parse_from(["agent-tui", "click", "-2", "@btn1"]);
        let Commands::Click { double, .. } = cli.command else {
            panic!("Expected Click command, got {:?}", cli.command);
        };
        assert!(double);
    }

    /// Test fill command requires element ref and value
    #[test]
    fn test_fill_command() {
        let cli = Cli::parse_from(["agent-tui", "fill", "@inp1", "test value"]);
        let Commands::Fill { element_ref, value } = cli.command else {
            panic!("Expected Fill command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
        assert_eq!(value, "test value");
    }

    /// Test key command
    #[test]
    fn test_key_command() {
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
            let cli = Cli::parse_from(["agent-tui", "key", k]);
            let Commands::Key {
                key,
                text,
                hold,
                release,
            } = cli.command
            else {
                panic!("Expected Key command for key: {k}, got {:?}", cli.command);
            };
            assert_eq!(key, Some(k.to_string()));
            assert!(text.is_none());
            assert!(!hold);
            assert!(!release);
        }
    }

    /// Test key --type command
    #[test]
    fn test_key_type_command() {
        let cli = Cli::parse_from(["agent-tui", "key", "--type", "Hello, World!"]);
        let Commands::Key { key, text, .. } = cli.command else {
            panic!("Expected Key command, got {:?}", cli.command);
        };
        assert!(key.is_none());
        assert_eq!(text, Some("Hello, World!".to_string()));

        // Short form
        let cli = Cli::parse_from(["agent-tui", "key", "-t", "hello"]);
        let Commands::Key { key, text, .. } = cli.command else {
            panic!("Expected Key command, got {:?}", cli.command);
        };
        assert!(key.is_none());
        assert_eq!(text, Some("hello".to_string()));
    }

    /// Test key --hold command
    #[test]
    fn test_key_hold_command() {
        let cli = Cli::parse_from(["agent-tui", "key", "Shift", "--hold"]);
        let Commands::Key {
            key, hold, release, ..
        } = cli.command
        else {
            panic!("Expected Key command, got {:?}", cli.command);
        };
        assert_eq!(key, Some("Shift".to_string()));
        assert!(hold);
        assert!(!release);
    }

    /// Test key --release command
    #[test]
    fn test_key_release_command() {
        let cli = Cli::parse_from(["agent-tui", "key", "Shift", "--release"]);
        let Commands::Key {
            key, hold, release, ..
        } = cli.command
        else {
            panic!("Expected Key command, got {:?}", cli.command);
        };
        assert_eq!(key, Some("Shift".to_string()));
        assert!(!hold);
        assert!(release);
    }

    /// Test key command flag conflicts
    #[test]
    fn test_key_flag_conflicts() {
        // --hold and --release conflict
        assert!(Cli::try_parse_from(["agent-tui", "key", "Shift", "--hold", "--release"]).is_err());

        // --type and --hold conflict
        assert!(Cli::try_parse_from(["agent-tui", "key", "--type", "hello", "--hold"]).is_err());

        // --type and --release conflict
        assert!(Cli::try_parse_from(["agent-tui", "key", "--type", "hello", "--release"]).is_err());
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

    /// Test scroll direction enum values
    #[test]
    fn test_scroll_directions() {
        for (arg, expected) in [
            ("up", ScrollDirection::Up),
            ("down", ScrollDirection::Down),
            ("left", ScrollDirection::Left),
            ("right", ScrollDirection::Right),
        ] {
            let cli = Cli::parse_from(["agent-tui", "scroll", arg]);
            let Commands::Scroll {
                direction,
                amount,
                element,
                to_ref,
            } = cli.command
            else {
                panic!("Expected Scroll command for {arg}, got {:?}", cli.command);
            };
            assert_eq!(direction.unwrap() as u8, expected as u8);

            assert_eq!(amount, 5, "Default scroll amount should be 5");
            assert!(element.is_none());
            assert!(to_ref.is_none());
        }
    }

    /// Test scroll --to flag
    #[test]
    fn test_scroll_to_flag() {
        let cli = Cli::parse_from(["agent-tui", "scroll", "--to", "@e5"]);
        let Commands::Scroll {
            direction, to_ref, ..
        } = cli.command
        else {
            panic!("Expected Scroll command, got {:?}", cli.command);
        };
        assert!(direction.is_none());
        assert_eq!(to_ref, Some("@e5".to_string()));
    }

    /// Test scroll with custom amount
    #[test]
    fn test_scroll_custom_amount() {
        let cli = Cli::parse_from(["agent-tui", "scroll", "down", "-a", "10"]);
        let Commands::Scroll { amount, .. } = cli.command else {
            panic!("Expected Scroll command, got {:?}", cli.command);
        };
        assert_eq!(amount, 10);
    }

    /// Test resize defaults
    #[test]
    fn test_resize_defaults() {
        let cli = Cli::parse_from(["agent-tui", "resize"]);
        let Commands::Resize { cols, rows } = cli.command else {
            panic!("Expected Resize command, got {:?}", cli.command);
        };

        assert_eq!(cols, 120, "Default cols should be 120");
        assert_eq!(rows, 40, "Default rows should be 40");
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

    /// Test assert command parsing
    #[test]
    fn test_assert_command() {
        let test_cases = vec!["text:Success", "element:@btn1", "session:main"];

        for condition in test_cases {
            let cli = Cli::parse_from(["agent-tui", "assert", condition]);
            let Commands::Assert { condition: parsed } = cli.command else {
                panic!(
                    "Expected Assert command for {condition}, got {:?}",
                    cli.command
                );
            };
            assert_eq!(parsed, condition);
        }
    }

    /// Test find command combinations
    #[test]
    fn test_find_command() {
        let cli = Cli::parse_from(["agent-tui", "find", "--role", "button"]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert_eq!(params.role, Some("button".to_string()));
        assert!(params.name.is_none());
        assert!(params.text.is_none());
        assert!(params.placeholder.is_none());
        assert!(!params.focused);
        assert!(params.nth.is_none());
        assert!(!params.exact);

        let cli = Cli::parse_from(["agent-tui", "find", "--role", "button", "--name", "Submit"]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert_eq!(params.role, Some("button".to_string()));
        assert_eq!(params.name, Some("Submit".to_string()));

        let cli = Cli::parse_from(["agent-tui", "find", "--focused"]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert!(params.focused);

        let cli = Cli::parse_from(["agent-tui", "find", "--role", "button", "--nth", "2"]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert_eq!(params.nth, Some(2));

        let cli = Cli::parse_from(["agent-tui", "find", "--text", "Submit", "--exact"]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert_eq!(params.text, Some("Submit".to_string()));
        assert!(params.exact);

        let cli = Cli::parse_from(["agent-tui", "find", "--placeholder", "Search..."]);
        let Commands::Find { params } = cli.command else {
            panic!("Expected Find command, got {:?}", cli.command);
        };
        assert_eq!(params.placeholder, Some("Search...".to_string()));
    }

    /// Test that missing required arguments fail
    #[test]
    fn test_missing_required_args() {
        assert!(Cli::try_parse_from(["agent-tui", "click"]).is_err());

        assert!(Cli::try_parse_from(["agent-tui", "fill"]).is_err());
        assert!(Cli::try_parse_from(["agent-tui", "fill", "@inp1"]).is_err());

        assert!(Cli::try_parse_from(["agent-tui", "run"]).is_err());

        assert!(Cli::try_parse_from(["agent-tui", "scroll"]).is_err());
    }

    /// Test output format enum values
    #[test]
    fn test_output_format_values() {
        let cli = Cli::parse_from(["agent-tui", "-f", "text", "status"]);
        assert_eq!(cli.format, OutputFormat::Text);

        let cli = Cli::parse_from(["agent-tui", "-f", "json", "status"]);
        assert_eq!(cli.format, OutputFormat::Json);

        assert!(Cli::try_parse_from(["agent-tui", "-f", "xml", "status"]).is_err());
    }

    /// Test --json shorthand flag
    #[test]
    fn test_json_shorthand_flag() {
        let cli = Cli::parse_from(["agent-tui", "--json", "status"]);
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

    /// Test toggle command
    #[test]
    fn test_toggle_command() {
        let cli = Cli::parse_from(["agent-tui", "toggle", "@cb1"]);
        let Commands::Toggle {
            element_ref,
            on,
            off,
        } = cli.command
        else {
            panic!("Expected Toggle command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        assert!(!on);
        assert!(!off);
    }

    /// Test toggle with --on flag
    #[test]
    fn test_toggle_with_on_flag() {
        let cli = Cli::parse_from(["agent-tui", "toggle", "@cb1", "--on"]);
        let Commands::Toggle {
            element_ref,
            on,
            off,
        } = cli.command
        else {
            panic!("Expected Toggle command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        assert!(on);
        assert!(!off);
    }

    /// Test toggle with --off flag
    #[test]
    fn test_toggle_with_off_flag() {
        let cli = Cli::parse_from(["agent-tui", "toggle", "@cb1", "--off"]);
        let Commands::Toggle {
            element_ref,
            on,
            off,
        } = cli.command
        else {
            panic!("Expected Toggle command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        assert!(!on);
        assert!(off);
    }

    /// Test toggle --on and --off are mutually exclusive
    #[test]
    fn test_toggle_on_off_mutually_exclusive() {
        let result = Cli::try_parse_from(["agent-tui", "toggle", "@cb1", "--on", "--off"]);
        assert!(result.is_err());
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

    /// Test attach command
    #[test]
    fn test_attach_command() {
        let cli = Cli::parse_from(["agent-tui", "attach", "my-session"]);
        let Commands::Attach {
            session_id,
            interactive,
        } = cli.command
        else {
            panic!("Expected Attach command, got {:?}", cli.command);
        };
        assert_eq!(session_id, "my-session");
        assert!(!interactive);
    }

    /// Test attach command with interactive flag
    #[test]
    fn test_attach_command_interactive() {
        let cli = Cli::parse_from(["agent-tui", "attach", "-i", "my-session"]);
        let Commands::Attach {
            session_id,
            interactive,
        } = cli.command
        else {
            panic!("Expected Attach command, got {:?}", cli.command);
        };
        assert_eq!(session_id, "my-session");
        assert!(interactive);
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

    /// Test dblclick command
    /// Test restart command
    #[test]
    fn test_restart_command() {
        let cli = Cli::parse_from(["agent-tui", "restart"]);
        assert!(matches!(cli.command, Commands::Restart));
    }

    /// Test selectall command
    #[test]
    fn test_selectall_command() {
        let cli = Cli::parse_from(["agent-tui", "selectall", "@inp1"]);
        let Commands::SelectAll { element_ref } = cli.command else {
            panic!("Expected SelectAll command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
    }

    /// Test multiselect command
    #[test]
    fn test_multiselect_command() {
        let cli = Cli::parse_from(["agent-tui", "multiselect", "@sel1", "opt1", "opt2", "opt3"]);
        let Commands::MultiSelect {
            element_ref,
            options,
        } = cli.command
        else {
            panic!("Expected MultiSelect command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@sel1");
        assert_eq!(options, vec!["opt1", "opt2", "opt3"]);
    }

    /// Test cleanup command
    #[test]
    fn test_cleanup_command() {
        let cli = Cli::parse_from(["agent-tui", "cleanup"]);
        let Commands::Cleanup { all } = cli.command else {
            panic!("Expected Cleanup command, got {:?}", cli.command);
        };
        assert!(!all);
    }

    /// Test cleanup command with --all
    #[test]
    fn test_cleanup_command_all() {
        let cli = Cli::parse_from(["agent-tui", "cleanup", "--all"]);
        let Commands::Cleanup { all } = cli.command else {
            panic!("Expected Cleanup command, got {:?}", cli.command);
        };
        assert!(all);
    }

    /// Test select command
    #[test]
    fn test_select_command() {
        let cli = Cli::parse_from(["agent-tui", "select", "@sel1", "option1"]);
        let Commands::Select {
            element_ref,
            option,
        } = cli.command
        else {
            panic!("Expected Select command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@sel1");
        assert_eq!(option, "option1");
    }

    /// Test scroll with element target
    #[test]
    fn test_scroll_with_element() {
        let cli = Cli::parse_from(["agent-tui", "scroll", "down", "-e", "@list1"]);
        let Commands::Scroll { element, .. } = cli.command else {
            panic!("Expected Scroll command, got {:?}", cli.command);
        };
        assert_eq!(element, Some("@list1".to_string()));
    }

    /// Test count command with role and name
    #[test]
    fn test_count_command_role_and_name() {
        let cli = Cli::parse_from(["agent-tui", "count", "--role", "button", "--name", "Submit"]);
        let Commands::Count { role, name, text } = cli.command else {
            panic!("Expected Count command, got {:?}", cli.command);
        };
        assert_eq!(role, Some("button".to_string()));
        assert_eq!(name, Some("Submit".to_string()));
        assert!(text.is_none());
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

    /// Test ls command
    #[test]
    fn test_ls_command() {
        let cli = Cli::parse_from(["agent-tui", "ls"]);
        assert!(matches!(cli.command, Commands::Ls));
    }

    /// Test kill command
    #[test]
    fn test_kill_command() {
        let cli = Cli::parse_from(["agent-tui", "kill"]);
        assert!(matches!(cli.command, Commands::Kill));
    }

    /// Test status command verbose flag
    #[test]
    fn test_status_verbose() {
        let cli = Cli::parse_from(["agent-tui", "status", "-v"]);
        let Commands::Status { verbose } = cli.command else {
            panic!("Expected Status command, got {:?}", cli.command);
        };
        assert!(verbose);
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
}
