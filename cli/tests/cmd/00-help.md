# Help and Version Tests

Tests for --help, --version, and subcommand help output.

## Top-level help

```console
$ agent-tui --help
agent-tui enables AI agents to interact with TUI[..]
...

```

## Version output

```console
$ agent-tui --version
agent-tui [..]

```

## Spawn short help

```console
$ agent-tui spawn -h
Spawn a new TUI application in a virtual terminal

Usage: agent-tui spawn [OPTIONS] <COMMAND> [ARGS]...

Arguments:
  <COMMAND>  Command to run (e.g., "htop" or "npx create-next-app")
  [ARGS]...  Arguments to pass to the command

Options:
  -d, --cwd <CWD>[..]
      --cols <COLS>[..]
      --rows <ROWS>[..]
...

```

## Snapshot short help

```console
$ agent-tui snapshot -h
Take a snapshot of the current screen state

Usage: agent-tui snapshot [OPTIONS]

Options:
  -i, --elements[..]
      --interactive-only[..]
  -c, --compact[..]
      --region <REGION>[..]
...

```

## Click short help

```console
$ agent-tui click -h
Click/activate an element by ref

Usage: agent-tui click [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Fill short help

```console
$ agent-tui fill -h
Fill an input element with a value

Usage: agent-tui fill [OPTIONS] <ref> <VALUE>

Arguments:
  <ref>    [..]
  <VALUE>  Value to fill
...

```

## Press short help

```console
$ agent-tui press -h
Press a key or key combination

Usage: agent-tui press [OPTIONS] <KEY>

Arguments:
  <KEY>  [..]
...

```

## Type short help

```console
$ agent-tui type -h
Type literal text

Usage: agent-tui type [OPTIONS] <TEXT>

Arguments:
  <TEXT>  Text to type
...

```

## Wait short help

```console
$ agent-tui wait -h
Wait for a condition to be met before continuing

Usage: agent-tui wait [OPTIONS] [TEXT]

Arguments:
  [TEXT]  [..]
...

```

## Kill short help

```console
$ agent-tui kill -h
Kill the TUI application

Usage: agent-tui kill [OPTIONS]
...

```

## Sessions short help

```console
$ agent-tui sessions -h
List all active sessions

Usage: agent-tui sessions [OPTIONS]
...

```

## Health short help

```console
$ agent-tui health -h
Check daemon health and connection status

Usage: agent-tui health [OPTIONS]

Options:
  -v, --verbose[..]
...

```

## Scroll short help

```console
$ agent-tui scroll -h
Scroll in a direction

Usage: agent-tui scroll [OPTIONS] <DIRECTION>

Arguments:
  <DIRECTION>  Direction to scroll [possible values: up, down, left, right]

Options:
  -a, --amount <AMOUNT>    Amount to scroll (default: 5) [default: 5]
  -e, --element <ELEMENT>  Element to scroll within (optional)
...

```

## Find short help

```console
$ agent-tui find -h
Find elements by semantic properties (role, name, text)

Usage: agent-tui find [OPTIONS]

Options:
      --role <ROLE>  [..]
      --name <NAME>  [..]
      --text <TEXT>  [..]
...

```

## Assert short help

```console
$ agent-tui assert -h
Assert a condition for testing/scripting

Usage: agent-tui assert [OPTIONS] <CONDITION>

Arguments:
  <CONDITION>  [..]
...

```

## Get short help

```console
$ agent-tui get -h
Get information about elements or session

Usage: agent-tui get [OPTIONS] <COMMAND>

Commands:
  text     Get the text content of an element
  value    Get the value of an input element
  focused  Get the currently focused element
  title    Get the session title/command
...

```

## Is short help

```console
$ agent-tui is -h
Check element state (visible, focused, enabled, checked)

Usage: agent-tui is [OPTIONS] <COMMAND>

Commands:
  visible  Check if an element is visible
  focused  Check if an element is focused
  enabled  Check if an element is enabled (not disabled)
  checked  Check if a checkbox or radio button is checked
...

```

## Resize short help

```console
$ agent-tui resize -h
Resize the terminal window[..]

Usage: agent-tui resize [OPTIONS]

Options:
      --cols <COLS>[..]
      --rows <ROWS>[..]
...

```

## Env short help

```console
$ agent-tui env -h
Show environment diagnostics

Usage: agent-tui env [OPTIONS]
...

```

## Attach short help

```console
$ agent-tui attach -h
Attach to an existing session[..]

Usage: agent-tui attach [OPTIONS] <SESSION_ID>

