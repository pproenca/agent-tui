//! Real daemon test harness.
#![expect(
    clippy::print_stderr,
    reason = "Real E2E cleanup diagnostics must remain visible when a test is already panicking."
)]

use assert_cmd::Command;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DAEMON_OUTPUT_LIMIT: usize = 64 * 1024;
const DAEMON_TERMINATE_TIMEOUT: Duration = Duration::from_secs(1);

pub struct RealTestHarness {
    _temp_dir: TempDir,
    socket_path: PathBuf,
    env_vars: Vec<(String, String)>,
    daemon: Option<Child>,
    daemon_status: Option<ExitStatus>,
    daemon_output: DaemonOutputCapture,
    capture_threads: Vec<JoinHandle<()>>,
}

impl RealTestHarness {
    pub fn new() -> Self {
        let temp_dir = TempDir::new_in("/tmp")
            .unwrap_or_else(|err| panic!("Failed to create temp dir for real E2E harness: {err}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o777));
        }
        let socket_path = temp_dir.path().join("agent-tui.sock");
        let env_vars = runtime_env_vars(temp_dir.path(), &socket_path);
        let daemon_output = DaemonOutputCapture::default();

        let mut daemon_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        apply_env_vars(&mut daemon_cmd, &env_vars);
        daemon_cmd
            .env("AGENT_TUI_DAEMON_FOREGROUND", "1")
            .args(["daemon", "start"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_daemon_process_group(&mut daemon_cmd);

        let mut daemon = daemon_cmd.spawn().unwrap_or_else(|err| {
            panic!(
                "Failed to start daemon for real E2E harness: {err}\n{}",
                startup_diagnostic(&socket_path, None, &daemon_output)
            )
        });

        let mut capture_threads = Vec::new();
        if let Some(stdout) = daemon.stdout.take() {
            capture_threads.push(spawn_output_capture(
                "daemon-stdout",
                stdout,
                daemon_output.clone(),
                OutputStream::Stdout,
            ));
        }
        if let Some(stderr) = daemon.stderr.take() {
            capture_threads.push(spawn_output_capture(
                "daemon-stderr",
                stderr,
                daemon_output.clone(),
                OutputStream::Stderr,
            ));
        }

        let start_timeout =
            timeout_from_env("AGENT_TUI_E2E_START_TIMEOUT_MS", Duration::from_secs(5));
        wait_for_daemon_ready(
            &socket_path,
            &env_vars,
            &mut daemon,
            &mut capture_threads,
            start_timeout,
            &daemon_output,
        );

        Self {
            _temp_dir: temp_dir,
            socket_path,
            env_vars,
            daemon: Some(daemon),
            daemon_status: None,
            daemon_output,
            capture_threads,
        }
    }

    pub fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }
        cmd
    }

    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command()
            .args(args)
            .assert()
            .append_context("real-daemon", self.daemon_diagnostic())
            .append_context("real-daemon-command", format_args_for_diagnostic(args))
    }

    pub fn output(&self, args: &[&str]) -> std::process::Output {
        let mut cmd = self.cli_command();
        cmd.args(args);
        cmd.output().unwrap_or_else(|err| {
            panic!(
                "failed to execute real daemon command {}\nerror: {err}\n{}",
                format_args_for_diagnostic(args),
                self.daemon_diagnostic()
            )
        })
    }

    pub fn command_failure_diagnostic(
        &self,
        args: &[&str],
        output: &std::process::Output,
    ) -> String {
        format_command_failure(&self.socket_path, args, output, &self.daemon_output)
    }

    pub fn env_vars(&self) -> Vec<(String, String)> {
        self.env_vars.clone()
    }

    pub fn stop(&mut self) {
        self.stop_daemon_inner()
            .unwrap_or_else(|err| panic!("real daemon cleanup failed\n{err}"));
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) {
        self.wait_for_exit_inner(timeout)
            .unwrap_or_else(|err| panic!("real daemon did not exit cleanly\n{err}"));
    }

