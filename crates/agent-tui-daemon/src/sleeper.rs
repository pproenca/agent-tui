//! Sleeper trait for deterministic timing in tests.
//!
//! This module provides a `Sleeper` trait that abstracts over `thread::sleep`,
//! allowing tests to replace real sleep with mock implementations that don't
//! actually wait, making tests fast and deterministic.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

/// Trait for abstracting sleep operations.
///
/// This trait allows production code to use `RealSleeper` which actually sleeps,
/// while tests can use `MockSleeper` which records calls without sleeping.
pub trait Sleeper: Send + Sync {
    /// Sleep for the specified duration.
    fn sleep(&self, duration: Duration);
}

/// Production sleeper that uses `thread::sleep`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealSleeper;

impl Sleeper for RealSleeper {
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Mock sleeper for testing that records calls without sleeping.
///
/// This implementation tracks:
/// - Total number of sleep calls
/// - Total duration of all sleep calls
/// - Individual durations of each sleep call
#[derive(Debug, Default)]
pub struct MockSleeper {
    call_count: AtomicU64,
    total_duration_ms: AtomicU64,
    durations: Mutex<Vec<Duration>>,
}

impl MockSleeper {
    /// Create a new mock sleeper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of times sleep was called.
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Returns the total duration of all sleep calls.
    pub fn total_duration(&self) -> Duration {
        Duration::from_millis(self.total_duration_ms.load(Ordering::SeqCst))
    }

    /// Returns all individual sleep durations.
    pub fn durations(&self) -> Vec<Duration> {
        self.durations.lock().unwrap().clone()
    }

    /// Reset all tracking state.
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
        self.total_duration_ms.store(0, Ordering::SeqCst);
        self.durations.lock().unwrap().clear();
    }
}

impl Sleeper for MockSleeper {
    fn sleep(&self, duration: Duration) {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
        self.durations.lock().unwrap().push(duration);
        // Don't actually sleep - that's the whole point!
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_sleeper_sleeps() {
        let sleeper = RealSleeper;
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();
        // Should have slept for at least 10ms (allowing some margin)
        assert!(elapsed >= Duration::from_millis(5));
    }

    #[test]
    fn test_mock_sleeper_does_not_sleep() {
        let sleeper = MockSleeper::new();
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_millis(1000));
        let elapsed = start.elapsed();
        // Should complete nearly instantly (less than 5ms)
        assert!(elapsed < Duration::from_millis(5));
    }

    #[test]
    fn test_mock_sleeper_tracks_call_count() {
        let sleeper = MockSleeper::new();

        sleeper.sleep(Duration::from_millis(10));
        sleeper.sleep(Duration::from_millis(20));
        sleeper.sleep(Duration::from_millis(30));

        assert_eq!(sleeper.call_count(), 3);
    }

    #[test]
    fn test_mock_sleeper_tracks_total_duration() {
        let sleeper = MockSleeper::new();

        sleeper.sleep(Duration::from_millis(10));
        sleeper.sleep(Duration::from_millis(20));
        sleeper.sleep(Duration::from_millis(30));

        assert_eq!(sleeper.total_duration(), Duration::from_millis(60));
    }

    #[test]
    fn test_mock_sleeper_tracks_individual_durations() {
        let sleeper = MockSleeper::new();

        sleeper.sleep(Duration::from_millis(10));
        sleeper.sleep(Duration::from_millis(20));
        sleeper.sleep(Duration::from_millis(30));

        let durations = sleeper.durations();
        assert_eq!(durations.len(), 3);
        assert_eq!(durations[0], Duration::from_millis(10));
        assert_eq!(durations[1], Duration::from_millis(20));
        assert_eq!(durations[2], Duration::from_millis(30));
    }

    #[test]
    fn test_mock_sleeper_reset() {
        let sleeper = MockSleeper::new();

        sleeper.sleep(Duration::from_millis(100));
        assert_eq!(sleeper.call_count(), 1);

        sleeper.reset();

        assert_eq!(sleeper.call_count(), 0);
        assert_eq!(sleeper.total_duration(), Duration::ZERO);
        assert!(sleeper.durations().is_empty());
    }
}
