# Terminal Raw I/O and TTY Patterns

Patterns drawn from production Rust projects: [alacritty](https://github.com/alacritty/alacritty) (56k stars), [crossterm](https://github.com/crossterm-rs/crossterm) (3k stars), [console](https://github.com/console-rs/console) (1.8k stars), [ratatui](https://github.com/ratatui-org/ratatui) (11k stars).

## Table of Contents
1. [Terminal Detection](#1-terminal-detection)
2. [Raw Mode with Termios](#2-raw-mode-with-termios)
3. [Terminal Size Queries](#3-terminal-size-queries)
4. [RAII Terminal State](#4-raii-terminal-state)
5. [Stdin/Stdout Bridging](#5-stdinstdout-bridging)
6. [Event Loop Integration (alacritty)](#6-event-loop-integration-alacritty)
7. [ANSI Escape Sequences](#7-ansi-escape-sequences)
8. [Alternate Screen Buffer](#8-alternate-screen-buffer)

---

## 1. Terminal Detection

Check if file descriptor is a terminal:

```rust
use std::os::unix::io::AsRawFd;

pub fn is_terminal<T: AsRawFd>(fd: &T) -> bool {
    unsafe { libc::isatty(fd.as_raw_fd()) != 0 }
}

pub fn is_stdin_terminal() -> bool {
    is_terminal(&std::io::stdin())
}

pub fn is_stdout_terminal() -> bool {
    is_terminal(&std::io::stdout())
}

pub fn is_stderr_terminal() -> bool {
    is_terminal(&std::io::stderr())
}

// Comprehensive check for interactive mode
pub fn is_interactive() -> bool {
    is_stdin_terminal() && is_stdout_terminal()
}
```

## 2. Raw Mode with Termios

Enable raw mode for unbuffered input:

```rust
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;

pub struct RawModeGuard {
    fd: i32,
    original: libc::termios,
}

impl RawModeGuard {
    pub fn enable<T: AsRawFd>(fd: &T) -> io::Result<Self> {
        let fd = fd.as_raw_fd();

        // Get current termios
        let mut termios = MaybeUninit::uninit();
        if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let original = unsafe { termios.assume_init() };

        // Create raw mode settings
        let mut raw = original.clone();
        unsafe { libc::cfmakeraw(&mut raw) };

        // Apply raw mode
        if unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { fd, original })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Restore original settings
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSADRAIN, &self.original);
        }
    }
}

// Manual raw mode setup (for platforms without cfmakeraw)
pub fn make_raw_manual(termios: &mut libc::termios) {
    // Input flags
    termios.c_iflag &= !(
        libc::IGNBRK |  // Ignore break condition
        libc::BRKINT |  // Signal interrupt on break
        libc::PARMRK |  // Mark parity errors
        libc::ISTRIP |  // Strip eighth bit
        libc::INLCR |   // Translate NL to CR
        libc::IGNCR |   // Ignore CR
        libc::ICRNL |   // Translate CR to NL
        libc::IXON      // Enable XON/XOFF flow control
    );

    // Output flags
    termios.c_oflag &= !libc::OPOST;  // Disable output processing

    // Local flags
    termios.c_lflag &= !(
        libc::ECHO |    // Disable echo
        libc::ECHONL |  // Disable newline echo
        libc::ICANON |  // Disable canonical mode
        libc::ISIG |    // Disable signal generation
        libc::IEXTEN    // Disable extended processing
    );

    // Control flags
    termios.c_cflag &= !(libc::CSIZE | libc::PARENB);
    termios.c_cflag |= libc::CS8;  // 8 bits per byte

    // Control characters
    termios.c_cc[libc::VMIN] = 1;   // Minimum bytes to read
    termios.c_cc[libc::VTIME] = 0;  // No timeout
}
```

## 3. Terminal Size Queries

Get terminal dimensions:

```rust
use std::os::unix::io::AsRawFd;

#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

pub fn terminal_size<T: AsRawFd>(fd: &T) -> Option<TerminalSize> {
    let mut winsize: libc::winsize = unsafe { std::mem::zeroed() };

    let result = unsafe {
        libc::ioctl(fd.as_raw_fd(), libc::TIOCGWINSZ, &mut winsize)
    };

    if result == 0 && winsize.ws_row > 0 && winsize.ws_col > 0 {
        Some(TerminalSize {
            rows: winsize.ws_row,
            cols: winsize.ws_col,
            pixel_width: winsize.ws_xpixel,
            pixel_height: winsize.ws_ypixel,
        })
    } else {
        None
    }
}

pub fn set_terminal_size<T: AsRawFd>(
    fd: &T,
    rows: u16,
    cols: u16,
) -> io::Result<()> {
    let winsize = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe {
        libc::ioctl(fd.as_raw_fd(), libc::TIOCSWINSZ, &winsize)
    };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

// Fallback to environment variables
pub fn terminal_size_from_env() -> Option<TerminalSize> {
    let cols = std::env::var("COLUMNS").ok()?.parse().ok()?;
    let rows = std::env::var("LINES").ok()?.parse().ok()?;

    Some(TerminalSize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}
```

## 4. RAII Terminal State

Scopeguard pattern for terminal mode restoration:

```rust
use scopeguard::guard;

pub struct Terminal {
    stdin: std::io::Stdin,
    stdout: std::io::Stdout,
    original_termios: Option<libc::termios>,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            stdin: std::io::stdin(),
            stdout: std::io::stdout(),
            original_termios: None,
        }
    }

    pub fn enable_raw_mode(&mut self) -> io::Result<()> {
        if self.original_termios.is_some() {
            return Ok(()); // Already in raw mode
        }

        let fd = self.stdin.as_raw_fd();
        let mut termios = MaybeUninit::uninit();

        if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let original = unsafe { termios.assume_init() };
        self.original_termios = Some(original);

        let mut raw = original.clone();
        unsafe { libc::cfmakeraw(&mut raw) };

        if unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &raw) } != 0 {
            self.original_termios = None;
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    pub fn disable_raw_mode(&mut self) -> io::Result<()> {
        if let Some(original) = self.original_termios.take() {
            let fd = self.stdin.as_raw_fd();
            if unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &original) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.disable_raw_mode();
    }
}

// Scoped raw mode
pub fn with_raw_mode<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> T,
{
    let stdin = std::io::stdin();
    let guard = RawModeGuard::enable(&stdin)?;

    let _cleanup = guard;  // Ensures restoration on drop
    Ok(f())
}
```

## 5. Stdin/Stdout Bridging

Bridge between PTY and terminal:

```rust
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};

pub struct TerminalBridge {
    pty_master: RawFd,
    stdin: RawFd,
    stdout: RawFd,
}

impl TerminalBridge {
    pub fn new(pty_master: RawFd) -> Self {
        Self {
            pty_master,
            stdin: std::io::stdin().as_raw_fd(),
            stdout: std::io::stdout().as_raw_fd(),
        }
    }

    pub fn run(&self) -> io::Result<()> {
        use nix::poll::{poll, PollFd, PollFlags};

        let mut buf = [0u8; 4096];
        let poll_in = PollFlags::POLLIN;

        loop {
            let mut fds = [
                PollFd::new(self.stdin, poll_in),
                PollFd::new(self.pty_master, poll_in),
            ];

            match poll(&mut fds, -1) {
                Ok(_) => {}
                Err(nix::errno::Errno::EINTR) => continue,
                Err(e) => return Err(e.into()),
            }

            // stdin -> pty
            if fds[0].revents().map(|r| r.contains(poll_in)).unwrap_or(false) {
                let n = nix::unistd::read(self.stdin, &mut buf)?;
                if n == 0 {
                    break;
                }
                nix::unistd::write(self.pty_master, &buf[..n])?;
            }

            // pty -> stdout
            if fds[1].revents().map(|r| r.contains(poll_in)).unwrap_or(false) {
                let n = nix::unistd::read(self.pty_master, &mut buf)?;
                if n == 0 {
                    break;
                }
                nix::unistd::write(self.stdout, &buf[..n])?;
            }

            // Check for hangup
            let hangup = PollFlags::POLLHUP;
            if fds[1].revents().map(|r| r.contains(hangup)).unwrap_or(false) {
                break;
            }
        }

        Ok(())
    }
}

// Non-blocking read helper
pub fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}
```

## 6. Event Loop Integration (alacritty)

Action context for coordinated event handling:

```rust
use std::collections::HashMap;

pub struct ActionContext<'a> {
    pub terminal: &'a mut Terminal,
    pub clipboard: &'a mut Clipboard,
    pub dirty: &'a mut bool,
    pub scheduler: &'a mut Scheduler,
}

pub trait EventHandler {
    fn handle_key(&mut self, key: Key, mods: Modifiers);
    fn handle_mouse(&mut self, event: MouseEvent);
    fn handle_resize(&mut self, size: TerminalSize);
}

impl<'a> EventHandler for ActionContext<'a> {
    fn handle_key(&mut self, key: Key, mods: Modifiers) {
        // Cancel any pending cursor blink
        self.scheduler.cancel_cursor_blink();

        // Process key
        let input = self.terminal.process_key(key, mods);

        if !input.is_empty() {
            self.terminal.write_to_pty(&input);
        }

        *self.dirty = true;
    }

    fn handle_mouse(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Scroll(delta) => {
                self.terminal.scroll(delta);
            }
            MouseEvent::Click(pos, button) => {
                self.terminal.handle_click(pos, button);
            }
            _ => {}
        }
        *self.dirty = true;
    }

    fn handle_resize(&mut self, size: TerminalSize) {
        self.terminal.resize(size);
        *self.dirty = true;
    }
}

// Event filter pattern
pub fn should_skip_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Synthetic(_) |
        Event::ThemeChanged |
        Event::ScaleFactorChanged { .. }
    )
}
```

## 7. ANSI Escape Sequences

Common ANSI sequences for terminal control:

```rust
use std::io::{self, Write};

pub struct AnsiWriter<W: Write> {
    writer: W,
}

impl<W: Write> AnsiWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    // Cursor control
    pub fn move_cursor(&mut self, row: u16, col: u16) -> io::Result<()> {
        write!(self.writer, "\x1b[{};{}H", row + 1, col + 1)
    }

    pub fn move_cursor_up(&mut self, n: u16) -> io::Result<()> {
        write!(self.writer, "\x1b[{}A", n)
    }

    pub fn move_cursor_down(&mut self, n: u16) -> io::Result<()> {
        write!(self.writer, "\x1b[{}B", n)
    }

    pub fn save_cursor(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b7")
    }

    pub fn restore_cursor(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b8")
    }

    pub fn hide_cursor(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[?25l")
    }

    pub fn show_cursor(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[?25h")
    }

    // Screen control
    pub fn clear_screen(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[2J")
    }

    pub fn clear_line(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[2K")
    }

    pub fn clear_to_end_of_line(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[K")
    }

    // Style
    pub fn reset_style(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[0m")
    }

    pub fn set_bold(&mut self) -> io::Result<()> {
        write!(self.writer, "\x1b[1m")
    }

    pub fn set_fg_color(&mut self, r: u8, g: u8, b: u8) -> io::Result<()> {
        write!(self.writer, "\x1b[38;2;{};{};{}m", r, g, b)
    }

    pub fn set_bg_color(&mut self, r: u8, g: u8, b: u8) -> io::Result<()> {
        write!(self.writer, "\x1b[48;2;{};{};{}m", r, g, b)
    }

    pub fn set_fg_indexed(&mut self, index: u8) -> io::Result<()> {
        write!(self.writer, "\x1b[38;5;{}m", index)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

// Alternate screen buffer
pub fn enter_alternate_screen<W: Write>(w: &mut W) -> io::Result<()> {
    write!(w, "\x1b[?1049h")
}

pub fn leave_alternate_screen<W: Write>(w: &mut W) -> io::Result<()> {
    write!(w, "\x1b[?1049l")
}

// Mouse tracking
pub fn enable_mouse_capture<W: Write>(w: &mut W) -> io::Result<()> {
    write!(w, "\x1b[?1000h\x1b[?1002h\x1b[?1015h\x1b[?1006h")
}

pub fn disable_mouse_capture<W: Write>(w: &mut W) -> io::Result<()> {
    write!(w, "\x1b[?1006l\x1b[?1015l\x1b[?1002l\x1b[?1000l")
}
```

## 8. Alternate Screen Buffer

RAII wrapper for alternate screen:

```rust
use std::io::{self, Write};

pub struct AlternateScreen<W: Write> {
    writer: W,
    active: bool,
}

impl<W: Write> AlternateScreen<W> {
    pub fn enter(mut writer: W) -> io::Result<Self> {
        write!(writer, "\x1b[?1049h")?;
        writer.flush()?;

        Ok(Self {
            writer,
            active: true,
        })
    }

    pub fn leave(&mut self) -> io::Result<()> {
        if self.active {
            write!(self.writer, "\x1b[?1049l")?;
            self.writer.flush()?;
            self.active = false;
        }
        Ok(())
    }

    pub fn writer(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<W: Write> Drop for AlternateScreen<W> {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

// Combined terminal setup
pub struct TerminalSetup<W: Write> {
    writer: W,
    raw_guard: Option<RawModeGuard>,
}

impl<W: Write> TerminalSetup<W> {
    pub fn new(mut writer: W) -> io::Result<Self> {
        // Enter alternate screen
        write!(writer, "\x1b[?1049h")?;

        // Hide cursor
        write!(writer, "\x1b[?25l")?;

        // Enable mouse
        write!(writer, "\x1b[?1000h\x1b[?1002h\x1b[?1006h")?;

        writer.flush()?;

        // Enable raw mode
        let raw_guard = RawModeGuard::enable(&std::io::stdin()).ok();

        Ok(Self { writer, raw_guard })
    }

    pub fn teardown(&mut self) -> io::Result<()> {
        // Disable mouse
        write!(self.writer, "\x1b[?1006l\x1b[?1002l\x1b[?1000l")?;

        // Show cursor
        write!(self.writer, "\x1b[?25h")?;

        // Leave alternate screen
        write!(self.writer, "\x1b[?1049l")?;

        self.writer.flush()?;

        // Raw mode guard handles restoration on drop
        self.raw_guard = None;

        Ok(())
    }
}

impl<W: Write> Drop for TerminalSetup<W> {
    fn drop(&mut self) {
        let _ = self.teardown();
    }
}
```

---

## Related Patterns

- [Daemon Patterns](daemon-rpc-patterns.md) - PTY handling in services
- [TUI Patterns](tui-patterns.md) - Terminal UI applications
- [Visual Processing](visual-processing.md) - Grid-based screen rendering
- [Concurrency](concurrency.md) - Async I/O and signal handling