    fn stop_daemon_inner(&mut self) -> Result<(), String> {
        let mut stop_cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        apply_env_vars(&mut stop_cmd, &self.env_vars);
        let stop_result = stop_cmd
            .args(["daemon", "stop", "--force", "--yes"])
            .output();

        let stop_timeout =
            timeout_from_env("AGENT_TUI_E2E_STOP_TIMEOUT_MS", Duration::from_secs(3));
        self.wait_for_exit_inner(stop_timeout).map_err(|err| {
            let stop_context = match stop_result {
                Ok(output) => format_command_failure(
                    &self.socket_path,
                    &["daemon", "stop", "--force", "--yes"],
                    &output,
                    &self.daemon_output,
                ),
                Err(err) => format!(
                    "failed to execute daemon stop command for socket {}: {err}",
                    self.socket_path.display()
                ),
            };
            format!("{stop_context}\n{err}")
        })
    }

    fn is_daemon_exited(&mut self) -> Result<bool, String> {
        match self.daemon.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => {
                    self.daemon_status = Some(status);
                    self.daemon = None;
                    join_capture_thread_handles(&mut self.capture_threads)?;
                    Ok(true)
                }
                Ok(None) => Ok(false),
                Err(err) => Err(format!(
                    "failed to poll daemon process {} for socket {}: {err}",
                    child.id(),
                    self.socket_path.display()
                )),
            },
            None => Ok(true),
        }
    }

    fn wait_for_exit_inner(&mut self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.is_daemon_exited()? {
                return Ok(());
            }
            thread::park_timeout(Duration::from_millis(50));
        }

        if self.is_daemon_exited()? {
            return Ok(());
        }

        Err(self.terminate_after_timeout(timeout))
    }

    fn terminate_after_timeout(&mut self, timeout: Duration) -> String {
        let mut messages = vec![format!(
            "daemon did not exit within {timeout:?} for socket {}",
            self.socket_path.display()
        )];

        if let Some(child) = self.daemon.as_ref() {
            match terminate_process_group(child) {
                Ok(()) => messages.push(format!(
                    "sent SIGTERM to daemon process group for pid {}",
                    child.id()
                )),
                Err(err) => messages.push(err),
            }
        }

        let group_deadline = Instant::now() + DAEMON_TERMINATE_TIMEOUT;
        while Instant::now() < group_deadline {
            match self.is_daemon_exited() {
                Ok(true) => {
                    messages.push("daemon exited after process group termination".to_string());
                    messages.push(self.daemon_diagnostic());
                    return messages.join("\n");
                }
                Ok(false) => thread::park_timeout(Duration::from_millis(50)),
                Err(err) => {
                    messages.push(err);
                    break;
                }
            }
        }

        if let Some(mut child) = self.daemon.take() {
            let pid = child.id();
            match child.kill() {
                Ok(()) => messages.push(format!("sent direct kill to daemon child pid {pid}")),
                Err(err) => messages.push(format!("failed to kill daemon child pid {pid}: {err}")),
            }
            match child.wait() {
                Ok(status) => {
                    self.daemon_status = Some(status);
                    messages.push(format!(
                        "daemon child pid {pid} reaped with status {status}"
                    ));
                }
                Err(err) => messages.push(format!("failed to reap daemon child pid {pid}: {err}")),
            }
        }

        if let Err(err) = join_capture_thread_handles(&mut self.capture_threads) {
            messages.push(err);
        }

        messages.push(self.daemon_diagnostic());
        messages.join("\n")
    }

    fn daemon_diagnostic(&self) -> String {
        startup_diagnostic(&self.socket_path, self.daemon_status, &self.daemon_output)
    }

    fn join_capture_threads(&mut self) -> Result<(), String> {
        join_capture_thread_handles(&mut self.capture_threads)
    }
}

impl Drop for RealTestHarness {
    fn drop(&mut self) {
        if let Err(err) = self.stop_daemon_inner() {
            if std::thread::panicking() {
                eprintln!("real daemon cleanup failed while test was already panicking\n{err}");
            } else {
                panic!("real daemon cleanup failed\n{err}");
            }
        }
    }
}

