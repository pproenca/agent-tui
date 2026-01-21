//! Session lock acquisition helpers

use crate::session::Session;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// Default lock timeout for handlers
pub const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

/// Try to acquire a session lock with timeout and exponential backoff
/// Returns None if unable to acquire lock within timeout
///
/// Uses exponential backoff starting at 100µs, doubling up to 50ms max,
/// to reduce CPU usage during contention while staying responsive.
pub fn acquire_session_lock(
    session: &Arc<Mutex<Session>>,
    timeout: Duration,
) -> Option<MutexGuard<'_, Session>> {
    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    const MAX_BACKOFF: Duration = Duration::from_millis(50);

    while start.elapsed() < timeout {
        if let Ok(guard) = session.try_lock() {
            return Some(guard);
        }
        thread::sleep(backoff);
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    None
}
