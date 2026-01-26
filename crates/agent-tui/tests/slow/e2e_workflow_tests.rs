#[path = "../common/mod.rs"]
mod common;

use common::RealTestHarness;
use predicates::prelude::*;
use std::time::Duration;

#[test]
fn e2e_daemon_status_reports_running() {
    let harness = RealTestHarness::new();

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
        .run(&["--no-color", "daemon", "status"])
        .code(3)
        .stdout(predicate::str::contains("Daemon is not running"));
}
