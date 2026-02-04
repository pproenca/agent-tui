#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only assertions use unwrap/expect for clarity."
)]

//! End-to-end system tests.

mod common;

#[cfg(feature = "slow-tests")]
mod e2e {
    use crate::common::RealTestHarness;
    use once_cell::sync::Lazy;
    use predicates::prelude::*;
    use std::sync::{Mutex, MutexGuard};
    use std::time::Duration;

    static E2E_HARNESS: Lazy<Mutex<RealTestHarness>> =
        Lazy::new(|| Mutex::new(RealTestHarness::new()));

    fn shared_harness() -> MutexGuard<'static, RealTestHarness> {
        E2E_HARNESS.lock().expect("e2e harness lock poisoned")
    }

    #[test]
    fn e2e_daemon_status_reports_running() {
        let harness = shared_harness();

        harness
            .run(&["--no-color", "daemon", "status"])
            .success()
            .stdout(predicate::str::contains("Daemon status:"))
            .stdout(predicate::str::contains("Sessions:"))
            .stdout(predicate::str::contains("Daemon version:"))
            .stdout(predicate::str::contains("CLI version:"));
    }

    #[test]
    fn e2e_sessions_empty_on_fresh_daemon() {
        let harness = shared_harness();

        harness
            .run(&["--no-color", "sessions"])
            .success()
            .stdout(predicate::str::contains("No active sessions"));
    }

    #[test]
    fn e2e_daemon_stop_shuts_down() {
        let mut harness = shared_harness();

        harness
            .run(&["--no-color", "daemon", "stop"])
            .success()
            .stdout(
                predicate::str::contains("Daemon stopped")
                    .or(predicate::str::contains("already stopped")),
            );

        harness.wait_for_exit(Duration::from_secs(3));

        harness
            .run(&["--no-color", "daemon", "status"])
            .code(3)
            .stdout(predicate::str::contains("Daemon is not running"));

        *harness = RealTestHarness::new();
    }
}
