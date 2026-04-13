//! Process control utilities.

use std::process::Command;

use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub started_at: Option<u64>,
}

pub fn current_process_identity() -> ProcessIdentity {
    let pid = std::process::id();
    let controller = UnixProcessController;
    match controller.process_identity(pid) {
        Ok(Some(identity)) => identity,
        Ok(None) | Err(_) => ProcessIdentity {
            pid,
            started_at: None,
        },
    }
}

pub fn check_expected_process<C: ProcessController>(
    controller: &C,
    expected: ProcessIdentity,
) -> std::io::Result<ProcessStatus> {
    let status = controller.check_process(expected.pid)?;
    if !matches!(status, ProcessStatus::Running | ProcessStatus::NoPermission) {
        return Ok(status);
    }

    let Some(expected_started_at) = expected.started_at else {
        return Ok(status);
    };

    let actual = controller.process_identity(expected.pid)?;
    match actual {
        Some(actual) if actual.started_at == Some(expected_started_at) => Ok(status),
        Some(_) | None => Ok(ProcessStatus::NotFound),
    }
}

pub trait ProcessController: Send + Sync {
    fn check_process(&self, pid: u32) -> std::io::Result<ProcessStatus>;

    fn send_signal(&self, pid: u32, signal: Signal) -> std::io::Result<()>;

    fn process_identity(&self, pid: u32) -> std::io::Result<Option<ProcessIdentity>>;
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

    fn process_identity(&self, pid: u32) -> std::io::Result<Option<ProcessIdentity>> {
        match self.check_process(pid)? {
            ProcessStatus::NotFound => Ok(None),
            ProcessStatus::Running | ProcessStatus::NoPermission => Ok(Some(ProcessIdentity {
                pid,
                started_at: process_started_at_secs(pid),
            })),
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

fn process_started_at_secs(pid: u32) -> Option<u64> {
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    let process = system.process(pid)?;
    let started_at = process.start_time();
    (started_at > 0).then(|| System::boot_time().saturating_add(started_at))
}

#[allow(clippy::expect_used)]
pub mod mock {
    use super::*;
    use crate::common::mutex_lock_or_recover;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Clone, Copy)]
    struct MockProcessRecord {
        status: ProcessStatus,
        started_at: Option<u64>,
    }

    pub struct MockProcessController {
        process_states: Mutex<HashMap<u32, MockProcessRecord>>,
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
            mutex_lock_or_recover(&self.process_states).insert(
                pid,
                MockProcessRecord {
                    status,
                    started_at: None,
                },
            );
            self
        }

        pub fn with_process_identity(
            self,
            pid: u32,
            status: ProcessStatus,
            started_at: Option<u64>,
        ) -> Self {
            mutex_lock_or_recover(&self.process_states)
                .insert(pid, MockProcessRecord { status, started_at });
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
                .map(|record| record.status)
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

        fn process_identity(&self, pid: u32) -> std::io::Result<Option<ProcessIdentity>> {
            let record = self
                .process_states
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&pid)
                .copied();
            Ok(record.map(|record| ProcessIdentity {
                pid,
                started_at: record.started_at,
            }))
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
