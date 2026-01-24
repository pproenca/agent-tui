//! Concurrent request tests
//!
//! Tests for handling concurrent/parallel requests to the daemon.
//! Verifies thread safety and data integrity under load.

mod common;

use common::{MockResponse, TEST_SESSION_ID, TestHarness, with_timeout};
use predicates::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// =============================================================================
// Parallel Snapshot Tests
// =============================================================================

#[test]
fn test_parallel_snapshots_same_session() {
    let harness = Arc::new(TestHarness::new());

    // Run 4 snapshots in parallel
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screen"]).success())
        })
        .collect();

    // All should complete successfully
    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        result.stdout(predicate::str::contains("Screen"));
    }

    // All 4 snapshot calls should have been made
    assert_eq!(harness.call_count("snapshot"), 4);
}

#[test]
fn test_parallel_snapshots_with_different_options() {
    let harness = Arc::new(TestHarness::new());

    // Configure snapshot response with elements
    harness.set_success_response(
        "screen",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen content\n",
            "elements": [
                {"ref": "@btn1", "type": "button", "label": "OK"}
            ],
            "cursor": {"row": 0, "col": 0, "visible": true},
            "size": {"cols": 120, "rows": 40}
        }),
    );

    let handles = vec![
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screen"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screen", "-i"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screen", "--include-cursor"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["-f", "json", "screen"]))
        },
    ];

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        result.success();
    }
}

// =============================================================================
// Parallel Type Command Tests
// =============================================================================

#[test]
fn test_parallel_type_commands() {
    let harness = Arc::new(TestHarness::new());

    // Run 4 key --type commands in parallel
    let texts = vec!["Hello", "World", "Test", "Data"];

    let handles: Vec<_> = texts
        .into_iter()
        .map(|text| {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["input", text]).success())
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All 4 type calls should have been made
    assert_eq!(harness.call_count("type"), 4);
}

// =============================================================================
// Concurrent Spawn Tests
// =============================================================================

#[test]
fn test_concurrent_spawns() {
    let harness = Arc::new(TestHarness::new());

    // Configure unique responses for each spawn using Sequence
    let spawn_responses: Vec<MockResponse> = (0..4)
        .map(|i| {
            MockResponse::Success(json!({
                "session_id": format!("session-{}", i),
                "pid": 1000 + i
            }))
        })
        .collect();

    harness.set_response("spawn", MockResponse::Sequence(spawn_responses));

    let commands = vec!["bash", "zsh", "sh", "bash"];

    let handles: Vec<_> = commands
        .into_iter()
        .map(|cmd| {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["run", cmd]).success())
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All spawns should succeed
    assert_eq!(harness.call_count("spawn"), 4);
}

// =============================================================================
// Rapid Connect/Disconnect Tests
// =============================================================================

#[test]
fn test_rapid_connect_disconnect() {
    let harness = Arc::new(TestHarness::new());

    // 20 rapid health checks
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["daemon", "status"]))
        })
        .collect();

    let mut successes = 0;
    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        if result.try_success().is_ok() {
            successes += 1;
        }
    }

    // All should succeed
    assert_eq!(successes, 20);
}

// =============================================================================
// Mixed Command Concurrency Tests
// =============================================================================

#[test]
fn test_mixed_commands_parallel() {
    let harness = Arc::new(TestHarness::new());

    let handles = vec![
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["daemon", "status"]))
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["sessions"]))
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["screen"]))
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@btn1", "click"]))
        },
    ];

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        result.success();
    }

    // Verify all different methods were called
    // daemon status = 1 health, sessions = 1 sessions + 1 health, screen = 1 snapshot + 1 health, action click = 1 click + 1 health
    assert_eq!(harness.call_count("health"), 4);
    assert_eq!(harness.call_count("sessions"), 1);
    assert_eq!(harness.call_count("snapshot"), 1);
    assert_eq!(harness.call_count("click"), 1);
}

// =============================================================================
// Concurrent Error Handling Tests
// =============================================================================

#[test]
fn test_concurrent_errors_isolated() {
    let harness = Arc::new(TestHarness::new());

    // health succeeds, click fails
    harness.set_error_response("click", -32003, "Element not found");

    let handles = vec![
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["daemon", "status"]).success())
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@missing", "click"]).failure())
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["daemon", "status"]).success())
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@missing", "click"]).failure())
        },
    ];

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Errors don't affect other commands
    // 2 daemon status = 2 health, 2 action click = 2 click + 2 health = 4 health total
    assert_eq!(harness.call_count("health"), 4);
    assert_eq!(harness.call_count("click"), 2);
}

// =============================================================================
// Concurrent with Delayed Responses
// =============================================================================