Arguments:
  <SESSION_ID>  Session ID to attach to

Options:
  -i, --interactive[..]
...

```

## Demo short help

```console
$ agent-tui demo -h
Start built-in demo TUI for testing element detection

Usage: agent-tui demo [OPTIONS]
...

```

## Toggle short help

```console
$ agent-tui toggle -h
Toggle a checkbox or radio button

Usage: agent-tui toggle [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Check short help

```console
$ agent-tui check -h
Check a checkbox[..]

Usage: agent-tui check [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Uncheck short help

```console
$ agent-tui uncheck -h
Uncheck a checkbox[..]

Usage: agent-tui uncheck [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Clear short help

```console
$ agent-tui clear -h
Clear an input field

Usage: agent-tui clear [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Focus short help

```console
$ agent-tui focus -h
Focus a specific element

Usage: agent-tui focus [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Select short help

```console
$ agent-tui select -h
Select an option in a dropdown/select element

Usage: agent-tui select [OPTIONS] <ref> <OPTION>

Arguments:
  <ref>     [..]
  <OPTION>  [..]
...

```

## Multiselect short help

```console
$ agent-tui multiselect -h
Select multiple options in a multi-select list

Usage: agent-tui multiselect [OPTIONS] <ref> <OPTIONS>...

Arguments:
  <ref>  [..]
...

```

## Count short help

```console
$ agent-tui count -h
Count elements matching criteria

Usage: agent-tui count [OPTIONS]

Options:
      --role <ROLE>  [..]
      --name <NAME>  [..]
      --text <TEXT>  [..]
...

```

## Screenshot short help

```console
$ agent-tui screenshot -h
Take a text screenshot of the terminal

Usage: agent-tui screenshot [OPTIONS]

Options:
      --strip-ansi      [..]
      --include-cursor  [..]
...

```

## Trace short help

```console
$ agent-tui trace -h
Show recent interaction trace

Usage: agent-tui trace [OPTIONS]

Options:
  -c, --count <COUNT>  [..]
...

```

## Console short help

```console
$ agent-tui console -h
Show console/terminal output

Usage: agent-tui console [OPTIONS]

Options:
  -n, --lines <LINES>  [..]
...

```

## Errors short help

```console
$ agent-tui errors -h
Show captured errors (stderr, signals, exit codes)

Usage: agent-tui errors [OPTIONS]

Options:
  -n, --count <COUNT>  [..]
...

```

## Cleanup short help

```console
$ agent-tui cleanup -h
Clean up stale sessions

Usage: agent-tui cleanup [OPTIONS]

Options:
      --all  [..]
...

```

## Restart short help

```console
$ agent-tui restart -h
Restart the TUI application (kill + respawn with same command)

Usage: agent-tui restart [OPTIONS]
...

```

## Version command short help

```console
$ agent-tui version -h
Show version information for CLI and daemon

Usage: agent-tui version [OPTIONS]
...

```

## Completions short help

```console
$ agent-tui completions -h
Generate shell completion scripts[..]

Usage: agent-tui completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]
...

```

## Double-click short help

```console
$ agent-tui dblclick -h
Double-click an element by ref

Usage: agent-tui dblclick [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Keydown short help

```console
$ agent-tui keydown -h
Hold a key down (for modifier sequences)

Usage: agent-tui keydown [OPTIONS] <KEY>

Arguments:
  <KEY>  [..]
...

```

## Keyup short help

```console
$ agent-tui keyup -h
Release a held key (for modifier sequences)

Usage: agent-tui keyup [OPTIONS] <KEY>

Arguments:
  <KEY>  [..]
...

```

## Record-start short help

```console
$ agent-tui record-start -h
Start recording a session

Usage: agent-tui record-start [OPTIONS]
...

```

## Record-stop short help

```console
$ agent-tui record-stop -h
Stop recording and save to file

Usage: agent-tui record-stop [OPTIONS]

Options:
  -o, --output <OUTPUT>  [..]
...

```

## Record-status short help

```console
$ agent-tui record-status -h
Check recording status

Usage: agent-tui record-status [OPTIONS]
...

```

## Selectall short help

```console
$ agent-tui selectall -h
Select all text in an element (agent-browser parity)

Usage: agent-tui selectall [OPTIONS] <ref>

Arguments:
  <ref>  [..]
...

```

## Scrollintoview short help

```console
$ agent-tui scrollintoview -h
Scroll until element is visible (agent-browser parity)

Usage: agent-tui scrollintoview [OPTIONS] <ref>
...

```
