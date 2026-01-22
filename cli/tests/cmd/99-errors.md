# Error Case Tests

Tests for error handling and invalid inputs.

## Missing required argument for spawn

```console
$ agent-tui spawn
? failed
error: the following required arguments were not provided:
  <COMMAND>

Usage: agent-tui spawn <COMMAND> [ARGS]...

For more information, try '--help'.

```

## Missing required argument for click

```console
$ agent-tui click
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui click <ref>

For more information, try '--help'.

```

## Missing required arguments for fill

```console
$ agent-tui fill
? failed
error: the following required arguments were not provided:
  <ref>
  <VALUE>

Usage: agent-tui fill <ref> <VALUE>

For more information, try '--help'.

```

## Missing value argument for fill

```console
$ agent-tui fill @e1
? failed
error: the following required arguments were not provided:
  <VALUE>

Usage: agent-tui fill <ref> <VALUE>

For more information, try '--help'.

```

## Missing required argument for press

```console
$ agent-tui press
? failed
error: the following required arguments were not provided:
  <KEY>

Usage: agent-tui press <KEY>

For more information, try '--help'.

```

## Missing required argument for type

```console
$ agent-tui type
? failed
error: the following required arguments were not provided:
  <TEXT>

Usage: agent-tui type <TEXT>

For more information, try '--help'.

```

## Missing required argument for scroll

```console
$ agent-tui scroll
? failed
error: the following required arguments were not provided:
  <DIRECTION>

Usage: agent-tui scroll <DIRECTION>

For more information, try '--help'.

```

## Invalid scroll direction

```console
$ agent-tui scroll diagonal
? failed
error: invalid value 'diagonal' for '<DIRECTION>'
  [possible values: up, down, left, right]
...

```

## Invalid output format

```console
$ agent-tui -f xml health
? failed
error: invalid value 'xml' for '--format <FORMAT>'
  [possible values: text, json, tree]
...

```

## Missing argument for attach

```console
$ agent-tui attach
? failed
error: the following required arguments were not provided:
  <SESSION_ID>

Usage: agent-tui attach <SESSION_ID>

For more information, try '--help'.

```

## Missing argument for toggle

```console
$ agent-tui toggle
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui toggle <ref>

For more information, try '--help'.

```

## Missing argument for check

```console
$ agent-tui check
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui check <ref>

For more information, try '--help'.

```

## Missing argument for uncheck

```console
$ agent-tui uncheck
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui uncheck <ref>

For more information, try '--help'.

```

## Missing argument for clear

```console
$ agent-tui clear
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui clear <ref>

For more information, try '--help'.

```

## Missing argument for focus

```console
$ agent-tui focus
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui focus <ref>

For more information, try '--help'.

```

## Missing arguments for select

```console
$ agent-tui select
? failed
error: the following required arguments were not provided:
  <ref>
  <OPTION>

Usage: agent-tui select <ref> <OPTION>

For more information, try '--help'.

```

## Missing arguments for multiselect

```console
$ agent-tui multiselect
? failed
error: the following required arguments were not provided:
  <ref>
  <OPTIONS>...

Usage: agent-tui multiselect <ref> <OPTIONS>...

For more information, try '--help'.

```

## Missing argument for completions

```console
$ agent-tui completions
? failed
error: the following required arguments were not provided:
  <SHELL>

Usage: agent-tui completions <SHELL>

For more information, try '--help'.

```

## Invalid shell for completions

```console
$ agent-tui completions invalid
? failed
error: invalid value 'invalid' for '<SHELL>'
  [possible values: bash, elvish, fish, powershell, zsh]
...

```

## Missing argument for assert

```console
$ agent-tui assert
? failed
error: the following required arguments were not provided:
  <CONDITION>

Usage: agent-tui assert <CONDITION>

For more information, try '--help'.

```

## Get without subcommand errors with help text

```console
$ agent-tui get
? failed
Get information about elements or session

Usage: agent-tui get [OPTIONS] <COMMAND>

Commands:
  text     Get the text content of an element
  value    Get the value of an input element
  focused  Get the currently focused element
  title    Get the session title/command
...

```

## Missing argument for get text

```console
$ agent-tui get text
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui get text <ref>

For more information, try '--help'.

```

## Missing argument for get value

```console
$ agent-tui get value
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui get value <ref>

For more information, try '--help'.

```

## Is without subcommand errors with help text

```console
$ agent-tui is
? failed
Check element state (visible, focused, enabled, checked)

Usage: agent-tui is [OPTIONS] <COMMAND>

Commands:
  visible  Check if an element is visible
  focused  Check if an element is focused
  enabled  Check if an element is enabled (not disabled)
  checked  Check if a checkbox or radio button is checked
...

```

## Missing argument for is visible

```console
$ agent-tui is visible
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui is visible <ref>

For more information, try '--help'.

```

## Missing argument for is focused

```console
$ agent-tui is focused
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui is focused <ref>

For more information, try '--help'.

```

## Missing argument for is enabled

```console
$ agent-tui is enabled
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui is enabled <ref>

For more information, try '--help'.

```

## Missing argument for is checked

```console
$ agent-tui is checked
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui is checked <ref>

For more information, try '--help'.

```

## Missing argument for dblclick

```console
$ agent-tui dblclick
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui dblclick <ref>

For more information, try '--help'.

```

## Missing argument for keydown

```console
$ agent-tui keydown
? failed
error: the following required arguments were not provided:
  <KEY>

Usage: agent-tui keydown <KEY>

For more information, try '--help'.

```

## Missing argument for keyup

```console
$ agent-tui keyup
? failed
error: the following required arguments were not provided:
  <KEY>

Usage: agent-tui keyup <KEY>

For more information, try '--help'.

```

## Missing argument for selectall

```console
$ agent-tui selectall
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui selectall <ref>

For more information, try '--help'.

```

## Missing argument for scrollintoview

```console
$ agent-tui scrollintoview
? failed
error: the following required arguments were not provided:
  <ref>

Usage: agent-tui scrollintoview <ref>

For more information, try '--help'.

```

## Unknown command

```console
$ agent-tui unknown
? failed
error: unrecognized subcommand 'unknown'
...

```
