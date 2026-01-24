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
    fn check_process(&self, pid: u32) -> Result<ProcessStatus, std::io::Error>;

    fn send_signal(&self, pid: u32, signal: Signal) -> Result<(), std::io::Error>;
}

pub struct UnixProcessController;

impl ProcessController for UnixProcessController {
    fn check_process(&self, pid: u32) -> Result<ProcessStatus, std::io::Error> {
        let pid_t: libc::pid_t = pid.try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID out of range")
        })?;

        let result = unsafe { libc::kill(pid_t, 0) };
        if result == 0 {
            return Ok(ProcessStatus::Running);
        }

        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ESRCH) => Ok(ProcessStatus::NotFound),
            Some(libc::EPERM) => Ok(ProcessStatus::NoPermission),
            _ => Err(err),
        }
    }

    fn send_signal(&self, pid: u32, signal: Signal) -> Result<(), std::io::Error> {
        let pid_t: libc::pid_t = pid.try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID out of range")
        })?;

        let sig = match signal {
            Signal::Term => libc::SIGTERM,
            Signal::Kill => libc::SIGKILL,
        };

        let result = unsafe { libc::kill(pid_t, sig) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct MockProcessController {
        process_states: Mutex<HashMap<u32, ProcessStatus>>,
        signals_sent: Mutex<Vec<(u32, Signal)>>,
        check_error: Mutex<Option<std::io::Error>>,
        signal_error: Mutex<Option<std::io::Error>>,
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
            }
        }

        pub fn with_process(self, pid: u32, status: ProcessStatus) -> Self {
            self.process_states.lock().unwrap().insert(pid, status);
            self
        }

        pub fn with_check_error(self, error: std::io::Error) -> Self {
            *self.check_error.lock().unwrap() = Some(error);
            self
        }

        pub fn with_signal_error(self, error: std::io::Error) -> Self {
            *self.signal_error.lock().unwrap() = Some(error);
            self
        }

        pub fn signals_sent(&self) -> Vec<(u32, Signal)> {
            self.signals_sent.lock().unwrap().clone()
        }
    }

    impl ProcessController for MockProcessController {
        fn check_process(&self, pid: u32) -> Result<ProcessStatus, std::io::Error> {
            if let Some(err) = self.check_error.lock().unwrap().take() {
                return Err(err);
            }
            Ok(self
                .process_states
                .lock()
                .unwrap()
                .get(&pid)
                .copied()
                .unwrap_or(ProcessStatus::NotFound))
        }

        fn send_signal(&self, pid: u32, signal: Signal) -> Result<(), std::io::Error> {
            if let Some(err) = self.signal_error.lock().unwrap().take() {
                return Err(err);
            }
            self.signals_sent.lock().unwrap().push((pid, signal));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockProcessController;

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
        assert_eq!(mock.check_process(1234).unwrap(), ProcessStatus::NotFound);
    }

    #[test]
    fn test_mock_check_process_running() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        assert_eq!(mock.check_process(1234).unwrap(), ProcessStatus::Running);
    }

    #[test]
    fn test_mock_send_signal() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        mock.send_signal(1234, Signal::Term).unwrap();
        assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
    }

    #[test]
    fn test_mock_send_multiple_signals() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        mock.send_signal(1234, Signal::Term).unwrap();
        mock.send_signal(1234, Signal::Kill).unwrap();
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
}
