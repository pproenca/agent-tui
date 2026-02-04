use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub struct RealTestHarness {
    _temp_dir: TempDir,
    socket_path: PathBuf,
    daemon: Option<Child>,
}

impl RealTestHarness {
    pub fn new() -> Self {
        let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o777));
        }
        let socket_path = temp_dir.path().join("agent-tui.sock");

        let mut daemon_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        daemon_cmd
            .env("AGENT_TUI_SOCKET", &socket_path)
            .args(["daemon", "start", "--foreground"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut daemon = daemon_cmd.spawn().expect("Failed to start daemon");
        let start_timeout =
            timeout_from_env("AGENT_TUI_E2E_START_TIMEOUT_MS", Duration::from_secs(5));
        wait_for_socket(&socket_path, &mut daemon, start_timeout);

        Self {
            _temp_dir: temp_dir,
            socket_path,
            daemon: Some(daemon),
        }
    }

    pub fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        cmd.env("AGENT_TUI_SOCKET", &self.socket_path);
        cmd
    }

    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }

    pub fn stop(&mut self) {
        self.stop_daemon_inner();
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) {
        self.wait_for_exit_inner(timeout);
    }

    fn stop_daemon_inner(&mut self) {
        let _ = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"))
            .env("AGENT_TUI_SOCKET", &self.socket_path)
            .args(["daemon", "stop", "--force"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let stop_timeout =
            timeout_from_env("AGENT_TUI_E2E_STOP_TIMEOUT_MS", Duration::from_secs(3));
        self.wait_for_exit_inner(stop_timeout);
    }

    fn is_daemon_exited(&mut self) -> bool {
        match self.daemon.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => true,
                Ok(None) => false,
                Err(_) => true,
            },
            None => true,
        }
    }

    fn wait_for_exit_inner(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.is_daemon_exited() {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }

        if !self.is_daemon_exited()
            && let Some(child) = self.daemon.as_mut()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for RealTestHarness {
    fn drop(&mut self) {
        self.stop_daemon_inner();
    }
}

fn wait_for_socket(socket_path: &Path, daemon: &mut Child, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if socket_path.exists() {
            return;
        }
        if let Ok(Some(status)) = daemon.try_wait() {
            panic!("Daemon exited early with status {}", status);
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "Timed out waiting for daemon socket to appear at {}",
        socket_path.display()
    );
}

fn timeout_from_env(var: &str, default: Duration) -> Duration {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}
