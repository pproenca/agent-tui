# agent-tui CLI Reference

Generated from clap. Run `just cli-docs` to update.

## `agent-tui`

```text
Drive TUI (text UI) applications programmatically or interactively.

Common flow: run -> screenshot -> press/type/scroll -> wait -> kill.
Use --format json for automation-friendly output.

Supported platforms: Unix-like systems only (Linux, macOS, and environments
with PTYs, Unix domain sockets, and POSIX signals).

Usage: agent-tui [OPTIONS] <COMMAND>

Commands:
  run               Run a TUI application in a virtual terminal
  screenshot        Capture a screenshot of the current session
  action            Deprecated selector action compatibility command
  resize            Resize the session terminal
  restart           Restart the current session
  press             Send key press(es) to the terminal (supports modifier hold/release)
  type              Type literal text character by character
  input             Legacy alias for `type`
  scroll            Scroll using repeated directional terminal input
  scroll-into-view  Deprecated element scroll compatibility command
  wait              Wait for text or screenshot stability
  kill              Kill the current session
  sessions          List and manage sessions
  live              Live preview API exposed by the local daemon
  daemon            Manage the background daemon
  version           Show version information
  env               Show environment diagnostics
  completions       Generate or install shell completions
  help              Print this message or the help of the given subcommand(s)

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

WORKFLOW:
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

Environment:
      --env <KEY=VALUE>
          Environment variable override for the spawned session (repeatable)

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui run bash
    agent-tui run --env FOO=bar --env BAZ=qux bash
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
          Reserved for future named regions; currently rejected if provided

Output Options:
      --strip-ansi
          Strip ANSI color codes from output

      --retain-ansi
          Preserve ANSI color/style codes in output (default)

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

Legacy Compatibility:
  -e
          Deprecated compatibility flag; returns the standard terminal screenshot

  -a
          Deprecated compatibility flag; returns the standard terminal screenshot

      --interactive-only
          Deprecated compatibility flag; returns the standard terminal screenshot

Session Options:
  -s, --session <ID>
          Session ID to use (defaults to the most recent session)

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui screenshot               # Screenshot with terminal colors/styles
    agent-tui screenshot --retain-ansi # Explicitly preserve terminal colors/styles
    agent-tui screenshot --strip-ansi  # Plain text without colors

LEGACY COMPATIBILITY:
    agent-tui screenshot -e             # Deprecated; returns the standard screenshot
    agent-tui screenshot -a             # Deprecated; returns the standard screenshot
    agent-tui screenshot --interactive-only # Deprecated; returns the standard screenshot
```

## `agent-tui action`

```text
Deprecated compatibility command for old selector-based action workflows.

Use current terminal commands (`press`, `type`, and `scroll`) for new scripts.

Usage: action [OPTIONS] <FORM>...

Arguments:
  <FORM>...
          Legacy selector/action form

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

SUPPORTED COMPATIBILITY FORMS:
    agent-tui action <selector> click        # Sends Enter with `agent-tui press Enter`
    agent-tui action <selector> fill <text>  # Types text with `agent-tui type <text>`

Unsupported selector actions return a compatibility error with migration guidance.
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui resize --cols 120 --rows 40
```

## `agent-tui restart`

```text
Restart the current session command, creating a new session.

Usage: restart [OPTIONS]

Options:
      --dry-run
          Preview the restart without changing the session

  -y, --yes
          Skip interactive confirmation

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui restart --yes
    agent-tui --session abc123 restart --dry-run
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

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
Type literal text character by character.

Pass `-` to read the text payload from stdin in non-interactive pipelines.

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui type "hello world"
    agent-tui type "user@example.com"
    printf 'project-name' | agent-tui type -
```

## `agent-tui input`

```text
Legacy alias for `agent-tui type`.

Use `agent-tui type` for new scripts.

Usage: input [OPTIONS] <TEXT>

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]
```

## `agent-tui scroll`

