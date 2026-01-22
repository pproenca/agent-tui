# Health Command Tests

Tests for the health command with various options.

## Basic health check

```console
$ agent-tui health
Daemon status: healthy
  PID: [..]
  Uptime: [..]
  Sessions: [..]
  Version: [..]

```

## Health with verbose flag

```console
$ agent-tui health -v
Daemon status: healthy
  PID: [..]
  Uptime: [..]
  Sessions: [..]
  Version: [..]

Connection:
  Socket: [..]
  PID file: [..]

```

## Health with JSON format

```console
$ agent-tui -f json health
{
...
}

```

## Health with --json shorthand

```console
$ agent-tui --json health
{
...
}

```
