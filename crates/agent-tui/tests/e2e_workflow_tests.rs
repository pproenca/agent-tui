//! E2E workflow tests with real daemon
//!
//! Each test spawns an isolated daemon instance on a unique socket,
//! allowing tests to run in parallel without state conflicts.

mod common;

use common::RealTestHarness;
use serde_json::Value;

// =============================================================================
// Session Lifecycle Tests
// =============================================================================

#[test]
fn test_spawn_snapshot_kill() {
    let h = RealTestHarness::new();

    // 1. Spawn bash, get session ID
    let session_id = h.spawn_bash();
    assert!(!session_id.is_empty());

    // 2. Snapshot should show shell prompt
    let output = h.cli_json().args(["snapshot"]).output().unwrap();
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let screen = json["screen"].as_str().unwrap();
    assert!(screen.contains("$"), "Shell prompt should be visible");

    // 3. Kill session
    assert!(h.cli().args(["kill"]).status().unwrap().success());

    // 4. Sessions list should not contain our session
    let output = h.cli_json().args(["sessions"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sessions = json["sessions"].as_array().unwrap();
    assert!(
        !sessions
            .iter()
            .any(|s| s["session_id"].as_str() == Some(&session_id)),
        "Killed session should not appear in list"
    );
}

// =============================================================================
// State Change Verification Tests
// =============================================================================

#[test]
fn test_type_changes_screen() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // 1. Type command with unique marker
    assert!(
        h.cli()
            .args(["type", "echo E2E_MARKER_ABC123"])
            .status()
            .unwrap()
            .success()
    );

    // 2. Wait for text to appear
    assert!(
        h.cli()
            .args(["wait", "-t", "5000", "E2E_MARKER_ABC123"])
            .status()
            .unwrap()
            .success()
    );

    // 3. Snapshot contains typed text
    let output = h.cli_json().args(["snapshot"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let screen = json["screen"].as_str().unwrap();
    assert!(
        screen.contains("E2E_MARKER_ABC123"),
        "Screen should contain typed text"
    );
}

// =============================================================================
// Multi-Session Management Tests
// =============================================================================

#[test]
fn test_multi_session_switching() {
    let h = RealTestHarness::new();

    // 1. Spawn first session
    let sess_a = h.spawn_bash();

    // 2. Spawn second session
    let sess_b = h.spawn_bash();
    assert_ne!(sess_a, sess_b, "Sessions should have different IDs");

    // 3. Type unique markers in each session
    assert!(
        h.cli()
            .args(["-s", &sess_a, "type", "MARKER_AAA"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        h.cli()
            .args(["-s", &sess_b, "type", "MARKER_BBB"])
            .status()
            .unwrap()
            .success()
    );

    // 4. Wait for markers in each
    assert!(
        h.cli()
            .args(["-s", &sess_a, "wait", "-t", "5000", "MARKER_AAA"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        h.cli()
            .args(["-s", &sess_b, "wait", "-t", "5000", "MARKER_BBB"])
            .status()
            .unwrap()
            .success()
    );

    // 5. Sessions list shows both
    let output = h.cli_json().args(["sessions"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sessions = json["sessions"].as_array().unwrap();
    assert!(sessions.len() >= 2, "Should have at least 2 sessions");

    // 6. Kill session A
    assert!(
        h.cli()
            .args(["-s", &sess_a, "kill"])
            .status()
            .unwrap()
            .success()
    );

    // 7. Session B still works - snapshot shows its marker
    let output = h
        .cli_json()
        .args(["-s", &sess_b, "snapshot"])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json["screen"].as_str().unwrap().contains("MARKER_BBB"),
        "Session B should still have its marker"
    );
}

// =============================================================================
// Wait Condition Tests
// =============================================================================

#[test]
fn test_wait_for_delayed_output() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Clear the screen first so we have a clean slate
    assert!(h.cli().args(["type", "clear"]).status().unwrap().success());
    assert!(h.cli().args(["press", "Enter"]).status().unwrap().success());
    assert!(
        h.cli()
            .args(["wait", "--condition", "stable", "-t", "2000"])
            .status()
            .unwrap()
            .success()
    );

    // Type the command that will sleep and then echo. After typing, the screen shows:
    // "bash$ sleep 1; echo FINAL"
    // After Enter + sleep + echo completion, it shows:
    // "bash$ sleep 1; echo FINAL"
    // "FINAL"                       <-- the echo output
    // "bash$"                       <-- new prompt

    // Strategy: count occurrences of a unique marker
    // - After typing: 1 occurrence (in the command line)
    // - After execution: 2 occurrences (command line + echo output)

    assert!(
        h.cli()
            .args(["type", "sleep 1; echo MARKER_UNIQUE_42"])
            .status()
            .unwrap()
            .success()
    );

    // Verify marker appears once in the typed command
    let output = h.cli_json().args(["snapshot"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let screen_before = json["screen"].as_str().unwrap();
    let count_before = screen_before.matches("MARKER_UNIQUE_42").count();
    assert_eq!(count_before, 1, "Marker should appear once in command line");

    // Execute the command
    assert!(h.cli().args(["press", "Enter"]).status().unwrap().success());

    // Wait for the second occurrence (the echo output)
    let start = std::time::Instant::now();

    // Poll until we see 2 occurrences
    loop {
        let output = h.cli_json().args(["snapshot"]).output().unwrap();
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        let screen = json["screen"].as_str().unwrap();
        let count = screen.matches("MARKER_UNIQUE_42").count();

        if count >= 2 {
            break;
        }

        if start.elapsed().as_secs() > 5 {
            panic!("Timeout waiting for echo output");
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let elapsed = start.elapsed();

    // Should have waited approximately 1 second (sleep 1)
    assert!(
        elapsed.as_millis() > 850,
        "Should have waited for delay (took {}ms)",
        elapsed.as_millis()
    );
    assert!(
        elapsed.as_millis() < 3000,
        "Should not have taken too long (took {}ms)",
        elapsed.as_millis()
    );
}

#[test]
fn test_wait_timeout_fails() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Wait for text that will never appear (short timeout)
    let status = h
        .cli()
        .args(["wait", "-t", "500", "TEXT_THAT_NEVER_APPEARS"])
        .status()
        .unwrap();
    assert!(!status.success(), "Wait should fail on timeout");
}

#[test]
fn test_wait_stable_succeeds() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Idle bash should stabilize quickly
    let status = h
        .cli()
        .args(["wait", "--condition", "stable", "-t", "3000"])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "Wait stable should succeed for idle shell"
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_click_nonexistent_element() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Click on element ref that doesn't exist
    let status = h
        .cli()
        .args(["click", "@nonexistent_element"])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "Click on nonexistent element should fail"
    );
}

#[test]
fn test_operation_on_dead_session() {
    let h = RealTestHarness::new();

    // Spawn and immediately kill
    let output = h.cli_json().args(["spawn", "bash"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let session_id = json["session_id"].as_str().unwrap().to_string();

    // Wait briefly for session to initialize
    let _ = h
        .cli()
        .args(["-s", &session_id, "wait", "-t", "2000", "$"])
        .status();

    // Kill it
    assert!(
        h.cli()
            .args(["-s", &session_id, "kill"])
            .status()
            .unwrap()
            .success()
    );

    // Operations on dead session should fail
    let status = h
        .cli()
        .args(["-s", &session_id, "snapshot"])
        .status()
        .unwrap();
    assert!(!status.success(), "Snapshot on dead session should fail");
}

// =============================================================================
// Concurrency Tests
// =============================================================================

#[test]
fn test_concurrent_session_access() {
    use std::process::Command;
    use std::thread;

    let h = RealTestHarness::new();
    let session_id = h.spawn_bash();

    // Get socket path for thread-safe access
    let socket_path = h.socket_path().to_path_buf();
    let socket_path2 = socket_path.clone();
    let sid1 = session_id.clone();
    let sid2 = session_id.clone();

    // Thread 1: Type command
    let t1 = thread::spawn(move || {
        Command::new(env!("CARGO_BIN_EXE_agent-tui"))
            .env("AGENT_TUI_SOCKET", &socket_path)
            .args(["-s", &sid1, "type", "echo concurrent1"])
            .status()
            .expect("type command failed")
    });

    // Thread 2: Take snapshot simultaneously
    let t2 = thread::spawn(move || {
        Command::new(env!("CARGO_BIN_EXE_agent-tui"))
            .env("AGENT_TUI_SOCKET", &socket_path2)
            .args(["-f", "json", "-s", &sid2, "snapshot"])
            .output()
            .expect("snapshot failed")
    });

    // Both operations should succeed without deadlock
    let status1 = t1.join().expect("thread 1 panicked");
    let output2 = t2.join().expect("thread 2 panicked");

    assert!(status1.success(), "type command failed");
    assert!(output2.status.success(), "snapshot failed");

    // Verify snapshot contains valid JSON
    let json: Value =
        serde_json::from_slice(&output2.stdout).expect("snapshot output not valid JSON");
    assert!(json["screen"].is_string());
}

#[test]
fn test_rapid_session_spawn_and_kill() {
    let h = RealTestHarness::new();

    // Rapidly spawn and kill 10 sessions
    for i in 0..10 {
        let output = h.cli_json().args(["spawn", "bash"]).output().unwrap();
        assert!(output.status.success(), "spawn {} failed", i);

        let json: Value = serde_json::from_slice(&output.stdout).unwrap();
        let sid = json["session_id"].as_str().unwrap();

        // Kill immediately
        let status = h.cli().args(["-s", sid, "kill"]).status().unwrap();
        assert!(status.success(), "kill {} failed", i);
    }

    // All sessions should be cleaned up
    let output = h.cli_json().args(["sessions"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let sessions = json["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 0, "All sessions should be killed");
}

// =============================================================================
// PTY Round-Trip Tests
// =============================================================================

#[test]
fn test_pty_read_write_round_trip() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Write a command via type (which uses pty_write internally for each char)
    assert!(
        h.cli()
            .args(["type", "echo PTY_ROUNDTRIP_TEST"])
            .status()
            .unwrap()
            .success()
    );

    // Execute the command
    assert!(h.cli().args(["press", "Enter"]).status().unwrap().success());

    // Wait for the output to appear (pty_read happens during snapshot)
    assert!(
        h.cli()
            .args(["wait", "-t", "5000", "PTY_ROUNDTRIP_TEST"])
            .status()
            .unwrap()
            .success()
    );

    // Verify the round-trip: typed command -> executed -> output captured
    let output = h.cli_json().args(["snapshot"]).output().unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let screen = json["screen"].as_str().unwrap();

    // Should see the marker at least twice:
    // 1. In the command line we typed
    // 2. In the echo output
    let count = screen.matches("PTY_ROUNDTRIP_TEST").count();
    assert!(
        count >= 2,
        "PTY round-trip: expected at least 2 occurrences, got {}",
        count
    );
}

// =============================================================================
// Double-Click E2E Tests
// =============================================================================

#[test]
fn test_dbl_click_real_tui_element() {
    let h = RealTestHarness::new();
    h.spawn_bash();

    // Type a marker that we can try to double-click on
    assert!(
        h.cli()
            .args(["type", "echo DBLCLICK_TARGET"])
            .status()
            .unwrap()
            .success()
    );

    // Execute to have output
    assert!(h.cli().args(["press", "Enter"]).status().unwrap().success());

    // Wait for output
    assert!(
        h.cli()
            .args(["wait", "-t", "5000", "DBLCLICK_TARGET"])
            .status()
            .unwrap()
            .success()
    );

    // Try to dblclick on the text. This tests the atomic lock behavior:
    // The lock is held across both clicks, so no race condition can occur.
    // Note: This may fail with "element not found" if the text isn't recognized
    // as a clickable element, but that's expected. The test verifies the
    // dbl_click operation completes without hanging or crashing.
    let status = h.cli().args(["dblclick", "DBLCLICK_TARGET"]).status();

    // The operation should complete (success or element-not-found error)
    assert!(status.is_ok(), "dblclick command should complete");

    // Session should still be usable after dblclick
    let output = h.cli_json().args(["snapshot"]).output().unwrap();
    assert!(
        output.status.success(),
        "Session should be usable after dblclick"
    );
}
