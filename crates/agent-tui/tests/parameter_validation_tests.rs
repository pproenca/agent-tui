//! Parameter validation tests
//!
//! Tests for CLI parameter validation and error handling for invalid inputs.

mod common;

use common::TestHarness;
use predicates::prelude::*;

// =============================================================================
// Click Command Validation
// =============================================================================

#[test]
fn test_click_requires_ref_argument() {
    let harness = TestHarness::new();

    // Click without ref should fail at CLI level
    harness
        .run(&["click"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_click_accepts_element_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["click", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    let req = harness.last_request_for("click").unwrap();
    assert_eq!(
        req.params.as_ref().unwrap()["ref"].as_str().unwrap(),
        "@btn1"
    );
}

// =============================================================================
// Fill Command Validation
// =============================================================================

#[test]
fn test_fill_requires_ref_argument() {
    let harness = TestHarness::new();

    harness
        .run(&["fill"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_fill_requires_value_argument() {
    let harness = TestHarness::new();

    harness
        .run(&["fill", "@inp1"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_fill_accepts_both_arguments() {
    let harness = TestHarness::new();

    harness
        .run(&["fill", "@inp1", "test value"])
        .success()
        .stdout(predicate::str::contains("Filled"));

    let req = harness.last_request_for("fill").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["ref"].as_str().unwrap(), "@inp1");
    assert_eq!(params["value"].as_str().unwrap(), "test value");
}

#[test]
fn test_fill_with_empty_value() {
    let harness = TestHarness::new();

    // Empty string is valid
    harness.run(&["fill", "@inp1", ""]).success();

    let req = harness.last_request_for("fill").unwrap();
    assert_eq!(req.params.as_ref().unwrap()["value"].as_str().unwrap(), "");
}

// =============================================================================
// Scroll Command Validation
// =============================================================================

#[test]
fn test_scroll_requires_direction() {
    let harness = TestHarness::new();

    harness
        .run(&["scroll"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_scroll_valid_directions() {
    let harness = TestHarness::new();

    for dir in &["up", "down", "left", "right"] {
        harness.run(&["scroll", dir]).success();
    }
}

#[test]
fn test_scroll_with_amount() {
    let harness = TestHarness::new();

    harness.run(&["scroll", "down", "-a", "5"]).success();

    let req = harness.last_request_for("scroll").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["direction"].as_str().unwrap(), "down");
    assert_eq!(params["amount"].as_u64().unwrap(), 5);
}

// =============================================================================
// Keystroke/Press Command Validation
// =============================================================================

#[test]
fn test_press_requires_key() {
    let harness = TestHarness::new();

    harness
        .run(&["press"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_press_valid_keys() {
    let harness = TestHarness::new();

    let valid_keys = vec![
        "Enter",
        "Tab",
        "Escape",
        "Backspace",
        "Delete",
        "Up",
        "Down",
        "Left",
        "Right",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "F1",
        "F12",
    ];

    for key in valid_keys {
        harness.clear_requests();
        harness.run(&["press", key]).success();

        let req = harness.last_request_for("keystroke").unwrap();
        assert!(req.params.is_some());
    }
}

#[test]
fn test_press_with_modifiers() {
    let harness = TestHarness::new();

    harness.run(&["press", "Ctrl+c"]).success();

    let req = harness.last_request_for("keystroke").unwrap();
    let params = req.params.as_ref().unwrap();
    assert!(params["key"].as_str().unwrap().contains("Ctrl"));
}

// =============================================================================
// Select Command Validation
// =============================================================================

#[test]
fn test_select_requires_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["select"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_select_requires_option() {
    let harness = TestHarness::new();

    harness
        .run(&["select", "@sel1"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_select_with_valid_args() {
    let harness = TestHarness::new();

    harness.run(&["select", "@sel1", "option1"]).success();

    let req = harness.last_request_for("select").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["ref"].as_str().unwrap(), "@sel1");
    assert_eq!(params["option"].as_str().unwrap(), "option1");
}

// =============================================================================
// Multiselect Command Validation
// =============================================================================

#[test]
fn test_multiselect_requires_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["multiselect"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_multiselect_requires_options() {
    let harness = TestHarness::new();

    harness
        .run(&["multiselect", "@sel1"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_multiselect_with_multiple_options() {
    let harness = TestHarness::new();

    harness
        .run(&["multiselect", "@sel1", "opt1", "opt2", "opt3"])
        .success();

    let req = harness.last_request_for("multiselect").unwrap();
    let params = req.params.as_ref().unwrap();
    let options = params["options"].as_array().unwrap();
    assert_eq!(options.len(), 3);
}

// =============================================================================
// Resize Command Validation
// =============================================================================

#[test]
fn test_resize_with_cols_and_rows() {
    let harness = TestHarness::new();

    harness
        .run(&["resize", "--cols", "100", "--rows", "30"])
        .success();

    let req = harness.last_request_for("resize").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["cols"].as_u64().unwrap(), 100);
    assert_eq!(params["rows"].as_u64().unwrap(), 30);
}

#[test]
fn test_resize_default_values() {
    let harness = TestHarness::new();

    // Resize with default values
    harness.run(&["resize"]).success();

    harness.assert_method_called("resize");
}

// =============================================================================
// Wait Command Validation
// =============================================================================

#[test]
fn test_wait_with_no_args_waits_for_stable() {
    let harness = TestHarness::new();

    // wait without args waits for screen stability
    harness.run(&["wait"]).success();
    harness.assert_method_called("wait");
}

#[test]
fn test_wait_with_timeout_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-t", "1000", "SomeText"]).success();

    let req = harness.last_request_for("wait").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["timeout_ms"].as_u64().unwrap(), 1000);
}

#[test]
fn test_wait_with_element_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "--element", "@btn1"]).success();

    // Verify wait was called
    harness.assert_method_called("wait");
}

// =============================================================================
// Spawn Command Validation
// =============================================================================

#[test]
fn test_spawn_requires_command() {
    let harness = TestHarness::new();

    harness
        .run(&["spawn"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_spawn_with_size_options() {
    let harness = TestHarness::new();

    harness
        .run(&["spawn", "--cols", "100", "--rows", "30", "bash"])
        .success();

    let req = harness.last_request_for("spawn").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["cols"].as_u64().unwrap(), 100);
    assert_eq!(params["rows"].as_u64().unwrap(), 30);
}

#[test]
fn test_spawn_with_cwd_option() {
    let harness = TestHarness::new();

    harness.run(&["spawn", "--cwd", "/tmp", "bash"]).success();

    let req = harness.last_request_for("spawn").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["cwd"].as_str().unwrap(), "/tmp");
}

// =============================================================================
// Attach Command Validation
// =============================================================================

#[test]
fn test_attach_requires_session_id() {
    let harness = TestHarness::new();

    harness
        .run(&["attach"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_attach_with_session_id() {
    let harness = TestHarness::new();

    harness.run(&["attach", "session-123"]).success();

    let req = harness.last_request_for("attach").unwrap();
    let params = req.params.as_ref().unwrap();
    // The positional arg is session_id
    assert!(
        params.get("session_id").is_some() || params.get("session").is_some(),
        "params should contain session_id or session: {:?}",
        params
    );
}

// =============================================================================
// Kill Command Validation
// =============================================================================

#[test]
fn test_kill_without_session_id_uses_active() {
    let harness = TestHarness::new();

    // Kill without session_id should work (kills active session)
    harness.run(&["kill"]).success();
    harness.assert_method_called("kill");
}

#[test]
fn test_kill_with_session_option() {
    let harness = TestHarness::new();

    harness.run(&["kill", "-s", "session-to-kill"]).success();

    // Verify the kill method was called with session param
    let req = harness.last_request_for("kill").unwrap();
    // The session is passed in params - check the structure
    if let Some(params) = req.params.as_ref() {
        // Session might be nested or at top level
        assert!(
            params.get("session_id").is_some() || params.get("session").is_some(),
            "params should contain session info: {:?}",
            params
        );
    }
}

// =============================================================================
// Find Command Validation
// =============================================================================

#[test]
fn test_find_with_role_filter() {
    let harness = TestHarness::new();

    harness.run(&["find", "--role", "button"]).success();

    let req = harness.last_request_for("find").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["role"].as_str().unwrap(), "button");
}

#[test]
fn test_find_with_name_filter() {
    let harness = TestHarness::new();

    harness.run(&["find", "--name", "Submit"]).success();

    let req = harness.last_request_for("find").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["name"].as_str().unwrap(), "Submit");
}

// =============================================================================
// Type Command Validation
// =============================================================================

#[test]
fn test_type_requires_text() {
    let harness = TestHarness::new();

    harness
        .run(&["type"])
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_type_with_text() {
    let harness = TestHarness::new();

    harness.run(&["type", "Hello World"]).success();

    let req = harness.last_request_for("type").unwrap();
    let params = req.params.as_ref().unwrap();
    assert_eq!(params["text"].as_str().unwrap(), "Hello World");
}

// =============================================================================
// Global Options Validation
// =============================================================================

#[test]
fn test_format_json_option() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "health"])
        .success()
        .stdout(predicate::str::contains("{"));
}

#[test]
fn test_format_text_option() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "text", "health"])
        .success()
        .stdout(predicate::str::contains("Daemon"));
}

#[test]
fn test_session_option_with_command() {
    let harness = TestHarness::new();

    harness.run(&["-s", "my-session", "snapshot"]).success();

    let req = harness.last_request_for("snapshot").unwrap();
    let params = req.params.as_ref().unwrap();
    // Check session_id is passed
    assert!(
        params.get("session_id").is_some() || params.get("session").is_some(),
        "params should contain session_id: {:?}",
        params
    );
}
