# Project Agent Guidelines

## Terminal IO (cross-platform)
- Use crossterm commands (`execute!`/`queue!`) for cursor movement, screen clearing, styles, and terminal modes.
- Avoid raw ANSI/escape byte sequences (`\x1b...`) in application code; prefer crossterm for portability and correctness.
- When drawing transient UI, use save/restore cursor commands and always `flush()` after queued output.
- Pair enable/disable commands (e.g., bracketed paste, focus change, mouse capture, alternate screen) to restore terminal state reliably.
