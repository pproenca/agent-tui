use clap::{Parser, Subcommand, ValueEnum};
pub use clap_complete::Shell;

const LONG_ABOUT: &str = r#"agent-tui enables AI agents to interact with TUI (Text User Interface) applications.

WORKFLOW:
    1. Spawn a TUI application
    2. Take snapshots to see the screen and detect elements
    3. Interact using fill, click, press commands
    4. Wait for UI changes
    5. Kill the session when done

ELEMENT REFS:
    Element refs are simple sequential identifiers like @e1, @e2, @e3 that
    you can use to interact with detected UI elements. Run 'agent-tui snapshot -i'
    to see available elements and their refs.

    @e1, @e2, @e3, ...  - Elements in document order (top-to-bottom, left-to-right)

    Refs reset on each snapshot. Always use the latest snapshot's refs.

EXAMPLES:
    # Start a new Next.js project wizard
    agent-tui spawn "npx create-next-app"
    agent-tui snapshot -i
    agent-tui fill @e1 "my-project"
    agent-tui press Enter
    agent-tui wait "success"
    agent-tui kill

    # Interactive menu navigation
    agent-tui spawn htop
    agent-tui press F10
    agent-tui snapshot -i
    agent-tui click @e1
    agent-tui kill

    # Check daemon status
    agent-tui health -v"#;

#[derive(Parser)]
#[command(name = "agent-tui")]
#[command(author, version)]
#[command(about = "CLI tool for AI agents to interact with TUI applications")]
#[command(long_about = LONG_ABOUT)]
#[command(propagate_version = true)]
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

    /// Enable debug output (shows full request/response details)
    #[arg(long, global = true)]
    pub debug: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Spawn a new TUI application in a virtual terminal
    #[command(long_about = r#"Spawn a new TUI application in a virtual terminal.

Creates a new PTY session with the specified command and returns a session ID.
The session runs in the background and can be interacted with using other commands.

EXAMPLES:
    agent-tui spawn bash
    agent-tui spawn htop
    agent-tui spawn "npx create-next-app"
    agent-tui spawn vim -- file.txt
    agent-tui spawn --cols 80 --rows 24 nano"#)]
    Spawn {
        /// Command to run (e.g., "htop" or "npx create-next-app")
        command: String,

        /// Arguments to pass to the command
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Working directory
        #[arg(short = 'd', long)]
        cwd: Option<String>,

        /// Terminal columns (default: 120)
        #[arg(long, default_value = "120")]
        cols: u16,

        /// Terminal rows (default: 40)
        #[arg(long, default_value = "40")]
        rows: u16,
    },

    /// Take a snapshot of the current screen state
    #[command(long_about = r#"Take a snapshot of the current screen state.

Returns the current terminal screen content and optionally detects
interactive UI elements like buttons, inputs, and menus.

Element detection uses the Visual Object Model (VOM) which identifies
UI components based on visual styling (colors, backgrounds) rather than
text patterns. This provides reliable detection across different TUI frameworks.

