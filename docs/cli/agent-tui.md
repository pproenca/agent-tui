# agent-tui CLI Reference

Generated from clap. Run `just cli-docs` to update.

## `agent-tui`

```text
Drive TUI (text UI) applications programmatically or interactively.

Common flow: run -> screenshot -> press/type -> wait -> kill.
Use --format json for automation-friendly output.

Usage: agent-tui [OPTIONS] <COMMAND>

Commands:
  run          Run a TUI application in a virtual terminal
  screenshot   Capture a screenshot of the current session
  resize       Resize the session terminal
  restart      Restart the current session
  press        Send key press(es) to the terminal (supports modifier hold/release)
  type         Type literal text character by character
  wait         Wait for text or screenshot stability
  kill         Kill the current session
  sessions     List and manage sessions
  live         Live preview API for the current session
  daemon       Manage the background daemon
  version      Show version information
  health       Show daemon health status
  env          Show environment diagnostics
  completions  Generate or install shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

WORKFLOW:
    1. Run a TUI application
    2. View the screenshot
    3. Interact with keys/text
    4. Wait for UI changes
    5. Kill the session when done

OUTPUT:
    --format json  Machine-readable JSON (recommended for automation)
    --format text  Human-readable text (default)

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

    # Check daemon status
    agent-tui daemon status
```

## `agent-tui run`

```text
Run a new TUI application in a virtual terminal.

Creates a new PTY session with the specified command and returns a session ID.
The session runs in the background and can be interacted with using other commands.
Use `--` before COMMAND args that start with `-` (e.g., `run -- vim -n`).

Usage: run [OPTIONS] <COMMAND> [ARG]...

Arguments:
  <COMMAND>
          Command to run inside the virtual terminal

  [ARG]...
          Arguments for the command (use -- to pass flags through)

Options:
  -d, --cwd <DIR>
          Working directory for the command

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Terminal Size:
      --cols <COLS>
          Terminal columns (default: 120)
          
          [default: 120]

      --rows <ROWS>
          Terminal rows (default: 40)
          
          [default: 40]

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui run bash
    agent-tui run htop
    agent-tui run "npx create-next-app"
    agent-tui run vim -- file.txt
    agent-tui run --cols 80 --rows 24 nano
```

## `agent-tui screenshot`

```text
View the current screenshot state.

Returns the current terminal screenshot content.

Usage: screenshot [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Filtering:
      --region <REGION>
          Limit capture to a named region (if supported)

Output Options:
      --strip-ansi
          Strip ANSI color codes from output

      --include-cursor
          Include cursor position in output

  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui screenshot              # Just the screenshot
    agent-tui screenshot --strip-ansi # Plain text without colors
```

## `agent-tui resize`

```text
Resize the current session terminal.

Usage: resize [OPTIONS] --cols <COLS> --rows <ROWS>

Options:
      --cols <COLS>
          Terminal columns

      --rows <ROWS>
          Terminal rows

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui resize --cols 120 --rows 40
```

## `agent-tui restart`

```text
Restart the current session command, creating a new session.

Usage: restart [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui restart
    agent-tui --session abc123 restart
```

## `agent-tui press`

```text
Send key press(es) to the terminal (supports modifier hold/release)

Usage: press [OPTIONS] <KEY>...

Arguments:
  <KEY>...
          Keys to press (e.g., Enter, Ctrl+C, ArrowDown)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Modifiers:
      --hold
          Hold a modifier key down (Ctrl, Alt, Shift, Meta)

      --release
          Release a held modifier key (Ctrl, Alt, Shift, Meta)

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

NOTES:
    --hold/--release require a single modifier key (Ctrl, Alt, Shift, Meta)

EXAMPLES:
    agent-tui press Enter
    agent-tui press Ctrl+C
    agent-tui press ArrowDown ArrowDown Enter
    agent-tui press Shift --hold
    agent-tui press Shift --release
```

## `agent-tui type`

```text
Type literal text character by character

Usage: type [OPTIONS] <TEXT>

Arguments:
  <TEXT>
          Text to type

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui type "hello world"
    agent-tui type "user@example.com"
```

## `agent-tui wait`

