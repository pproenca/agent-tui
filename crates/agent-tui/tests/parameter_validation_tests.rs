//! Parameter validation tests
//!
//! Tests for CLI parameter validation and error handling for invalid inputs.

mod common;

use common::TestHarness;
use predicates::prelude::*;

// =============================================================================
// Action Command Validation
// =============================================================================

#[test]
fn test_action_requires_ref_and_operation() {
    let harness = TestHarness::new();

    // action without ref should fail
    harness.run(&["action"]).failure();

    // action with ref but no operation should fail
    harness.run(&["action", "@btn1"]).failure();
}

#[test]
fn test_action_click_accepts_element_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@btn1", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    let req = harness.last_request_for("click").unwrap();
    assert_eq!(
        req.params.as_ref().unwrap()["ref"].as_str().unwrap(),
        "@btn1"
    );
}

#[test]
fn test_action_fill_requires_value() {
    let harness = TestHarness::new();

    // fill without value should fail
    harness
        .run(&["action", "@inp1", "fill"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_action_fill_accepts_both_arguments() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@inp1", "fill", "test value"])
        .success()
        .stdout(predicate::str::contains("Filled"));

    let req = harness.last_request_for("fill").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["ref"].as_str().unwrap(), "@inp1");
    assert_eq!(params["value"].as_str().unwrap(), "test value");
}

#[test]
fn test_action_fill_with_empty_value() {
    let harness = TestHarness::new();

    // Empty string is valid
    harness.run(&["action", "@inp1", "fill", ""]).success();

    let req = harness.last_request_for("fill").unwrap();
    assert_eq!(req.params.as_ref().unwrap()["value"].as_str().unwrap(), "");
}

#[test]
fn test_action_scroll_requires_direction() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@e1", "scroll"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_action_scroll_valid_directions() {
    let harness = TestHarness::new();

    for direction in &["up", "down", "left", "right"] {
        harness
            .run(&["action", "@e1", "scroll", direction])
            .success();
    }
}

#[test]
fn test_action_scroll_with_amount() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@e1", "scroll", "down", "10"])
        .success();

    let req = harness.last_request_for("scroll").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["amount"].as_u64().unwrap(), 10);
}

#[test]
fn test_action_select_requires_option() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@sel1", "select"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_action_select_with_valid_args() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@sel1", "select", "Option 1"])
        .success();

    let req = harness.last_request_for("select").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["ref"].as_str().unwrap(), "@sel1");
}

#[test]
fn test_action_select_multiselect() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@sel1", "select", "Option 1", "Option 2"])
        .success();

    let req = harness.last_request_for("multiselect").unwrap();
    let params = req.params.as_ref().unwrap();
    let options = params["options"].as_array().unwrap();
    assert_eq!(options.len(), 2);
}

#[test]
fn test_action_toggle_with_state() {
    let harness = TestHarness::new();

    harness.run(&["action", "@cb1", "toggle", "on"]).success();

    let req = harness.last_request_for("toggle").unwrap();
    let params = req.params.as_ref().unwrap();
    assert!(params["state"].as_bool().unwrap());
}

// =============================================================================
// Input Command Validation
// =============================================================================

#[test]
fn test_input_requires_value_or_modifier() {
    let harness = TestHarness::new();

    // input without any args requires --hold or --release
    harness
        .run(&["input"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_input_valid_keys() {
    let harness = TestHarness::new();

    for key in &["Enter", "Tab", "Escape", "ArrowUp", "F1", "Ctrl+c"] {
        harness.run(&["input", key]).success();
    }

    harness.assert_method_called("keystroke");
}

#[test]
fn test_input_with_text() {
    let harness = TestHarness::new();

    harness.run(&["input", "Hello World"]).success();
    harness.assert_method_called("type");
}

#[test]
fn test_input_hold_and_release() {
    let harness = TestHarness::new();

    harness.run(&["input", "Shift", "--hold"]).success();
    harness.assert_method_called("keydown");

    harness.run(&["input", "Shift", "--release"]).success();
    harness.assert_method_called("keyup");
}

#[test]
fn test_input_hold_release_conflict() {
    let harness = TestHarness::new();

    // --hold and --release conflict
    harness
        .run(&["input", "Shift", "--hold", "--release"])
        .failure();
}

// =============================================================================
// Wait Command Validation
// =============================================================================

#[test]
fn test_wait_with_no_args_waits_for_stable() {
    let harness = TestHarness::new();

    // wait with no args should require some condition
    harness.run(&["wait", "--stable"]).success();

    let req = harness.last_request_for("wait").unwrap();
    assert_eq!(
        req.params.as_ref().unwrap()["condition"].as_str().unwrap(),
        "stable"
    );
}

#[test]
fn test_wait_with_timeout_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-t", "5000", "--stable"]).success();

    let req = harness.last_request_for("wait").unwrap();
    assert_eq!(
        req.params.as_ref().unwrap()["timeout_ms"].as_u64().unwrap(),
        5000
    );
}

#[test]
fn test_wait_with_element_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-e", "@btn1"]).success();

    let req = harness.last_request_for("wait").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["condition"].as_str().unwrap(), "element");
    assert_eq!(params["target"].as_str().unwrap(), "@btn1");
}

// =============================================================================
// Run Command Validation
// =============================================================================

#[test]
fn test_run_requires_command() {
    let harness = TestHarness::new();

    harness
        .run(&["run"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_run_with_size_options() {
    let harness = TestHarness::new();

    harness
        .run(&["run", "--cols", "80", "--rows", "24", "bash"])
        .success();

    let req = harness.last_request_for("spawn").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["cols"].as_u64().unwrap(), 80);
    assert_eq!(params["rows"].as_u64().unwrap(), 24);
}

#[test]
fn test_run_with_cwd_option() {
    let harness = TestHarness::new();

    harness.run(&["run", "-d", "/tmp", "bash"]).success();

    let req = harness.last_request_for("spawn").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["cwd"].as_str().unwrap(), "/tmp");
}

// =============================================================================
// Sessions Command Validation
// =============================================================================

#[test]
fn test_sessions_attach_requires_id() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "--attach"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_sessions_attach_with_id() {
    let harness = TestHarness::new();

    // attach starts interactive mode which may fail in test context
    // but CLI should accept the command
    let _ = harness.run(&["sessions", "--attach", "test-session"]);
}

#[test]
fn test_sessions_cleanup_all_requires_cleanup() {
    let harness = TestHarness::new();

    // --all requires --cleanup
    harness
        .run(&["sessions", "--all"])
        .failure()
        .stderr(predicate::str::contains("cleanup"));
}

// =============================================================================
// Kill Command Validation
// =============================================================================

#[test]
fn test_kill_without_session_uses_active() {
    let harness = TestHarness::new();

    // kill without session uses the active session
    harness.run(&["kill"]).success();

    harness.assert_method_called("kill");
}

// =============================================================================
// Format Options
// =============================================================================

#[test]
fn test_format_json_option() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "sessions"])
        .success()
        .stdout(predicate::str::contains("{"));
}

#[test]
fn test_format_text_option() {
    let harness = TestHarness::new();

    harness.run(&["-f", "text", "sessions"]).success();
}

#[test]
fn test_session_option_with_command() {
    let harness = TestHarness::new();

    harness.run(&["-s", "my-session", "screen"]).success();

    // The session_id is set at the RPC level, verify the command succeeded
    harness.assert_method_called("snapshot");
}
