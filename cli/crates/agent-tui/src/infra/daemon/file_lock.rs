use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::common::DaemonError;

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
        writeln!(lock_file, "{}", std::process::id()).map_err(|e| DaemonError::LockFailed {
            operation: "write PID",
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
        let dir = TempDir::new().unwrap();
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
        let _lock = LockFile::acquire(&path).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(pid, std::process::id());
    }

    #[test]
    fn test_remove_lock_file() {
        let (_dir, path) = temp_lock_path();
        std::fs::write(&path, "test").unwrap();
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
