//! Process control utilities.

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Term,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    NotFound,
    NoPermission,
}

pub trait ProcessController: Send + Sync {
    fn check_process(&self, pid: u32) -> std::io::Result<ProcessStatus>;

    fn send_signal(&self, pid: u32, signal: Signal) -> std::io::Result<()>;
}

pub struct UnixProcessController;

impl ProcessController for UnixProcessController {
    fn check_process(&self, pid: u32) -> std::io::Result<ProcessStatus> {
        let pid_t: libc::pid_t = pid.try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID out of range")
        })?;

        // SAFETY: `kill` with signal 0 performs a permission check without sending a signal.
        // `pid_t` is derived from a validated u32 and is safe for libc calls.
        let result = unsafe { libc::kill(pid_t, 0) };
        if result == 0 {
            if is_defunct_process(pid) {
                return Ok(ProcessStatus::NotFound);
            }
            return Ok(ProcessStatus::Running);
        }

        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ESRCH) => Ok(ProcessStatus::NotFound),
            Some(libc::EPERM) => Ok(ProcessStatus::NoPermission),
            _ => Err(err),
        }
    }

    fn send_signal(&self, pid: u32, signal: Signal) -> std::io::Result<()> {
        let pid_t: libc::pid_t = pid.try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID out of range")
        })?;

        let sig = match signal {
            Signal::Term => libc::SIGTERM,
            Signal::Kill => libc::SIGKILL,
        };

        // SAFETY: `pid_t` is validated and `sig` is a valid libc signal constant.
        let result = unsafe { libc::kill(pid_t, sig) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

fn is_defunct_process(pid: u32) -> bool {
    matches!(process_status_code(pid), Some('Z' | 'X'))
}

fn process_status_code(pid: u32) -> Option<char> {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()?.trim().chars().next()
}

#[allow(clippy::expect_used)]
pub mod mock {
    use super::*;
    use crate::common::mutex_lock_or_recover;
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct MockProcessController {
        process_states: Mutex<HashMap<u32, ProcessStatus>>,
        signals_sent: Mutex<Vec<(u32, Signal)>>,
        check_error: Mutex<Option<std::io::Error>>,
        signal_error: Mutex<Option<std::io::Error>>,
        signal_kills_process_on: Option<Signal>,
    }

    impl Default for MockProcessController {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockProcessController {
        pub fn new() -> Self {
            Self {
                process_states: Mutex::new(HashMap::new()),
                signals_sent: Mutex::new(Vec::new()),
                check_error: Mutex::new(None),
                signal_error: Mutex::new(None),
                signal_kills_process_on: None,
            }
        }

        pub fn with_signal_kills_process(mut self) -> Self {
            self.signal_kills_process_on = Some(Signal::Term);
            self
        }

        pub fn with_signal_kills_process_on(mut self, signal: Signal) -> Self {
            self.signal_kills_process_on = Some(signal);
            self
        }

        pub fn with_process(self, pid: u32, status: ProcessStatus) -> Self {
            mutex_lock_or_recover(&self.process_states).insert(pid, status);
            self
        }

        pub fn with_check_error(self, error: std::io::Error) -> Self {
            *mutex_lock_or_recover(&self.check_error) = Some(error);
            self
        }

        pub fn with_signal_error(self, error: std::io::Error) -> Self {
            *mutex_lock_or_recover(&self.signal_error) = Some(error);
            self
        }

        pub fn signals_sent(&self) -> Vec<(u32, Signal)> {
            mutex_lock_or_recover(&self.signals_sent).clone()
        }
    }

    impl ProcessController for MockProcessController {
        fn check_process(&self, pid: u32) -> std::io::Result<ProcessStatus> {
            if let Some(err) = mutex_lock_or_recover(&self.check_error).take() {
                return Err(err);
            }
            Ok(self
                .process_states
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&pid)
                .copied()
                .unwrap_or(ProcessStatus::NotFound))
        }

        fn send_signal(&self, pid: u32, signal: Signal) -> std::io::Result<()> {
            if let Some(err) = mutex_lock_or_recover(&self.signal_error).take() {
                return Err(err);
            }
            mutex_lock_or_recover(&self.signals_sent).push((pid, signal));
            if self.signal_kills_process_on == Some(signal) {
                mutex_lock_or_recover(&self.process_states).remove(&pid);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockProcessController;
    use std::process::Command;
    use std::time::Duration;
    use std::time::Instant;

    #[test]
    fn test_signal_variants() {
        assert_ne!(Signal::Term, Signal::Kill);
    }

    #[test]
    fn test_process_status_variants() {
        assert_ne!(ProcessStatus::Running, ProcessStatus::NotFound);
        assert_ne!(ProcessStatus::Running, ProcessStatus::NoPermission);
        assert_ne!(ProcessStatus::NotFound, ProcessStatus::NoPermission);
    }

    #[test]
    fn test_mock_check_process_not_found() {
        let mock = MockProcessController::new();
        assert_eq!(
            mock.check_process(1234)
                .expect("check_process should succeed"),
            ProcessStatus::NotFound
        );
    }

    #[test]
    fn test_mock_check_process_running() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        assert_eq!(
            mock.check_process(1234)
                .expect("check_process should succeed"),
            ProcessStatus::Running
        );
    }

    #[test]
    fn test_mock_send_signal() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        mock.send_signal(1234, Signal::Term)
            .expect("send_signal should succeed");
        assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
    }

    #[test]
    fn test_mock_send_multiple_signals() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        mock.send_signal(1234, Signal::Term)
            .expect("first send_signal should succeed");
        mock.send_signal(1234, Signal::Kill)
            .expect("second send_signal should succeed");
        assert_eq!(
            mock.signals_sent(),
            vec![(1234, Signal::Term), (1234, Signal::Kill)]
        );
    }

    #[test]
    fn test_mock_check_error() {
        let mock =
            MockProcessController::new().with_check_error(std::io::Error::other("test error"));
        assert!(mock.check_process(1234).is_err());
    }

    #[test]
    fn test_mock_signal_error() {
        let mock = MockProcessController::new()
            .with_process(1234, ProcessStatus::Running)
            .with_signal_error(std::io::Error::other("test error"));
        assert!(mock.send_signal(1234, Signal::Term).is_err());
    }

    #[test]
    fn test_check_process_treats_zombie_as_not_found() {
        let mut child = Command::new("sleep")
            .arg("0.1")
            .spawn()
            .expect("spawn should succeed");
        let pid = child.id();

        std::thread::park_timeout(Duration::from_millis(300));

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if matches!(process_status_code(pid), Some('Z')) {
                break;
            }
            std::thread::park_timeout(Duration::from_millis(20));
        }

        assert_eq!(process_status_code(pid), Some('Z'));

        let controller = UnixProcessController;
        assert_eq!(
            controller
                .check_process(pid)
                .expect("check_process should succeed"),
            ProcessStatus::NotFound
        );

        let _ = child.wait();
    }
}
