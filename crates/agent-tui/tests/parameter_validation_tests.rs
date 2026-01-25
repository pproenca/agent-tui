mod common;

use common::TestHarness;
use predicates::prelude::*;

#[test]
fn test_action_requires_ref_and_operation() {
    let harness = TestHarness::new();

    harness.run(&["action"]).failure();

    // Default action when only ref is provided should be click.
    harness
        .run(&["action", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Clicked"));
}

#[test]
fn test_action_click_accepts_element_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@btn1", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked"));
}

#[test]
fn test_action_fill_requires_value() {
    let harness = TestHarness::new();

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
}

#[test]
fn test_action_fill_with_empty_value() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@inp1", "fill", ""])
        .success()
        .stdout(predicate::str::contains("Filled"));
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
        .success()
        .stdout(predicate::str::contains("Selected").or(predicate::str::contains("success")));
}

#[test]
fn test_action_select_multiselect() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@sel1", "select", "Option 1", "Option 2"])
        .success()
        .stdout(predicate::str::contains("Selected").or(predicate::str::contains("success")));
}

#[test]
fn test_action_toggle_with_state() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@cb1", "toggle", "on"])
        .success()
        .stdout(
            predicate::str::contains("checked")
                .or(predicate::str::contains("Toggle"))
                .or(predicate::str::contains("success")),
        );
}

#[test]
fn test_input_requires_value_or_modifier() {
    let harness = TestHarness::new();

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
}

#[test]
fn test_input_with_text() {
    let harness = TestHarness::new();

    harness.run(&["input", "Hello World"]).success().stdout(
        predicate::str::contains("typed")
            .or(predicate::str::contains("input"))
            .or(predicate::str::contains("Text")),
    );
}

#[test]
fn test_input_hold_and_release() {
    let harness = TestHarness::new();

    harness.run(&["input", "Shift", "--hold"]).success();
    harness.run(&["input", "Shift", "--release"]).success();
}

#[test]
fn test_input_hold_release_conflict() {
    let harness = TestHarness::new();

    harness
        .run(&["input", "Shift", "--hold", "--release"])
        .failure();
}

#[test]
fn test_wait_with_no_args_waits_for_stable() {
    let harness = TestHarness::new();

    harness.run(&["wait", "--stable"]).success();
}

#[test]
fn test_wait_with_timeout_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-t", "5000", "--stable"]).success();
}

#[test]
fn test_wait_with_element_option() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-e", "@btn1"]).success();
}

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
}

#[test]
fn test_run_with_cwd_option() {
    let harness = TestHarness::new();

    harness
        .run(&["run", "-d", "/tmp", "bash"])
        .success()
        .stdout(predicate::str::contains("/tmp").or(predicate::str::contains("Session started")));
}

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

    harness
        .run(&["sessions", "--attach", "test-session"])
        .failure()
        .stderr(predicate::str::contains("Terminal").or(predicate::str::contains("Device")));
}

#[test]
fn test_sessions_cleanup_all_requires_cleanup() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "--all"])
        .failure()
        .stderr(predicate::str::contains("cleanup"));
}

#[test]
fn test_kill_without_session_uses_active() {
    let harness = TestHarness::new();

    harness
        .run(&["kill"])
        .success()
        .stdout(predicate::str::contains("killed").or(predicate::str::contains("Session")));
}

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

    harness
        .run(&["-s", "my-session", "screen"])
        .success()
        .stdout(predicate::str::contains("Screen").or(predicate::str::contains("my-session")));
}
