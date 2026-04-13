//! Daemon lock helper utilities.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::infra::daemon::session::Session;

pub const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_BACKOFF: Duration = Duration::from_millis(50);

fn compute_jitter(backoff_micros: u64) -> u64 {
    let thread_id = std::thread::current().id();
    let thread_hash = format!("{thread_id:?}")
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    if backoff_micros < 4 {
        return 0;
    }

    let jitter_range = backoff_micros / 4;
    if jitter_range == 0 {
        return 0;
    }

    (thread_hash ^ backoff_micros) % jitter_range
}

pub fn acquire_session_lock(
    session: &Arc<Mutex<Session>>,
    timeout: Duration,
) -> Option<MutexGuard<'_, Session>> {
    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);

    while start.elapsed() < timeout {
        if let Ok(guard) = session.try_lock() {
            return Some(guard);
        }
        let jitter = compute_jitter(backoff.as_micros() as u64);
        let sleep_duration = backoff + Duration::from_micros(jitter);
        thread::park_timeout(sleep_duration);
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    None
}

#[cfg(test)]
#[path = "lock_helpers_tests.rs"]
mod tests;
