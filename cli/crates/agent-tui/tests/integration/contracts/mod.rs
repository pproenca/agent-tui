//! Contract tests: load JSON fixtures that specify an RPC method, a canned response, and CLI expectations.

mod runner;

#[test]
fn contracts_health_fixture() {
    runner::run_fixture("health.json");
}
// Declared as an integration test module; loaded by tests/integration.rs.