```text
Send repeated directional input to the terminal.

This is a thin convenience wrapper over terminal keys:
    up    -> ArrowUp
    down  -> ArrowDown
    left  -> ArrowLeft
    right -> ArrowRight

Usage: scroll [OPTIONS] <DIRECTION> [AMOUNT]

Arguments:
  <DIRECTION>
          Direction to move

          [possible values: up, down, left, right]

  [AMOUNT]
          Number of steps to send

          [default: 1]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui scroll down
    agent-tui scroll up 10
    agent-tui scroll right 3
```

## `agent-tui scroll-into-view`

```text
Deprecated compatibility command for old element scroll workflows.

The current CLI has no element selector engine. This command does not send terminal input; use `scroll` or `press` for new scripts.

Usage: scroll-into-view [OPTIONS] <FORM>...

Arguments:
  <FORM>...
          Legacy selector form

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

SUPPORTED COMPATIBILITY FORMS:
    agent-tui scroll-into-view <selector>  # No-op compatibility success

Unsupported selector options return a compatibility error with migration guidance.
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
    -e <ref>     Deprecated: treats element ref as literal text

ASSERT MODE:
    --assert            Exit with code 0 if condition met, 75 if timeout.
                        Without --assert, always exit 0 (timeout still reported).

Usage: wait [OPTIONS] <TEXT|--stable|-e <REF>>

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

Legacy Compatibility:
  -e <REF>
          Deprecated compatibility flag; treats the element ref as literal text

Behavior:
      --assert
          Exit with status 0 if met, 75 on timeout

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

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
      --dry-run
          Preview the kill without changing the session

  -y, --yes
          Skip interactive confirmation

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui kill --yes
    agent-tui --session abc123 kill --dry-run
```

## `agent-tui sessions`

```text
Manage sessions - list, show details, attach, switch active, or cleanup.

By default, lists all active sessions.

MODES:
    list              List active sessions (default)
    show <id>         Show details for a session
    attach            Attach with TTY (defaults to --session or active)
    switch <id>       Set the active session
    cleanup [--all]   Remove dead/orphaned sessions

Usage: sessions [OPTIONS] [COMMAND]

Commands:
  list     List active sessions
  show     Show details for a specific session
  attach   Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-B or --detach-keys)
  switch   Set the active session without attaching
  cleanup  Remove dead/orphaned sessions
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

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
    agent-tui sessions cleanup --all --dry-run  # Preview removing all sessions
```

## `agent-tui sessions list`

```text
List active sessions

Usage: list [OPTIONS]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui sessions list
    agent-tui --json sessions list
```

## `agent-tui sessions show`

```text
Show details for a specific session

Usage: show [OPTIONS] <ID>

Arguments:
  <ID>


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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui sessions show abc123
    agent-tui --json sessions show abc123
```

## `agent-tui sessions attach`

```text
Attach to the active session (TTY by default; detach with Ctrl-P Ctrl-B or --detach-keys)

Usage: attach [OPTIONS]

Options:
  -T, --no-tty
          Disable TTY mode (stream output only)

      --detach-keys <KEYS>
          Detach key sequence (docker-style, e.g. "ctrl-p,ctrl-b"; use "none" to disable)

          [env: AGENT_TUI_DETACH_KEYS=]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

NOTES:
    --no-input implies --no-tty for automation-safe streaming.

EXAMPLES:
    agent-tui sessions attach
    agent-tui -s abc123 sessions attach --no-tty
    agent-tui --no-input sessions attach
```

## `agent-tui sessions switch`

```text
Set the active session without attaching

Usage: switch [OPTIONS] <ID>

Arguments:
  <ID>


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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui sessions switch abc123
    agent-tui sessions select abc123
```

## `agent-tui sessions cleanup`