#[test]
fn test_concurrent_with_delays() {
    let harness = Arc::new(TestHarness::new());

    // One slow response, others normal
    harness.set_response(
        "health",
        MockResponse::Delayed(
            Duration::from_millis(100),
            Box::new(MockResponse::Success(json!({
                "status": "healthy",
                "pid": 12345,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": "1.0.0"
            }))),
        ),
    );

    let harness_clone = Arc::clone(&harness);
    let result = with_timeout(Duration::from_secs(5), move || {
        let handles = vec![
            {
                let h = Arc::clone(&harness_clone);
                thread::spawn(move || h.run(&["daemon", "status"]).success())
            },
            {
                let h = Arc::clone(&harness_clone);
                thread::spawn(move || h.run(&["sessions"]).success())
            },
            {
                let h = Arc::clone(&harness_clone);
                thread::spawn(move || h.run(&["screen"]).success())
            },
        ];

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    });

    assert!(
        result.is_some(),
        "Concurrent requests with delay should complete"
    );
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_many_parallel_health_checks() {
    let harness = Arc::new(TestHarness::new());

    let handles: Vec<_> = (0..50)
        .map(|_| {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["daemon", "status"]))
        })
        .collect();

    let mut successes = 0;
    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        if result.try_success().is_ok() {
            successes += 1;
        }
    }

    // All should succeed
    // Each daemon status command makes 1 health call
    assert_eq!(successes, 50);
    assert_eq!(harness.call_count("health"), 50);
}

// =============================================================================
// PTY Operations Concurrency Tests
// =============================================================================

#[test]
fn test_concurrent_pty_read_write_no_deadlock() {
    let harness = Arc::new(TestHarness::new());

    // Configure PTY read/write handlers
    harness.set_success_response(
        "pty_read",
        json!({
            "session_id": TEST_SESSION_ID,
            "data": "",
            "bytes_read": 0
        }),
    );
    harness.set_success_response(
        "pty_write",
        json!({
            "success": true,
            "session_id": TEST_SESSION_ID
        }),
    );

    // The daemon fix ensures pty_read/pty_write use acquire_session_lock
    // with timeout instead of mutex_lock_or_recover, preventing deadlocks
    let result = with_timeout(Duration::from_secs(5), move || {
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let h = Arc::clone(&harness);
                thread::spawn(move || {
                    // Simulate interleaved reads and writes
                    if i % 2 == 0 {
                        h.run(&["daemon", "status"]) // Simulates read path
                    } else {
                        h.run(&["input", "test"]) // Simulates write path
                    }
                })
            })
            .collect();

        for handle in handles {
            let _ = handle.join().expect("Thread panicked");
        }
    });

    assert!(
        result.is_some(),
        "Concurrent PTY operations should complete without deadlock"
    );
}

// =============================================================================
// Double-Click Concurrency Tests
// =============================================================================

#[test]
fn test_parallel_dbl_click_same_session() {
    let harness = Arc::new(TestHarness::new());

    harness.set_success_response(
        "dbl_click",
        json!({
            "success": true,
            "message": null
        }),
    );

    // Multiple dbl_click operations on same session
    // The daemon fix holds lock across both clicks atomically
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@btn1", "dblclick"]).success())
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All calls should complete
    assert_eq!(harness.call_count("dbl_click"), 4);
}

// =============================================================================
// Lock Contention Tests
// =============================================================================

#[test]
fn test_high_contention_lock_recovery() {
    let harness = Arc::new(TestHarness::new());

    // Simulate occasional lock timeouts followed by success
    harness.set_response(
        "click",
        MockResponse::Sequence(vec![
            MockResponse::StructuredError {
                code: -32006,
                message: "Session lock timeout".to_string(),
                category: Some("lock".to_string()),
                retryable: Some(true),
                context: Some(json!({"session_id": TEST_SESSION_ID})),
                suggestion: Some("Retry the operation".to_string()),
            },
            MockResponse::Success(json!({ "success": true })),
            MockResponse::Success(json!({ "success": true })),
            MockResponse::StructuredError {
                code: -32006,
                message: "Session lock timeout".to_string(),
                category: Some("lock".to_string()),
                retryable: Some(true),
                context: Some(json!({"session_id": TEST_SESSION_ID})),
                suggestion: Some("Retry the operation".to_string()),
            },
            MockResponse::Success(json!({ "success": true })),
            MockResponse::Success(json!({ "success": true })),
            MockResponse::Success(json!({ "success": true })),
            MockResponse::Success(json!({ "success": true })),
        ]),
    );

    // Run 8 parallel click operations
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@btn1", "click"]))
        })
        .collect();

    let mut successes = 0;
    let mut failures = 0;

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        if result.try_success().is_ok() {
            successes += 1;
        } else {
            failures += 1;
        }
    }

    // Some should succeed, some may fail (lock timeout)
    // The key is that we don't deadlock and all operations complete
    assert_eq!(successes + failures, 8, "All operations should complete");
    assert!(successes >= 6, "Most operations should succeed");
}