```text
Wait for a condition to be met before continuing.

Waits for text to appear/disappear or the screenshot to stabilize.
Returns success if the condition is met within the timeout period.

WAIT CONDITIONS:
    <text>       Wait for text to appear on screenshot
    --stable     Wait for screenshot to stop changing
    -g, --gone   Modifier: wait for text to disappear

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 1 if timeout.
                        Without --assert, always exit 0 (timeout still reported).

Usage: wait [OPTIONS] <TEXT|--stable>

Arguments:
  [TEXT]
          Text to wait for (positional)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Timing:
  -t, --timeout <MILLIS>
          Timeout in milliseconds (default: 30000)
          
          [default: 30000]

Wait Condition:
      --stable
          Wait for the screenshot to stop changing

  -g, --gone
          Wait for the text to disappear

Behavior:
      --assert
          Exit with status 0 if met, 1 on timeout

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui wait "Continue"           # Wait for text
    agent-tui wait --stable             # Wait for screenshot stability
    agent-tui wait "Loading" --gone     # Wait for text to disappear
    agent-tui wait -t 5000 "Done"       # 5 second timeout
```

## `agent-tui kill`

```text
Kill the current session

Usage: kill [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui kill
    agent-tui --session abc123 kill
```

## `agent-tui sessions`

```text
Manage sessions - list, show details, attach, switch active, cleanup, or status.

By default, lists all active sessions.

MODES:
    list              List active sessions (default)
    show <id>         Show details for a session
    attach            Attach with TTY (defaults to --session or active)
    switch <id>       Set the active session
    cleanup [--all]   Remove dead/orphaned sessions
    status            Show daemon health

Usage: sessions [OPTIONS] [COMMAND]

Commands:
  list     List active sessions
  show     Show details for a specific session
  attach   Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-Q or --detach-keys)
  switch   Set the active session without attaching
  cleanup  Remove dead/orphaned sessions
  status   Show daemon health status
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui sessions                    # List sessions
    agent-tui sessions list               # List sessions (explicit)
    agent-tui sessions show abc123        # Show session details
    agent-tui sessions attach             # Attach to active session (TTY)
    agent-tui -s abc123 sessions attach   # Attach to session by id (TTY)
    agent-tui sessions switch abc123      # Set active session
    agent-tui -s abc123 sessions attach -T # Attach without TTY (stream output only)
    agent-tui sessions attach --detach-keys 'ctrl-]'  # Custom detach sequence
    agent-tui sessions cleanup            # Remove dead sessions
    agent-tui sessions cleanup --all      # Remove all sessions
    agent-tui sessions status             # Show daemon health
```

## `agent-tui sessions list`

```text
List active sessions

Usage: list [OPTIONS]

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions show`

```text
Show details for a specific session

Usage: show [OPTIONS] <ID>

Arguments:
  <ID>
          

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions attach`

```text
Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-Q or --detach-keys)

Usage: attach [OPTIONS]

Options:
  -T, --no-tty
          Disable TTY mode (stream output only)

      --detach-keys <KEYS>
          Detach key sequence (docker-style, e.g. "ctrl-p,ctrl-q"; use "none" to disable)
          
          [env: AGENT_TUI_DETACH_KEYS=]

  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions switch`

```text
Set the active session without attaching

Usage: switch [OPTIONS] <ID>

Arguments:
  <ID>
          

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions cleanup`

```text
Remove dead/orphaned sessions

Usage: cleanup [OPTIONS]

Options:
      --all
          Remove all sessions (including active)

  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions status`

```text
Show daemon health status

Usage: status [OPTIONS]

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui sessions help`

```text
Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)
```

## `agent-tui live`

```text
Show the daemon's live preview API endpoints.

The daemon exposes HTTP + WebSocket endpoints for live preview by default.
The daemon also serves a built-in web UI at /ui.
Use this command to print the URLs and token so external frontends can connect.

CONFIGURATION:
    AGENT_TUI_API_LISTEN         Bind address (default: 127.0.0.1:0)
    AGENT_TUI_API_ALLOW_REMOTE   Allow non-loopback bind (default: false)
    AGENT_TUI_API_TOKEN          Override token (or 'none' to disable)
    AGENT_TUI_API_STATE          State file path (default: ~/.agent-tui/api.json)
    AGENT_TUI_UI_URL             External UI URL to open with --open (CLI appends api/ws/token/session)
    PORT                         Fallback port for API listen when AGENT_TUI_API_LISTEN is unset

SECURITY:
    API is token-protected by default. Use --allow-remote only when needed.

Usage: live [OPTIONS] [COMMAND]

Commands:
  start   Show the live preview API details
  stop    Stop the live preview API (stop the daemon)
  status  Show live preview API status
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui live start
    agent-tui live status
    agent-tui live stop
```

## `agent-tui live start`

