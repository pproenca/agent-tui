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
