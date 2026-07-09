//! Bounded thread join helpers.

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use tracing::warn;

const DEFAULT_JOIN_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadJoinOutcome {
    Joined,
    ReapingInBackground,
}

pub fn join_thread_with_timeout_or_reap(
    handle: thread::JoinHandle<()>,
    timeout: Duration,
    thread_label: &'static str,
    reaper_name: &'static str,
) -> ThreadJoinOutcome {
    join_thread_with_timeout_or_reap_with_poll_interval(
        handle,
        timeout,
        DEFAULT_JOIN_POLL_INTERVAL,
        thread_label,
        reaper_name,
    )
}

pub fn join_thread_with_timeout_or_reap_with_poll_interval(
    handle: thread::JoinHandle<()>,
    timeout: Duration,
    poll_interval: Duration,
    thread_label: &'static str,
    reaper_name: &'static str,
) -> ThreadJoinOutcome {
    let deadline = Instant::now() + timeout;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            warn!(
                thread = thread_label,
                timeout_ms = timeout.as_millis(),
                "Timed out joining thread; handing ownership to background reaper"
            );
            return spawn_join_reaper(handle, thread_label, reaper_name);
        }
        thread::park_timeout(poll_interval);
    }
    join_thread_and_warn_on_panic(handle, thread_label);
    ThreadJoinOutcome::Joined
}

pub fn join_thread_and_warn_on_panic(handle: thread::JoinHandle<()>, thread_label: &'static str) {
    if handle.join().is_err() {
        warn!(thread = thread_label, "Joined thread exited with a panic");
    }
}

fn spawn_join_reaper(
    handle: thread::JoinHandle<()>,
    thread_label: &'static str,
    reaper_name: &'static str,
) -> ThreadJoinOutcome {
    let handle_cell = Arc::new(Mutex::new(Some(handle)));
    let handle_for_thread = Arc::clone(&handle_cell);
    match thread::Builder::new()
        .name(reaper_name.to_string())
        .spawn(move || {
            let Some(handle) = handle_for_thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            else {
                return;
            };

            join_thread_and_warn_on_panic(handle, thread_label);
        }) {
        Ok(_) => ThreadJoinOutcome::ReapingInBackground,
        Err(err) => {
            warn!(
                thread = thread_label,
                reaper = reaper_name,
                error = %err,
                "Failed to spawn background join reaper; joining synchronously"
            );
            let handle = handle_cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take();
            if let Some(handle) = handle {
                join_thread_and_warn_on_panic(handle, thread_label);
            }
            ThreadJoinOutcome::Joined
        }
    }
}

#[cfg(test)]
#[path = "thread_join_tests.rs"]
mod tests;
