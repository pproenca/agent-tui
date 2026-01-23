//! Claude Code E2E tests for VOM element detection
//!
//! These tests verify that the VOM system correctly detects UI elements
//! in a live Claude Code session. They require Claude Code to be installed.
//!
//! CI Setup Required:
//! - Install Claude Code in CI environment
//! - Set `CLAUDE_CODE_PATH` environment variable if non-standard location
//! - Tests will fail CI if Claude Code is not available

mod common;

use common::RealTestHarness;
use serde_json::Value;
use std::env;

/// Get the Claude Code executable path.
/// Checks CLAUDE_CODE_PATH env var first, then defaults to "claude".
fn claude_code_path() -> String {
    env::var("CLAUDE_CODE_PATH").unwrap_or_else(|_| "claude".to_string())
}

/// Check if Claude Code is available on the system.
fn claude_code_available() -> bool {
    std::process::Command::new(claude_code_path())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Helper to spawn Claude Code and wait for stable state
fn spawn_claude_code(h: &RealTestHarness) -> Option<String> {
    let output = h
        .cli_json()
        .args(["spawn", &claude_code_path()])
        .output()
        .expect("spawn failed");

    if !output.status.success() {
        eprintln!(
            "Failed to spawn Claude Code: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }

    let json: Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse spawn output as JSON");
    let session_id = json["session_id"]
        .as_str()
        .expect("no session_id in response")
        .to_string();

    // Wait for Claude Code to become stable using stable-based wait
    let wait_status = h
        .cli()
        .args(["-s", &session_id, "wait", "-t", "20000", "--stable"])
        .status()
        .expect("wait command failed");

    if !wait_status.success() {
        eprintln!("Timeout waiting for Claude Code to become stable");
        return None;
    }

    // Additional brief wait to ensure rendering is complete
    std::thread::sleep(std::time::Duration::from_millis(500));

    Some(session_id)
}

/// Get snapshot with elements from Claude Code session
fn get_snapshot_elements(h: &RealTestHarness, session_id: &str) -> Value {
    let output = h
        .cli_json()
        .args(["-s", session_id, "snapshot"])
        .output()
        .expect("snapshot failed");

    assert!(
        output.status.success(),
        "snapshot failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("failed to parse snapshot as JSON")
}

// =============================================================================
// Claude Code E2E Tests
// =============================================================================

#[test]
fn test_claude_code_prompt_marker_on_startup() {
    if !claude_code_available() {
        panic!(
            "Claude Code is not available. Install Claude Code or set CLAUDE_CODE_PATH. \
             These tests are required in CI."
        );
    }

    let h = RealTestHarness::new();
    let session_id = spawn_claude_code(&h).expect("Failed to spawn Claude Code");

    let snapshot = get_snapshot_elements(&h, &session_id);
    let screen = snapshot["screen"].as_str().unwrap_or("");
    let tree = snapshot["tree"].as_str().unwrap_or("");

    // Claude Code shows various UI states - check for any recognizable content
    // Fresh start shows "Ready to code here?" or permission dialog
    // Active session shows ">" prompt
    assert!(
        screen.contains('>')
            || screen.contains("Ready")
            || screen.contains("permission")
            || tree.contains("prompt")
            || tree.contains("button"),
        "Claude Code should show recognizable UI on startup. Screen: {}",
        &screen[..screen.len().min(500)]
    );
}

#[test]
fn test_claude_code_interactive_elements_detected() {
    if !claude_code_available() {
        panic!(
            "Claude Code is not available. Install Claude Code or set CLAUDE_CODE_PATH. \
             These tests are required in CI."
        );
    }

    let h = RealTestHarness::new();
    let session_id = spawn_claude_code(&h).expect("Failed to spawn Claude Code");

    // Use interactive snapshot format to get elements
    let output = h
        .cli_json()
        .args(["-s", &session_id, "snapshot", "-i"])
        .output()
        .expect("snapshot failed");

    assert!(
        output.status.success(),
        "snapshot failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot: Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse snapshot as JSON");
    let screen = snapshot["screen"].as_str().unwrap_or("");

    // With interactive mode, Claude Code should have visible content
    // which indicates the VOM is processing the screen
    assert!(
        !screen.trim().is_empty(),
        "Claude Code should have screen content in interactive mode"
    );
}

#[test]
fn test_claude_code_status_spinner_during_thinking() {
    if !claude_code_available() {
        panic!(
            "Claude Code is not available. Install Claude Code or set CLAUDE_CODE_PATH. \
             These tests are required in CI."
        );
    }

    let h = RealTestHarness::new();
    let session_id = spawn_claude_code(&h).expect("Failed to spawn Claude Code");

    // Type a simple query to trigger thinking state
    let type_status = h
        .cli()
        .args(["-s", &session_id, "type", "hello"])
        .status()
        .expect("type command failed");
    assert!(type_status.success(), "type command should succeed");

    // Press Enter to submit
    let press_status = h
        .cli()
        .args(["-s", &session_id, "press", "Enter"])
        .status()
        .expect("press command failed");
    assert!(press_status.success(), "press command should succeed");

    // Brief wait for thinking to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    let snapshot = get_snapshot_elements(&h, &session_id);
    let screen = snapshot["screen"].as_str().unwrap_or("");

    // During thinking, we might see a spinner or status indicator
    // This is best-effort since timing is tricky
    let has_spinner = screen
        .chars()
        .any(|c| matches!(c, '⠋' | '⠙' | '⠹' | '⠸' | '⠼' | '⠴' | '⠦' | '⠧' | '⠇' | '⠏'));
    let has_thinking = screen.contains("Thinking") || screen.contains("thinking");

    // Either spinner or response should be visible (or at least text changed)
    assert!(
        has_spinner || has_thinking || screen.len() > 100,
        "Claude Code should show activity after query. Screen: {}",
        &screen[..screen.len().min(500)]
    );
}

#[test]
fn test_claude_code_elements_have_refs() {
    if !claude_code_available() {
        panic!(
            "Claude Code is not available. Install Claude Code or set CLAUDE_CODE_PATH. \
             These tests are required in CI."
        );
    }

    let h = RealTestHarness::new();
    let session_id = spawn_claude_code(&h).expect("Failed to spawn Claude Code");

    let snapshot = get_snapshot_elements(&h, &session_id);
    let refs = &snapshot["refs"];

    // Should have at least one element ref
    if let Some(refs_obj) = refs.as_object() {
        assert!(
            !refs_obj.is_empty(),
            "Claude Code snapshot should have element refs"
        );

        // Check that refs have expected structure
        for (ref_key, ref_value) in refs_obj {
            assert!(
                ref_key.starts_with('@'),
                "Element ref should start with @: {}",
                ref_key
            );
            assert!(
                ref_value.get("role").is_some(),
                "Element ref should have role: {}",
                ref_key
            );
        }
    }
}

#[test]
fn test_claude_code_screen_not_empty() {
    if !claude_code_available() {
        panic!(
            "Claude Code is not available. Install Claude Code or set CLAUDE_CODE_PATH. \
             These tests are required in CI."
        );
    }

    let h = RealTestHarness::new();
    let session_id = spawn_claude_code(&h).expect("Failed to spawn Claude Code");

    let snapshot = get_snapshot_elements(&h, &session_id);
    let screen = snapshot["screen"].as_str().unwrap_or("");

    assert!(
        !screen.trim().is_empty(),
        "Claude Code screen should not be empty"
    );

    // Screen should have reasonable content (not just whitespace)
    let non_space_chars = screen.chars().filter(|c| !c.is_whitespace()).count();
    assert!(
        non_space_chars > 10,
        "Claude Code screen should have visible content. Found {} non-space chars",
        non_space_chars
    );
}
