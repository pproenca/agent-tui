use crate::common::TestHarness;
use predicates::prelude::*;

#[test]
fn test_sessions_attach_without_active_session() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "attach"])
        .failure()
        .stderr(predicate::str::contains("No active session"));
}

#[test]
fn test_sessions_attach_with_id() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "attach", "test-session"])
        .failure()
        .stderr(predicate::str::contains("Terminal").or(predicate::str::contains("Device")));
}