```text
Show the live preview API details

Usage: start [OPTIONS]

Options:
      --open
          Open the preview URL in a browser (uses AGENT_TUI_UI_URL if set)

      --browser <CMD>
          Browser command to use (overrides $BROWSER)

  -h, --help
          Print help

  -V, --version
          Print version

Deprecated:
  -l, --listen [<ADDR>]
          Deprecated (use AGENT_TUI_API_LISTEN and restart the daemon)

      --allow-remote
          Deprecated (use AGENT_TUI_API_ALLOW_REMOTE and restart the daemon)

      --max-viewers <COUNT>
          Deprecated (use AGENT_TUI_API_MAX_CONNECTIONS and restart the daemon)
          
          [env: AGENT_TUI_API_MAX_CONNECTIONS=]

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui live stop`

```text
Stop the live preview API (stop the daemon)

Usage: stop [OPTIONS]

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui live status`

```text
Show live preview API status

Usage: status [OPTIONS]

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui live help`

```text
Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)
```

## `agent-tui daemon`

```text
Manage the background daemon

Usage: daemon [OPTIONS] <COMMAND>

Commands:
  start    Start the daemon process
  stop     Stop the running daemon
  status   Show daemon status and version
  restart  Restart the daemon
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui daemon start`

```text
Start the daemon process.

By default, starts the daemon in the background. Use --foreground to run
in the current terminal (useful for debugging).

Usage: start [OPTIONS]

Options:
      --foreground
          Run in the foreground (debugging)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui daemon start              # Start in background
    agent-tui daemon start --foreground # Run in foreground
```

## `agent-tui daemon stop`

```text
Stop the running daemon.

Sends SIGTERM to gracefully stop the daemon, allowing it to clean up
sessions and resources. Use --force to send SIGKILL for immediate
termination (not recommended unless daemon is unresponsive).

Usage: stop [OPTIONS]

Options:
      --force
          Force kill the daemon (SIGKILL)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui daemon stop          # Graceful stop
    agent-tui daemon stop --force  # Force kill
```

## `agent-tui daemon status`

```text
Show daemon status and version information.

Displays whether the daemon is running, its PID, uptime, and version.
Also checks for version mismatch between CLI and daemon.

EXIT CODES (following LSB init script conventions):
    0 - Daemon is running and healthy
    3 - Daemon is not running

Usage: status [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui daemon restart`

```text
Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.

Usage: restart [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)
```

## `agent-tui daemon help`

```text
Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)
```

## `agent-tui version`

```text
Show detailed version information.

Shows version info for both the CLI binary and the running daemon.
Useful for debugging and ensuring CLI/daemon compatibility.

Usage: version [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui version
    agent-tui --format json version
```

## `agent-tui health`

```text
Show daemon health status and version information.

Alias for `agent-tui daemon status`.

Usage: health [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui health
    agent-tui --format json health
```

## `agent-tui env`

```text
Show environment diagnostics.

Displays all environment variables and configuration that affect
agent-tui behavior. Useful for debugging connection issues.

CONFIGURATION:
    AGENT_TUI_SESSION_STORE      Session metadata log path (default: ~/.agent-tui/sessions.jsonl)
    AGENT_TUI_LOG                Log file path (optional)
    AGENT_TUI_LOG_FORMAT         Log format (text or json; default: text)
    AGENT_TUI_LOG_STREAM         Log output stream (stderr or stdout; default: stderr)
    PORT                         Fallback port for API listen when AGENT_TUI_API_LISTEN is unset

Usage: env [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

EXAMPLES:
    agent-tui env
    agent-tui --format json env
```

## `agent-tui completions`

```text
Generate or install shell completions for bash, zsh, fish, powershell, or elvish.

Runs an interactive setup by default (auto-detects your shell) and checks
whether your installed completions are up-to-date. Use --print to output the
raw completion script for scripting or redirection.

Usage: completions [OPTIONS] [SHELL]

Arguments:
  [SHELL]
          [possible values: bash, elvish, fish, powershell, zsh]

Options:
      --print
          Print the completion script to stdout

      --install
          Install completions to the default location for the shell

  -y, --yes
          Skip prompts and accept defaults

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Output Options:
  -f, --format <FORMAT>
          Output format (text or json)
          
          [default: text]
          [possible values: text, json]

      --json
          Shorthand for --format json (overrides --format if both are set)

      --no-color
          Disable colored output (also respects NO_COLOR)
          
          [env: NO_COLOR=1]

Debug Options:
  -v, --verbose
          Enable verbose output (shows request timing)

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
    agent-tui completions powershell --print | Out-String | Invoke-Expression
```

## `agent-tui help`

```text
Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)
```
