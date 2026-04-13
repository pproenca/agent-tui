use super::*;
use mock::MockProcessController;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

fn identity(pid: u32, started_at: u64) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        started_at: Some(started_at),
    }
}

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
    let mock = MockProcessController::new().with_check_error(std::io::Error::other("test error"));
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
fn test_check_expected_process_matches_identity() {
    let mock =
        MockProcessController::new().with_process_identity(1234, ProcessStatus::Running, Some(42));

    assert_eq!(
        check_expected_process(&mock, identity(1234, 42)).expect("identity check should succeed"),
        ProcessStatus::Running
    );
}

#[test]
fn test_check_expected_process_rejects_reused_pid() {
    let mock =
        MockProcessController::new().with_process_identity(1234, ProcessStatus::Running, Some(99));

    assert_eq!(
        check_expected_process(&mock, identity(1234, 42)).expect("identity check should succeed"),
        ProcessStatus::NotFound
    );
}

#[test]
fn test_check_expected_process_allows_legacy_pid_only_records() {
    let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);

    assert_eq!(
        check_expected_process(
            &mock,
            ProcessIdentity {
                pid: 1234,
                started_at: None,
            }
        )
        .expect("identity check should succeed"),
        ProcessStatus::Running
    );
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
