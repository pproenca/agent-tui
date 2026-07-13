use super::*;
use std::io;

struct FailingShutdownNotifier;

impl crate::usecases::ports::ShutdownNotifier for FailingShutdownNotifier {
    fn notify(&self) -> Result<(), io::Error> {
        Err(io::Error::other("wakeup pipe closed"))
    }
}

#[test]
fn test_shutdown_usecase_sets_flag_to_true() {
    let shutdown_flag = AtomicBool::new(false);
    let notifier = crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;

    assert!(!shutdown_flag.load(Ordering::SeqCst));

    shutdown(&shutdown_flag, &notifier, ShutdownInput)
        .expect("shutdown notification should succeed");

    assert!(shutdown_flag.load(Ordering::SeqCst));
}

#[test]
fn test_shutdown_usecase_propagates_notify_failure() {
    let shutdown_flag = AtomicBool::new(false);

    let error = shutdown(&shutdown_flag, &FailingShutdownNotifier, ShutdownInput)
        .expect_err("shutdown notification should fail");

    assert!(shutdown_flag.load(Ordering::SeqCst));
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "wakeup pipe closed");
}

#[test]
fn test_shutdown_usecase_is_idempotent() {
    let shutdown_flag = AtomicBool::new(false);
    let notifier = crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;

    shutdown(&shutdown_flag, &notifier, ShutdownInput)
        .expect("first shutdown notification should succeed");
    shutdown(&shutdown_flag, &notifier, ShutdownInput)
        .expect("second shutdown notification should succeed");

    assert!(shutdown_flag.load(Ordering::SeqCst));
}
