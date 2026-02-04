#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only assertions use unwrap/expect for clarity."
)]

//! CLI contract tests.

mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    method: String,
    response: serde_json::Value,
    expected_stdout: Vec<String>,
    expected_exit: i32,
}

fn run_fixture(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("contracts")
        .join(file);
    let data =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let fixture: Fixture =
        serde_json::from_str(&data).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
    assert!(
        !fixture.name.is_empty(),
        "fixture name missing for {}",
        file
    );

    let harness = TestHarness::new();
    harness.set_response(
        &fixture.method,
        MockResponse::Success(fixture.response["result"].clone()),
    );

    let assert = harness.run(&["daemon", "status"]);

    if fixture.expected_exit == 0 {
        let mut ok = assert.success();
        for needle in &fixture.expected_stdout {
            ok = ok.stdout(predicate::str::contains(needle));
        }
    } else {
        let mut fail = assert.code(fixture.expected_exit);
        for needle in &fixture.expected_stdout {
            fail = fail.stdout(predicate::str::contains(needle));
        }
    }
}

#[test]
fn contracts_health_fixture() {
    run_fixture("health.json");
}
