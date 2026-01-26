#[path = "../common/mod.rs"]
mod common;

use common::{MockResponse, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn max_parallel() -> usize {
    let env_override = std::env::var("AGENT_TUI_TEST_MAX_PARALLEL")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0);

    let default_parallel = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(16);

    env_override.unwrap_or(default_parallel).max(1)
}

fn run_batched<T, F>(count: usize, f: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> T + Send + Sync + 'static,
{
    let mut results = Vec::with_capacity(count);
    let mut handles = Vec::new();
    let limit = max_parallel();
    let f = Arc::new(f);

    for i in 0..count {
        let f = Arc::clone(&f);
        handles.push(thread::spawn(move || f(i)));

        if handles.len() >= limit {
            for handle in handles.drain(..) {
                results.push(handle.join().expect("Thread panicked"));
            }
        }
    }

    for handle in handles {
        results.push(handle.join().expect("Thread panicked"));
    }

    results
}

#[test]
fn test_parallel_snapshots_same_session() {
    let harness = Arc::new(TestHarness::new());

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screenshot"]).success())
        })
        .collect();

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        result.stdout(predicate::str::contains("Screenshot"));
    }
}

#[test]
fn test_parallel_snapshots_with_different_options() {
    let harness = Arc::new(TestHarness::new());

    harness.set_success_response(
        "screenshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screenshot": "Test screen content\n",
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
            thread::spawn(move || harness.run(&["screenshot"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screenshot", "-e"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["screenshot", "--include-cursor"]))
        },
        {
            let harness = Arc::clone(&harness);
            thread::spawn(move || harness.run(&["-f", "json", "screenshot"]))
        },
    ];

    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        result.success();
    }
}

#[test]
fn test_parallel_type_commands() {
    let harness = Arc::new(TestHarness::new());

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
}

#[test]
fn test_concurrent_spawns() {
    let harness = Arc::new(TestHarness::new());

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
}

#[test]
fn test_rapid_connect_disconnect() {
    let harness = Arc::new(TestHarness::new());

    let mut successes = 0;
    let results = run_batched(20, {
        let harness = Arc::clone(&harness);
        move |_| {
            let h = Arc::clone(&harness);
            h.run(&["daemon", "status"])
        }
    });

    for result in results {
        if result.try_success().is_ok() {
            successes += 1;
        }
    }

    assert_eq!(successes, 20);
}

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
            thread::spawn(move || h.run(&["screenshot"]))
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
}

#[test]
fn test_concurrent_errors_isolated() {
    let harness = Arc::new(TestHarness::new());

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
}

#[test]
fn test_concurrent_with_delays() {
    let harness = Arc::new(TestHarness::new());

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

    let handles = vec![
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["daemon", "status"]).success())
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["sessions"]).success())
        },
        {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["screenshot"]).success())
        },
    ];

    harness.wait_for_request_count(3);
    harness.wait_for_pending_delays(1);
    assert!(
        !handles[0].is_finished(),
        "Expected daemon status to wait for delayed response"
    );
    harness.advance_time(Duration::from_millis(100));

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_many_parallel_health_checks() {
    let harness = Arc::new(TestHarness::new());

    let mut successes = 0;
    let results = run_batched(50, {
        let harness = Arc::clone(&harness);
        move |_| {
            let h = Arc::clone(&harness);
            h.run(&["daemon", "status"])
        }
    });

    for result in results {
        if result.try_success().is_ok() {
            successes += 1;
        }
    }

    assert_eq!(successes, 50);
}

#[test]
fn test_concurrent_pty_read_write_no_deadlock() {
    let harness = Arc::new(TestHarness::new());

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

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let h = Arc::clone(&harness);
            thread::spawn(move || {
                if i % 2 == 0 {
                    h.run(&["daemon", "status"])
                } else {
                    h.run(&["input", "test"])
                }
            })
        })
        .collect();

    harness.wait_for_request_count(10);

    for handle in handles {
        let _ = handle.join().expect("Thread panicked");
    }
}

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

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let h = Arc::clone(&harness);
            thread::spawn(move || h.run(&["action", "@btn1", "dblclick"]).success())
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_high_contention_lock_recovery() {
    let harness = Arc::new(TestHarness::new());

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

    assert_eq!(successes + failures, 8, "All operations should complete");
    assert!(successes >= 6, "Most operations should succeed");
}