```text
Remove dead/orphaned sessions

Usage: cleanup [OPTIONS]

Options:
      --all
          Remove all sessions (including active)

      --dry-run
          Preview which sessions would be cleaned without killing them

  -y, --yes
          Skip interactive confirmation

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui sessions cleanup --yes
    agent-tui sessions cleanup --all --dry-run
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
    Remote exposure is opt-in. Set AGENT_TUI_WS_ALLOW_REMOTE=1 for non-loopback binds.

Usage: live [OPTIONS] [COMMAND]

Commands:
  start   Show the live preview API details
  stop    Stop any managed UI server and show how to stop daemon-backed live preview
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui live start
    agent-tui live start --open
```

## `agent-tui live stop`

```text
Stop any managed UI server and show how to stop daemon-backed live preview

Usage: stop [OPTIONS]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui live stop
    agent-tui daemon stop --yes   # Stop daemon-backed live preview
```

## `agent-tui live status`

```text
Show live preview API status

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui live status
    agent-tui --json live status
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
Manage the background daemon lifecycle.

Use `daemon start` to launch in the background, `daemon run` for foreground
debugging, and `daemon status` to inspect the local daemon state.

Usage: daemon [OPTIONS] <COMMAND>

Commands:
  start    Start the daemon process
  run      Run the daemon in the foreground
  stop     Stop the running daemon
  status   Show daemon status
  restart  Restart the daemon
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon start
    agent-tui daemon status
    agent-tui daemon stop --yes
```

## `agent-tui daemon start`

```text
Start the daemon process.

Starts the daemon in the background. Use `daemon run` to keep it in the
foreground.

Usage: start [OPTIONS]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon start              # Start in background
```

## `agent-tui daemon run`

```text
Run the daemon in the foreground.

This is the UNIX-style form for supervisors and local debugging when you want
the daemon attached to the current process instead of forking to the
background.

Usage: run [OPTIONS]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon run
    AGENT_TUI_WS_LISTEN=0.0.0.0:8080 agent-tui daemon run
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

      --dry-run
          Preview the stop without changing daemon state

  -y, --yes
          Skip interactive confirmation

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon stop --yes          # Graceful stop
    agent-tui daemon stop --force --yes  # Force kill
```

## `agent-tui daemon status`

```text
Show daemon status.

Reports whether the daemon is running, its PID, versions, and any discovered
WS/UI endpoints.

EXIT CODES (LSB init script conventions):
    0 - Daemon is running
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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon status
    agent-tui --json daemon status
```

## `agent-tui daemon restart`

```text
Restart the daemon.

Stops the running daemon and starts a new one. Useful after updating
the agent-tui binary to ensure the daemon is running the new version.

All active sessions will be terminated during restart.

Usage: restart [OPTIONS]

Options:
      --dry-run
          Preview the restart without changing daemon state

  -y, --yes
          Skip interactive confirmation

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui daemon restart --yes
    agent-tui daemon restart --dry-run
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

Shows version info for both the CLI binary and the local running daemon.
Useful for verifying CLI/daemon compatibility.

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui version
    agent-tui --format json version
```

## `agent-tui env`

```text
Show environment diagnostics.

Displays all environment variables and configuration that affect
agent-tui behavior. Useful for troubleshooting connection issues.

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

EXAMPLES:
    agent-tui env
    agent-tui --format json env
```

## `agent-tui completions`

```text
Generate or install shell completions for bash, zsh, fish, or elvish.

Runs an interactive setup by default (auto-detects your shell) and checks
whether your installed completions are up-to-date. Use --print to output the
raw completion script for scripting or redirection.

Use --no-input to disable prompts and require explicit shell selection.

Usage: completions [OPTIONS] [SHELL]

Arguments:
  [SHELL]
          [possible values: bash, zsh, fish, elvish]

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

Interaction Options:
      --no-input
          Disable prompts and interactive TTY behavior; require explicit flags instead

          [env: AGENT_TUI_NO_INPUT=]

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
    agent-tui completions elvish --print > ~/.elvish/lib/agent-tui.elv
```

## `agent-tui help`

```text
Print this message or the help of the given subcommand(s)

Usage: help [COMMAND]...

Arguments:
  [COMMAND]...
          Print help for the subcommand(s)
```

