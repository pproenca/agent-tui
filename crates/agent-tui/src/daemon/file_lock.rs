use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::daemon::error::DaemonError;

pub struct LockFile {
    #[allow(dead_code)]
    file: File,
}

impl LockFile {
    pub fn acquire(lock_path: &Path) -> Result<Self, DaemonError> {
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| DaemonError::LockFailed(format!("failed to open lock file: {}", e)))?;

        let fd = lock_file.as_raw_fd();

        let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            return Err(DaemonError::AlreadyRunning);
        }

        lock_file
            .set_len(0)
            .map_err(|e| DaemonError::LockFailed(format!("failed to truncate lock file: {}", e)))?;

        let mut lock_file = lock_file;
        writeln!(lock_file, "{}", std::process::id())
            .map_err(|e| DaemonError::LockFailed(format!("failed to write PID: {}", e)))?;

        Ok(Self { file: lock_file })
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