EXAMPLES:
    agent-tui snapshot              # Just the screen
    agent-tui snapshot -i           # Screen + detected elements
    agent-tui snapshot --strip-ansi # Plain text without colors
    agent-tui snapshot -f json      # JSON output for parsing"#)]
    Snapshot {
        /// Include detected interactive elements
        #[arg(short = 'i', long)]
        elements: bool,

        /// Scope to a specific region (e.g., "modal", "menu")
        #[arg(long)]
        region: Option<String>,

        /// Strip ANSI escape codes (colors, formatting)
        #[arg(long)]
        strip_ansi: bool,

        /// Include cursor position in output
        #[arg(long)]
        include_cursor: bool,
    },

    /// Click/activate an element by ref
    Click {
        /// Element reference (e.g., @btn1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Double-click an element by ref
    #[command(name = "dblclick")]
    DblClick {
        /// Element reference (e.g., @btn1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Fill an input element with a value
    Fill {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Value to fill
        value: String,
    },

    /// Press a key or key combination
    #[command(long_about = r#"Press a key or key combination.

Sends a key press to the active terminal session. Supports special keys,
modifiers, and key combinations.

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
    agent-tui press Enter
    agent-tui press Tab
    agent-tui press Ctrl+C
    agent-tui press ArrowDown
    agent-tui press F10"#)]
    Press {
        /// Key to press (e.g., "Enter", "Tab", "Ctrl+C", "ArrowDown")
        key: String,
    },

    /// Type literal text
    Type {
        /// Text to type
        text: String,
    },

    /// Hold a key down (for modifier sequences)
    #[command(long_about = r#"Hold a key down for modifier sequences.

Use this with keyup to create modifier key sequences. For example, hold Shift
while performing other actions.

SUPPORTED KEYS:
    Ctrl, Alt, Shift, Meta

Note: Most use cases are better served by keystroke with modifiers (e.g., Ctrl+A).
Use keydown/keyup only when you need to hold a modifier while performing
multiple actions.

EXAMPLES:
    agent-tui keydown Shift
    agent-tui click @item1
    agent-tui click @item2
    agent-tui keyup Shift"#)]
    #[command(name = "keydown")]
    KeyDown {
        /// Key to hold down (e.g., "Shift", "Ctrl", "Alt")
        key: String,
    },

    /// Release a held key (for modifier sequences)
    #[command(long_about = r#"Release a held key.

Use this to release a key that was held with keydown.

EXAMPLES:
    agent-tui keydown Shift
    agent-tui click @item1
    agent-tui keyup Shift"#)]
    #[command(name = "keyup")]
    KeyUp {
        /// Key to release (e.g., "Shift", "Ctrl", "Alt")
        key: String,
    },

    /// Wait for a condition to be met before continuing
    #[command(long_about = r#"Wait for a condition to be met before continuing.

Waits for text to appear, elements to change, or the screen to stabilize.
Returns success if the condition is met within the timeout period.

WAIT CONDITIONS:
    <text>           Wait for text to appear on screen
    --element <ref>  Wait for element to appear
    --visible <ref>  Wait for element to appear (alias for --element)
    --focused <ref>  Wait for element to be focused
    --not-visible    Wait for element to disappear
    --text-gone      Wait for text to disappear
    --stable         Wait for screen to stop changing
    --value <ref>=<val>  Wait for input to have specific value

EXAMPLES:
    agent-tui wait "Continue"           # Wait for text
    agent-tui wait --element @btn1      # Wait for button
    agent-tui wait --stable             # Wait for screen stability
    agent-tui wait -t 5000 "Loading"    # 5 second timeout"#)]
    Wait {
        #[command(flatten)]
        params: WaitParams,
    },

    /// Kill the TUI application
    Kill,

    /// Restart the TUI application (kill + respawn with same command)
    #[command(long_about = r#"Restart the TUI application.

Kills the current session and respawns it with the same command.
Equivalent to running 'kill' followed by 'spawn' with the original command.

This is the TUI equivalent of browser's 'reload' command.

EXAMPLES:
    agent-tui restart                 # Restart current session
    agent-tui restart -s htop-abc123  # Restart specific session"#)]
    Restart,

    /// List all active sessions
    Sessions,

    /// Start built-in demo TUI for testing element detection
    #[command(long_about = r#"Start the built-in demo TUI.

This command spawns a simple TUI form for testing agent-tui without needing
external applications like htop or create-next-app. Perfect for first-time
setup verification and learning the element ref system.

The demo TUI displays:
- An input field for entering your name
- A checkbox for enabling notifications
- Submit and Cancel buttons

EXAMPLE WORKFLOW:
    agent-tui demo                    # Start demo TUI
    agent-tui snapshot -i             # See detected elements:
                                      #   @e1 [input:Name]
                                      #   @e2 [checkbox]
                                      #   @e3 [button:Submit]
                                      #   @e4 [button:Cancel]
    agent-tui fill @e1 "Hello"        # Fill the input
    agent-tui toggle @e2              # Toggle checkbox
    agent-tui click @e3               # Click Submit
    agent-tui kill                    # End session"#)]
    Demo,

    /// Check daemon health and connection status
    Health {
        /// Show verbose output with additional details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Select an option in a dropdown/select element
    Select {
        /// Element reference (e.g., @sel1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Option to select (name or value)
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

        /// Options to select (one or more)
        #[arg(required = true)]
        options: Vec<String>,
    },

    /// Scroll in a direction
    Scroll {
        /// Direction to scroll
        #[arg(value_enum)]
        direction: ScrollDirection,

        /// Amount to scroll (default: 5)
        #[arg(short, long, default_value = "5")]
        amount: u16,

        /// Element to scroll within (optional)
        #[arg(short, long)]
        element: Option<String>,
    },

    /// Scroll until element is visible (agent-browser parity)
    ///
    /// Scrolls the terminal in the appropriate direction until the target element
    /// appears on screen. Useful for navigating long lists or scrollable content.
    #[command(name = "scrollintoview")]
    ScrollIntoView {
        /// Element ref to scroll into view
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Focus a specific element
    Focus {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Clear an input field
    Clear {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Select all text in an element (agent-browser parity)
    #[command(name = "selectall")]
    SelectAll {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Get information about elements or session
    #[command(subcommand)]
    Get(GetCommand),

    /// Check element state (visible, focused, enabled, checked)
    #[command(subcommand)]
    Is(IsCommand),

    /// Count elements matching criteria
    #[command(long_about = r#"Count elements matching criteria.

This is the TUI equivalent of browser's count command.
Returns the number of elements matching the specified role, name, or text.

EXAMPLES:
    agent-tui count --role button
    agent-tui count --text "Submit"
    agent-tui count --role input --name "Email""#)]
    Count {
        /// Element role to count (button, input, checkbox, etc.)
        #[arg(long)]
        role: Option<String>,

        /// Element name/label to match
        #[arg(long)]
        name: Option<String>,

        /// Text content to find (searches label and value)
        #[arg(long)]
        text: Option<String>,
    },

    /// Toggle a checkbox or radio button
    #[command(long_about = r#"Toggle a checkbox or radio button.

Use this to toggle the checked state of a checkbox or radio button.
By default, it inverts the current state. Use --state to force a specific state.

For explicit check/uncheck operations, prefer using 'check' and 'uncheck' commands.

EXAMPLES:
    agent-tui toggle @e5              # Toggle current state
    agent-tui toggle @e5 --state true # Force checked
    agent-tui toggle @e5 --state false # Force unchecked"#)]
    Toggle {
        /// Element reference (e.g., @cb1)
        #[arg(name = "ref")]
        element_ref: String,

        /// Force specific state: true to check, false to uncheck
        #[arg(long)]
        state: Option<bool>,
    },

    /// Check a checkbox (agent-browser parity)
    #[command(long_about = r#"Check a checkbox element.

Ensures the checkbox is checked. If already checked, this is a no-op.

EXAMPLES:
    agent-tui check @e2               # Check the checkbox"#)]
    Check {
        /// Element reference (e.g., @cb1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Uncheck a checkbox (agent-browser parity)
    #[command(long_about = r#"Uncheck a checkbox element.

Ensures the checkbox is unchecked. If already unchecked, this is a no-op.

EXAMPLES:
    agent-tui uncheck @e2             # Uncheck the checkbox"#)]
    Uncheck {
        /// Element reference (e.g., @cb1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Start recording a session
    RecordStart,

    /// Stop recording and save to file
    RecordStop {
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,

        /// Output format (json or asciicast)
        #[arg(long = "record-format", value_enum, default_value = "json")]
        record_format: RecordFormat,
    },

    /// Check recording status
    RecordStatus,

    /// Show recent interaction trace
    Trace {
        /// Number of entries to show
        #[arg(short, long, default_value = "10")]
        count: usize,

        /// Start tracing
        #[arg(long)]
        start: bool,

        /// Stop tracing
        #[arg(long)]
        stop: bool,
    },

    /// Show console/terminal output
    Console {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,

        /// Clear console buffer
        #[arg(long)]
        clear: bool,
    },

    /// Show captured errors (stderr, signals, exit codes)
    #[command(long_about = r#"Show captured errors from the session.

Displays stderr output, non-zero exit codes, and signals received by the
spawned process. Useful for debugging application failures.

ERROR SOURCES:
    stderr    - Standard error output from the process
    exit_code - Non-zero exit codes when process terminates
    signal    - Signals received (SIGSEGV, SIGTERM, etc.)

EXAMPLES:
    agent-tui errors              # Show recent errors
    agent-tui errors --count 20   # Show last 20 errors
    agent-tui errors --clear      # Clear error buffer"#)]
    Errors {
        /// Number of errors to show
        #[arg(short = 'n', long, default_value = "50")]
        count: usize,

        /// Clear error buffer
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
    agent-tui sessions               # List session IDs first"#)]
    Attach {
        /// Session ID to attach to
        session_id: String,

        /// Interactive mode: attach terminal directly to the session
        #[arg(short, long)]
        interactive: bool,
    },

    /// Start the daemon (usually called automatically)
    #[command(hide = true)]
    Daemon,

    /// Run the demo TUI directly (internal command, used by 'demo')
    #[command(hide = true, name = "demo-run")]
    DemoRun,

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

/// Subcommands for the `get` command
#[derive(Debug, Subcommand)]
pub enum GetCommand {
    /// Get the text content of an element
    Text {
        /// Element reference (e.g., @btn1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Get the value of an input element
    Value {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Get the currently focused element
    Focused,

    /// Get the session title/command
    Title,
}

/// Subcommands for the `is` command
#[derive(Debug, Subcommand)]
pub enum IsCommand {
    /// Check if an element is visible
    Visible {
        /// Element reference (e.g., @btn1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Check if an element is focused
    Focused {
        /// Element reference (e.g., @inp1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Check if an element is enabled (not disabled)
    Enabled {
        /// Element reference (e.g., @btn1)
        #[arg(name = "ref")]
        element_ref: String,
    },

    /// Check if a checkbox or radio button is checked
    Checked {
        /// Element reference (e.g., @cb1)
        #[arg(name = "ref")]
        element_ref: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum, Default)]
pub enum RecordFormat {
    #[default]
    Json,
    Asciicast,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum WaitConditionArg {
    Text,
    Element,
    Focused,
    NotVisible,
    Stable,
    TextGone,
    Value,
}

impl std::fmt::Display for WaitConditionArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitConditionArg::Text => write!(f, "text"),
            WaitConditionArg::Element => write!(f, "element"),
            WaitConditionArg::Focused => write!(f, "focused"),
            WaitConditionArg::NotVisible => write!(f, "not_visible"),
            WaitConditionArg::Stable => write!(f, "stable"),
            WaitConditionArg::TextGone => write!(f, "text_gone"),
            WaitConditionArg::Value => write!(f, "value"),
        }
    }
}

/// Parameters for the wait command
#[derive(Debug, Clone, Default, Parser)]
#[command(group = clap::ArgGroup::new("wait_condition").multiple(false).args(&["element", "visible", "focused", "not_visible", "text_gone", "stable", "value"]))]
pub struct WaitParams {
    /// Text to wait for (legacy mode, use --condition for more options)
    pub text: Option<String>,

    /// Timeout in milliseconds (default: 30000)
    #[arg(short, long, default_value = "30000")]
    pub timeout: u64,

    /// Wait condition type
    #[arg(long, value_enum, conflicts_with = "wait_condition")]
    pub condition: Option<WaitConditionArg>,

    /// Target for the condition (element ref or text pattern)
    #[arg(long)]
    pub target: Option<String>,

    /// Wait for element to appear
    #[arg(long, group = "wait_condition")]
    pub element: Option<String>,

    /// Wait for element to appear (alias for --element, agent-browser parity)
    #[arg(long, group = "wait_condition")]
    pub visible: Option<String>,

    /// Wait for element to be focused
    #[arg(long, group = "wait_condition")]
    pub focused: Option<String>,

    /// Wait for element to disappear
    #[arg(long, group = "wait_condition")]
    pub not_visible: Option<String>,

    /// Wait for text to disappear
    #[arg(long, group = "wait_condition")]
    pub text_gone: Option<String>,

    /// Wait for screen to stabilize
    #[arg(long, group = "wait_condition")]
    pub stable: bool,

    /// Wait for input to have specific value (format: @ref=value)
    #[arg(long, group = "wait_condition")]
    pub value: Option<String>,
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
        let cli = Cli::parse_from(["agent-tui", "health"]);
        assert!(cli.session.is_none());
        assert_eq!(cli.format, OutputFormat::Text);
        assert!(!cli.no_color);
        assert!(!cli.verbose);
        assert!(!cli.debug);
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
            "--debug",
            "health",
        ]);
        assert_eq!(cli.session, Some("my-session".to_string()));
        assert_eq!(cli.format, OutputFormat::Json);
        assert!(cli.no_color);
        assert!(cli.verbose);
        assert!(cli.debug);
    }

    /// Test spawn command default values match documentation
    #[test]
    fn test_spawn_defaults() {
        let cli = Cli::parse_from(["agent-tui", "spawn", "bash"]);
        let Commands::Spawn {
            command,
            args,
            cwd,
            cols,
            rows,
        } = cli.command
        else {
            panic!("Expected Spawn command, got {:?}", cli.command);
        };
        assert_eq!(command, "bash");
        assert!(args.is_empty());
        assert!(cwd.is_none());

        assert_eq!(cols, 120, "Default cols should be 120");
        assert_eq!(rows, 40, "Default rows should be 40");
    }

    /// Test spawn with custom dimensions
    #[test]
    fn test_spawn_custom_dimensions() {
        let cli = Cli::parse_from(["agent-tui", "spawn", "--cols", "80", "--rows", "24", "vim"]);
        let Commands::Spawn {
            cols,
            rows,
            command,
            ..
        } = cli.command
        else {
            panic!("Expected Spawn command, got {:?}", cli.command);
        };
        assert_eq!(cols, 80);
        assert_eq!(rows, 24);
        assert_eq!(command, "vim");
    }

    /// Test spawn with trailing arguments
    #[test]
    fn test_spawn_with_args() {
        let cli = Cli::parse_from(["agent-tui", "spawn", "vim", "--", "file.txt", "-n"]);
        let Commands::Spawn { command, args, .. } = cli.command else {
            panic!("Expected Spawn command, got {:?}", cli.command);
        };
        assert_eq!(command, "vim");
        assert_eq!(args, vec!["file.txt".to_string(), "-n".to_string()]);
    }

    /// Test snapshot command flags
    #[test]
    fn test_snapshot_flags() {
        let cli = Cli::parse_from(["agent-tui", "snapshot", "-i"]);
        let Commands::Snapshot {
            elements,
            region,
            strip_ansi,
            include_cursor,
        } = cli.command
        else {
            panic!("Expected Snapshot command, got {:?}", cli.command);
        };
        assert!(elements, "-i should enable elements");
        assert!(region.is_none());
        assert!(!strip_ansi);
        assert!(!include_cursor);
    }

    /// Test snapshot with all flags
    #[test]
    fn test_snapshot_all_flags() {
        let cli = Cli::parse_from([
            "agent-tui",
            "snapshot",
            "-i",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ]);
        let Commands::Snapshot {
            elements,
            region,
            strip_ansi,
            include_cursor,
        } = cli.command
        else {
            panic!("Expected Snapshot command, got {:?}", cli.command);
        };
        assert!(elements);
        assert_eq!(region, Some("modal".to_string()));
        assert!(strip_ansi);
        assert!(include_cursor);
    }

    /// Test click command requires element ref
    #[test]
    fn test_click_command() {
        let cli = Cli::parse_from(["agent-tui", "click", "@btn1"]);
        let Commands::Click { element_ref } = cli.command else {
            panic!("Expected Click command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
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

    /// Test press command
    #[test]
    fn test_press_command() {
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

        for key in test_cases {
            let cli = Cli::parse_from(["agent-tui", "press", key]);
            let Commands::Press { key: parsed_key } = cli.command else {
                panic!(
                    "Expected Press command for key: {key}, got {:?}",
                    cli.command
                );
            };
            assert_eq!(parsed_key, key);
        }
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

    /// Test wait with --visible flag (alias for --element)
    #[test]
    fn test_wait_visible() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--visible", "@btn1"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.visible, Some("@btn1".to_string()));
        assert!(params.text.is_none());
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

    /// Test wait with --not-visible flag
    #[test]
    fn test_wait_not_visible() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--not-visible", "@spinner"]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.not_visible, Some("@spinner".to_string()));
    }

    /// Test wait with --text-gone flag
    #[test]
    fn test_wait_text_gone() {
        let cli = Cli::parse_from(["agent-tui", "wait", "--text-gone", "Loading..."]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert_eq!(params.text_gone, Some("Loading...".to_string()));
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

    /// Test wait with --condition and --target flags
    #[test]
    fn test_wait_condition_with_target() {
        let cli = Cli::parse_from([
            "agent-tui",
            "wait",
            "--condition",
            "element",
            "--target",
            "@btn1",
        ]);
        let Commands::Wait { params } = cli.command else {
            panic!("Expected Wait command, got {:?}", cli.command);
        };
        assert!(matches!(params.condition, Some(WaitConditionArg::Element)));
        assert_eq!(params.target, Some("@btn1".to_string()));
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
            } = cli.command
            else {
                panic!("Expected Scroll command for {arg}, got {:?}", cli.command);
            };
            assert_eq!(direction as u8, expected as u8);

            assert_eq!(amount, 5, "Default scroll amount should be 5");
            assert!(element.is_none());
        }
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

        assert!(Cli::try_parse_from(["agent-tui", "spawn"]).is_err());

        assert!(Cli::try_parse_from(["agent-tui", "scroll"]).is_err());
    }

    /// Test output format enum values
    #[test]
    fn test_output_format_values() {
        let cli = Cli::parse_from(["agent-tui", "-f", "text", "health"]);
        assert_eq!(cli.format, OutputFormat::Text);

        let cli = Cli::parse_from(["agent-tui", "-f", "json", "health"]);
        assert_eq!(cli.format, OutputFormat::Json);

        assert!(Cli::try_parse_from(["agent-tui", "-f", "xml", "health"]).is_err());
    }

    // =========================================================================
    // Additional CLI Parsing Tests
    // =========================================================================

    /// Test --json shorthand flag
    #[test]
    fn test_json_shorthand_flag() {
        let cli = Cli::parse_from(["agent-tui", "--json", "health"]);
        assert!(cli.json);
    }

    /// Test spawn with cwd argument
    #[test]
    fn test_spawn_with_cwd() {
        let cli = Cli::parse_from(["agent-tui", "spawn", "-d", "/tmp", "bash"]);
        let Commands::Spawn { command, cwd, .. } = cli.command else {
            panic!("Expected Spawn command, got {:?}", cli.command);
        };
        assert_eq!(command, "bash");
        assert_eq!(cwd, Some("/tmp".to_string()));
    }

    /// Test get text subcommand
    #[test]
    fn test_get_text_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "get", "text", "@btn1"]);
        let Commands::Get(GetCommand::Text { element_ref }) = cli.command else {
            panic!("Expected Get Text command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
    }

    /// Test get value subcommand
    #[test]
    fn test_get_value_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "get", "value", "@inp1"]);
        let Commands::Get(GetCommand::Value { element_ref }) = cli.command else {
            panic!("Expected Get Value command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
    }

    /// Test get focused subcommand
    #[test]
    fn test_get_focused_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "get", "focused"]);
        assert!(matches!(cli.command, Commands::Get(GetCommand::Focused)));
    }

    /// Test get title subcommand
    #[test]
    fn test_get_title_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "get", "title"]);
        assert!(matches!(cli.command, Commands::Get(GetCommand::Title)));
    }

    /// Test is visible subcommand
    #[test]
    fn test_is_visible_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "is", "visible", "@btn1"]);
        let Commands::Is(IsCommand::Visible { element_ref }) = cli.command else {
            panic!("Expected Is Visible command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
    }

    /// Test is focused subcommand
    #[test]
    fn test_is_focused_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "is", "focused", "@inp1"]);
        let Commands::Is(IsCommand::Focused { element_ref }) = cli.command else {
            panic!("Expected Is Focused command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@inp1");
    }

    /// Test is enabled subcommand
    #[test]
    fn test_is_enabled_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "is", "enabled", "@btn1"]);
        let Commands::Is(IsCommand::Enabled { element_ref }) = cli.command else {
            panic!("Expected Is Enabled command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@btn1");
    }

    /// Test is checked subcommand
    #[test]
    fn test_is_checked_subcommand() {
        let cli = Cli::parse_from(["agent-tui", "is", "checked", "@cb1"]);
        let Commands::Is(IsCommand::Checked { element_ref }) = cli.command else {
            panic!("Expected Is Checked command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
    }

    /// Test toggle with state flag
    #[test]
    fn test_toggle_with_state_true() {
        let cli = Cli::parse_from(["agent-tui", "toggle", "@cb1", "--state", "true"]);
        let Commands::Toggle { element_ref, state } = cli.command else {
            panic!("Expected Toggle command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        assert_eq!(state, Some(true));
    }

    /// Test toggle with state false
    #[test]
    fn test_toggle_with_state_false() {
        let cli = Cli::parse_from(["agent-tui", "toggle", "@cb1", "--state", "false"]);
        let Commands::Toggle { element_ref, state } = cli.command else {
            panic!("Expected Toggle command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
        assert_eq!(state, Some(false));
    }

    /// Test check command
    #[test]
    fn test_check_command() {
        let cli = Cli::parse_from(["agent-tui", "check", "@cb1"]);
        let Commands::Check { element_ref } = cli.command else {
            panic!("Expected Check command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
    }

    /// Test uncheck command
    #[test]
    fn test_uncheck_command() {
        let cli = Cli::parse_from(["agent-tui", "uncheck", "@cb1"]);
        let Commands::Uncheck { element_ref } = cli.command else {
            panic!("Expected Uncheck command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@cb1");
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

    /// Test demo command
    #[test]
    fn test_demo_command() {
        let cli = Cli::parse_from(["agent-tui", "demo"]);
        assert!(matches!(cli.command, Commands::Demo));
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
    #[test]
    fn test_dblclick_command() {
        let cli = Cli::parse_from(["agent-tui", "dblclick", "@item1"]);
        let Commands::DblClick { element_ref } = cli.command else {
            panic!("Expected DblClick command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@item1");
    }

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

    /// Test scrollintoview command
    #[test]
    fn test_scrollintoview_command() {
        let cli = Cli::parse_from(["agent-tui", "scrollintoview", "@item5"]);
        let Commands::ScrollIntoView { element_ref } = cli.command else {
            panic!("Expected ScrollIntoView command, got {:?}", cli.command);
        };
        assert_eq!(element_ref, "@item5");
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

    /// Test type command
    #[test]
    fn test_type_command() {
        let cli = Cli::parse_from(["agent-tui", "type", "Hello, World!"]);
        let Commands::Type { text } = cli.command else {
            panic!("Expected Type command, got {:?}", cli.command);
        };
        assert_eq!(text, "Hello, World!");
    }

    /// Test keydown command
    #[test]
    fn test_keydown_command() {
        let cli = Cli::parse_from(["agent-tui", "keydown", "Shift"]);
        let Commands::KeyDown { key } = cli.command else {
            panic!("Expected KeyDown command, got {:?}", cli.command);
        };
        assert_eq!(key, "Shift");
    }

    /// Test keyup command
    #[test]
    fn test_keyup_command() {
        let cli = Cli::parse_from(["agent-tui", "keyup", "Shift"]);
        let Commands::KeyUp { key } = cli.command else {
            panic!("Expected KeyUp command, got {:?}", cli.command);
        };
        assert_eq!(key, "Shift");
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

    /// Test sessions command
    #[test]
    fn test_sessions_command() {
        let cli = Cli::parse_from(["agent-tui", "sessions"]);
        assert!(matches!(cli.command, Commands::Sessions));
    }

    /// Test kill command
    #[test]
    fn test_kill_command() {
        let cli = Cli::parse_from(["agent-tui", "kill"]);
        assert!(matches!(cli.command, Commands::Kill));
    }

    /// Test health command verbose flag
    #[test]
    fn test_health_verbose() {
        let cli = Cli::parse_from(["agent-tui", "health", "-v"]);
        let Commands::Health { verbose } = cli.command else {
            panic!("Expected Health command, got {:?}", cli.command);
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
}
