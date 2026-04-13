//! Daemon lock file helpers.

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::common::DaemonError;
use crate::infra::ipc::current_process_identity;

#[derive(serde::Serialize)]
struct LockFilePayload {
    pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_started_at: Option<u64>,
}

pub struct LockFile {
    _file: File,
}

impl LockFile {
    pub fn acquire(lock_path: &Path) -> Result<Self, DaemonError> {
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| DaemonError::LockFailed {
                operation: "open lock file",
                source: Box::new(e),
            })?;

        let fd = lock_file.as_raw_fd();

        // SAFETY: `flock` is safe to call with a valid file descriptor obtained from
        // `as_raw_fd()`. The file is kept open for the lifetime of `LockFile`, ensuring
        // the fd remains valid. LOCK_EX | LOCK_NB requests an exclusive, non-blocking lock.
        let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            match err.raw_os_error() {
                Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN => {
                    return Err(DaemonError::AlreadyRunning);
                }
                _ => {
                    return Err(DaemonError::LockFailed {
                        operation: "flock lock file",
                        source: Box::new(err),
                    });
                }
            }
        }

        lock_file.set_len(0).map_err(|e| DaemonError::LockFailed {
            operation: "truncate lock file",
            source: Box::new(e),
        })?;

        let mut lock_file = lock_file;
        let identity = current_process_identity();
        serde_json::to_writer_pretty(
            &mut lock_file,
            &LockFilePayload {
                pid: identity.pid,
                process_started_at: identity.started_at,
            },
        )
        .map_err(|e| DaemonError::LockFailed {
            operation: "write daemon identity",
            source: Box::new(e),
        })?;
        writeln!(lock_file).map_err(|e| DaemonError::LockFailed {
            operation: "flush daemon identity",
            source: Box::new(e),
        })?;

        Ok(Self { _file: lock_file })
    }
}

pub fn remove_lock_file(lock_path: &Path) {
    if lock_path.exists() {
        let _ = std::fs::remove_file(lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn temp_lock_path() -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("temp dir should be created");
        let path = dir.path().join("test.lock");
        (dir, path)
    }

    #[test]
    fn test_acquire_lock_succeeds() {
        let (_dir, path) = temp_lock_path();
        let lock = LockFile::acquire(&path);
        assert!(lock.is_ok());
    }

    #[test]
    fn test_acquire_lock_writes_pid() {
        let (_dir, path) = temp_lock_path();
        let _lock = LockFile::acquire(&path).expect("lock should be acquired");

        let contents = std::fs::read_to_string(&path).expect("lock file should be readable");
        let payload: serde_json::Value =
            serde_json::from_str(&contents).expect("identity payload should parse");
        assert_eq!(payload["pid"].as_u64(), Some(u64::from(std::process::id())));
        assert!(
            payload.get("process_started_at").is_some(),
            "lock file should carry process identity metadata"
        );
    }

    #[test]
    fn test_remove_lock_file() {
        let (_dir, path) = temp_lock_path();
        std::fs::write(&path, "test").expect("lock file should be written");
        assert!(path.exists());

        remove_lock_file(&path);
        assert!(!path.exists());
    }

    #[test]
    fn test_remove_nonexistent_lock_file_is_ok() {
        let (_dir, path) = temp_lock_path();
        assert!(!path.exists());
        remove_lock_file(&path);
    }
}