fn wait_for_daemon_ready(
    socket_path: &Path,
    env_vars: &[(String, String)],
    daemon: &mut Child,
    capture_threads: &mut Vec<JoinHandle<()>>,
    timeout: Duration,
    daemon_output: &DaemonOutputCapture,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if socket_path.exists() && daemon_accepts_requests(env_vars) {
            return;
        }
        if let Ok(Some(status)) = daemon.try_wait() {
            let capture_status = join_capture_thread_handles(capture_threads)
                .map(|()| String::new())
                .unwrap_or_else(|err| format!("\n{err}"));
            panic!(
                "Daemon exited early with status {status}\n{}{}",
                startup_diagnostic(socket_path, Some(status), daemon_output),
                capture_status
            );
        }
        thread::park_timeout(Duration::from_millis(50));
    }
    let cleanup = terminate_unready_daemon(daemon, capture_threads);
    panic!(
        "Timed out waiting for daemon readiness\n{}\n{}",
        startup_diagnostic(socket_path, None, daemon_output),
        cleanup
    );
}

fn daemon_accepts_requests(env_vars: &[(String, String)]) -> bool {
    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
    apply_env_vars(&mut cmd, env_vars);
    cmd.args(["--no-color", "sessions"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn timeout_from_env(var: &str, default: Duration) -> Duration {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

fn apply_env_vars(cmd: &mut StdCommand, env_vars: &[(String, String)]) {
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
}

pub(crate) fn runtime_env_vars(base_dir: &Path, socket_path: &Path) -> Vec<(String, String)> {
    vec![
        (
            "AGENT_TUI_SOCKET".to_string(),
            socket_path.to_string_lossy().into_owned(),
        ),
        (
            "AGENT_TUI_SESSION_STORE".to_string(),
            base_dir
                .join("sessions.jsonl")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "AGENT_TUI_WS_STATE".to_string(),
            base_dir.join("api.json").to_string_lossy().into_owned(),
        ),
        (
            "AGENT_TUI_UI_STATE".to_string(),
            base_dir.join("ui.json").to_string_lossy().into_owned(),
        ),
    ]
}

fn spawn_output_capture<R>(
    name: &'static str,
    mut reader: R,
    output: DaemonOutputCapture,
    stream: OutputStream,
) -> JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => output.push(stream, &buffer[..n]),
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        })
        .unwrap_or_else(|err| panic!("failed to spawn {name} capture thread: {err}"))
}

fn join_capture_thread_handles(capture_threads: &mut Vec<JoinHandle<()>>) -> Result<(), String> {
    let mut errors = Vec::new();
    for handle in capture_threads.drain(..) {
        if handle.join().is_err() {
            errors.push("daemon output capture thread panicked".to_string());
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn terminate_unready_daemon(
    daemon: &mut Child,
    capture_threads: &mut Vec<JoinHandle<()>>,
) -> String {
    let mut messages = vec![format!(
        "cleaning up unready daemon child pid {}",
        daemon.id()
    )];

    match terminate_process_group(daemon) {
        Ok(()) => messages.push(format!(
            "sent SIGTERM to daemon process group for pid {}",
            daemon.id()
        )),
        Err(err) => messages.push(err),
    }

    let group_deadline = Instant::now() + DAEMON_TERMINATE_TIMEOUT;
    let mut exited = false;
    while Instant::now() < group_deadline {
        match daemon.try_wait() {
            Ok(Some(status)) => {
                messages.push(format!(
                    "unready daemon child pid {} exited with status {status}",
                    daemon.id()
                ));
                exited = true;
                break;
            }
            Ok(None) => thread::park_timeout(Duration::from_millis(50)),
            Err(err) => {
                messages.push(format!(
                    "failed to poll unready daemon child pid {}: {err}",
                    daemon.id()
                ));
                break;
            }
        }
    }

    if !exited {
        match daemon.kill() {
            Ok(()) => messages.push(format!(
                "sent direct kill to unready daemon child pid {}",
                daemon.id()
            )),
            Err(err) => messages.push(format!(
                "failed to kill unready daemon child pid {}: {err}",
                daemon.id()
            )),
        }
        match daemon.wait() {
            Ok(status) => messages.push(format!(
                "unready daemon child pid {} reaped with status {status}",
                daemon.id()
            )),
            Err(err) => messages.push(format!(
                "failed to reap unready daemon child pid {}: {err}",
                daemon.id()
            )),
        }
    }

    if let Err(err) = join_capture_thread_handles(capture_threads) {
        messages.push(err);
    }

    messages.join("\n")
}

#[derive(Clone, Default)]
pub(crate) struct DaemonOutputCapture {
    inner: Arc<Mutex<DaemonOutputBuffers>>,
}

impl DaemonOutputCapture {
    pub(crate) fn push(&self, stream: OutputStream, bytes: &[u8]) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match stream {
            OutputStream::Stdout => inner.stdout.push(bytes),
            OutputStream::Stderr => inner.stderr.push(bytes),
        }
    }

    fn snapshot(&self) -> DaemonOutputSnapshot {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        DaemonOutputSnapshot {
            stdout: inner.stdout.to_lossy_string(),
            stderr: inner.stderr.to_lossy_string(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Default)]
struct DaemonOutputBuffers {
    stdout: BoundedBytes,
    stderr: BoundedBytes,
}

#[derive(Default)]
struct BoundedBytes {
    bytes: Vec<u8>,
    truncated: bool,
}

impl BoundedBytes {
    fn push(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
        if self.bytes.len() > DAEMON_OUTPUT_LIMIT {
            let overflow = self.bytes.len() - DAEMON_OUTPUT_LIMIT;
            self.bytes.drain(..overflow);
            self.truncated = true;
        }
    }

    fn to_lossy_string(&self) -> String {
        let rendered = String::from_utf8_lossy(&self.bytes);
        if self.truncated {
            format!("<truncated to last {DAEMON_OUTPUT_LIMIT} bytes>\n{rendered}")
        } else {
            rendered.into_owned()
        }
    }
}

struct DaemonOutputSnapshot {
    stdout: String,
    stderr: String,
}

impl DaemonOutputSnapshot {
    fn is_empty(&self) -> bool {
        self.stdout.is_empty() && self.stderr.is_empty()
    }
}

fn startup_diagnostic(
    socket_path: &Path,
    daemon_status: Option<ExitStatus>,
    daemon_output: &DaemonOutputCapture,
) -> String {
    let mut message = format!("real daemon socket: {}", socket_path.display());
    if let Some(status) = daemon_status {
        let _ = write!(message, "\nreal daemon status: {status}");
    }
    let output = daemon_output.snapshot();
    if output.is_empty() {
        message.push_str("\nreal daemon output: <empty>");
    } else {
        message.push_str("\nreal daemon stdout:\n");
        message.push_str(&output.stdout);
        message.push_str("\nreal daemon stderr:\n");
        message.push_str(&output.stderr);
    }
    message
}

pub(crate) fn format_command_failure(
    socket_path: &Path,
    args: &[&str],
    output: &std::process::Output,
    daemon_output: &DaemonOutputCapture,
) -> String {
    format!(
        "real daemon command failed\ncommand: agent-tui {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}\n{}",
        format_args_for_diagnostic(args),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        startup_diagnostic(socket_path, None, daemon_output)
    )
}

fn format_args_for_diagnostic(args: &[&str]) -> String {
    args.iter()
        .map(|arg| {
            if arg
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':'))
            {
                (*arg).to_string()
            } else {
                format!("{arg:?}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(unix)]
fn configure_daemon_process_group(cmd: &mut StdCommand) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn configure_daemon_process_group(_cmd: &mut StdCommand) {}

#[cfg(unix)]
fn terminate_process_group(child: &Child) -> Result<(), String> {
    let group = format!("-{}", child.id());
    let status = StdCommand::new("kill")
        .args(["-TERM", &group])
        .status()
        .map_err(|err| {
            format!(
                "failed to signal daemon process group for pid {}: {err}",
                child.id()
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to signal daemon process group for pid {}: kill exited with {status}",
            child.id()
        ))
    }
}

#[cfg(not(unix))]
fn terminate_process_group(child: &Child) -> Result<(), String> {
    Err(format!(
        "process group termination is not supported on this platform for pid {}",
        child.id()
    ))
}
