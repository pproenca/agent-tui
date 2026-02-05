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
    use predicates::prelude::*;
    use std::time::Duration;

    #[test]
    fn e2e_daemon_version_and_sessions_report_running() {
        let harness = RealTestHarness::new();

        harness
            .run(&["--no-color", "version"])
            .success()
            .stdout(predicate::str::contains("CLI version:"))
            .stdout(predicate::str::contains("Daemon version:"))
            .stdout(predicate::str::contains("unavailable").not());

        harness
            .run(&["--no-color", "sessions"])
            .success()
            .stdout(predicate::str::contains("No active sessions"));
    }

    #[test]
    fn e2e_sessions_empty_on_fresh_daemon() {
        let harness = RealTestHarness::new();

        harness
            .run(&["--no-color", "sessions"])
            .success()
            .stdout(predicate::str::contains("No active sessions"));
    }

    #[test]
    fn e2e_daemon_stop_shuts_down() {
        let mut harness = RealTestHarness::new();

        harness
            .run(&["--no-color", "daemon", "stop"])
            .success()
            .stdout(
                predicate::str::contains("Daemon stopped")
                    .or(predicate::str::contains("already stopped")),
            );

        harness.wait_for_exit(Duration::from_secs(3));

        harness
            .run(&["--no-color", "sessions"])
            .code(69)
            .stderr(predicate::str::contains("Daemon not running"));

        harness.stop();
    }
}
