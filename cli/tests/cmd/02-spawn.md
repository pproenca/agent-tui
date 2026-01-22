# Spawn Command Tests

Tests for spawning TUI applications.

## Spawn basic command

```console
$ agent-tui spawn bash
Session started: [..]
  PID: [..]

```

## Spawn with custom dimensions

```console
$ agent-tui spawn --cols 80 --rows 24 bash
Session started: [..]
  PID: [..]

```

## Spawn with JSON output

```console
$ agent-tui -f json spawn bash
{
...
}

```

## Spawn with working directory

```console
$ agent-tui spawn -d /tmp bash
Session started: [..]
  PID: [..]

```

## Spawn with arguments

```console
$ agent-tui spawn bash -- -c "echo hello"
Session started: [..]
  PID: [..]

```
