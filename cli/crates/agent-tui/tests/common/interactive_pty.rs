//! Interactive PTY test runner for attach/wizard style commands.

use portable_pty::Child;
use portable_pty::CommandBuilder;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct InteractivePtyRunner {
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    output: Vec<u8>,
    reader_join: Option<thread::JoinHandle<()>>,
}

impl InteractivePtyRunner {
    pub fn spawn(binary: &Path, args: &[&str], env_vars: &[(String, String)]) -> io::Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io::Error::other)?;

        let mut cmd = CommandBuilder::new(binary.to_string_lossy().to_string());
        cmd.args(args);
        cmd.env("TERM", "xterm-256color");
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;

        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;

        let (output_tx, output_rx) = mpsc::channel();
        let reader_join = thread::Builder::new()
            .name("interactive-pty-reader".to_string())
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if output_tx.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })
            .map_err(io::Error::other)?;

        Ok(Self {
            child,
            _master: pair.master,
            writer,
            output_rx,
            output: Vec::new(),
            reader_join: Some(reader_join),
        })
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    pub fn send_text(&mut self, text: &str) -> io::Result<()> {
        self.send_bytes(text.as_bytes())
    }

    pub fn read_until_contains(&mut self, needle: &str, timeout: Duration) -> io::Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if self.output_as_string().contains(needle) {
                return Ok(self.output_as_string());
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "Timed out waiting for '{needle}'. Output so far: {}",
                        self.output_as_string()
                    ),
                ));
            }

            let remaining = deadline.saturating_duration_since(now);
            let poll = remaining.min(Duration::from_millis(100));
            match self.output_rx.recv_timeout(poll) {
                Ok(chunk) => self.output.extend_from_slice(&chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if self.output_as_string().contains(needle) {
                        return Ok(self.output_as_string());
                    }
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "PTY output stream closed before expected content appeared",
                    ));
                }
            }
        }
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> io::Result<portable_pty::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().map_err(io::Error::other)? {
                self.drain_output(Duration::from_millis(150));
                return Ok(status);
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "PTY process did not exit within timeout",
                ));
            }
            self.drain_output(Duration::from_millis(25));
        }
    }

    pub fn output_as_string(&self) -> String {
        String::from_utf8_lossy(&self.output).to_string()
    }

    fn drain_output(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if Instant::now() >= deadline {
                break;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let poll = remaining.min(Duration::from_millis(20));
            match self.output_rx.recv_timeout(poll) {
                Ok(chunk) => self.output.extend_from_slice(&chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}

fn join_reader_thread(reader_join: &mut Option<thread::JoinHandle<()>>) {
    if let Some(handle) = reader_join.take() {
        let _ = handle.join();
    }
}

impl Drop for InteractivePtyRunner {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        join_reader_thread(&mut self.reader_join);
    }
}

#[cfg(test)]
mod tests {
    use super::join_reader_thread;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn join_reader_thread_waits_for_completion() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_for_thread = Arc::clone(&completed);
        let mut reader_join = Some(thread::spawn(move || {
            thread::sleep(Duration::from_millis(25));
            completed_for_thread.store(true, Ordering::SeqCst);
        }));

        join_reader_thread(&mut reader_join);

        assert!(completed.load(Ordering::SeqCst));
        assert!(reader_join.is_none());
    }
}
