use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::session::Session;

pub const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_BACKOFF: Duration = Duration::from_millis(50);

fn compute_jitter(backoff_micros: u64) -> u64 {
    let thread_id = std::thread::current().id();
    let thread_hash = format!("{:?}", thread_id)
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
        thread::sleep(sleep_duration);
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_respects_max() {
        let mut backoff = Duration::from_micros(100);
        for _ in 0..20 {
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
        assert_eq!(backoff, MAX_BACKOFF);
    }

    #[test]
    fn test_jitter_deterministic_per_thread() {
        let backoff = 1000u64;
        let jitter1 = compute_jitter(backoff);
        let jitter2 = compute_jitter(backoff);
        assert_eq!(jitter1, jitter2);
    }

    #[test]
    fn test_jitter_bounded() {
        for backoff in [100u64, 1000, 10000, 50000] {
            let jitter = compute_jitter(backoff);
            assert!(jitter <= backoff / 4);
        }
    }

    #[test]
    fn test_jitter_zero_for_tiny_backoff() {
        assert_eq!(compute_jitter(0), 0);
        assert_eq!(compute_jitter(3), 0);
    }

    #[test]
    fn test_acquire_lock_with_simple_mutex() {
        let data = Arc::new(Mutex::new(42i32));
        let start = Instant::now();
        let mut backoff = Duration::from_micros(100);
        let timeout = Duration::from_millis(100);

        while start.elapsed() < timeout {
            if let Ok(guard) = data.try_lock() {
                assert_eq!(*guard, 42);
                return;
            }
            let jitter = compute_jitter(backoff.as_micros() as u64);
            thread::sleep(backoff + Duration::from_micros(jitter));
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
        panic!("Should have acquired lock");
    }

    #[test]
    fn test_lock_timeout_with_held_mutex() {
        let data = Arc::new(Mutex::new(42i32));
        let _held = data.lock().unwrap();
        let start = Instant::now();
        let mut backoff = Duration::from_micros(100);
        let timeout = Duration::from_millis(50);

        while start.elapsed() < timeout {
            if data.try_lock().is_ok() {
                panic!("Should not acquire lock while held");
            }
            let jitter = compute_jitter(backoff.as_micros() as u64);
            thread::sleep(backoff + Duration::from_micros(jitter));
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
        assert!(start.elapsed() >= Duration::from_millis(50));
    }
}
