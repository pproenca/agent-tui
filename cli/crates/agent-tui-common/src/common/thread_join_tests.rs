use std::sync::mpsc;
use std::time::Duration;

use super::ThreadJoinOutcome;
use super::join_thread_with_timeout_or_reap;
use super::join_thread_with_timeout_or_reap_with_poll_interval;

#[test]
fn join_thread_with_timeout_returns_joined_for_finished_thread() {
    let handle = std::thread::spawn(|| {});

    let outcome =
        join_thread_with_timeout_or_reap(handle, Duration::from_secs(1), "test thread", "reaper");

    assert_eq!(outcome, ThreadJoinOutcome::Joined);
}

#[test]
fn join_thread_with_timeout_spawns_background_reaper_on_timeout() {
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let handle = std::thread::spawn(move || {
        let _ = release_rx.recv();
        let _ = finished_tx.send(());
    });

    let outcome = join_thread_with_timeout_or_reap_with_poll_interval(
        handle,
        Duration::from_millis(10),
        Duration::from_millis(1),
        "test thread",
        "test-thread-reaper",
    );

    assert_eq!(outcome, ThreadJoinOutcome::ReapingInBackground);
    assert!(
        finished_rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "reaper handoff should return before the slow thread exits"
    );
    release_tx
        .send(())
        .expect("slow thread should still be waiting");
    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "slow thread should finish after release"
    );
}
