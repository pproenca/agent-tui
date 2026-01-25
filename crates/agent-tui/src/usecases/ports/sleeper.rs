use std::time::Duration;

pub trait Sleeper: Send + Sync {
    fn sleep(&self, duration: Duration);
}

#[derive(Debug, Default)]
pub struct MockSleeper {
    call_count: std::sync::atomic::AtomicU64,
    total_duration_ms: std::sync::atomic::AtomicU64,
    durations: std::sync::Mutex<Vec<Duration>>,
}

impl MockSleeper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn call_count(&self) -> u64 {
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn total_duration(&self) -> Duration {
        Duration::from_millis(
            self.total_duration_ms
                .load(std::sync::atomic::Ordering::SeqCst),
        )
    }

    pub fn durations(&self) -> Vec<Duration> {
        self.durations.lock().unwrap().clone()
    }

    pub fn reset(&self) {
        self.call_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.total_duration_ms
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.durations.lock().unwrap().clear();
    }
}

impl Sleeper for MockSleeper {
    fn sleep(&self, duration: Duration) {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.total_duration_ms.fetch_add(
            duration.as_millis() as u64,
            std::sync::atomic::Ordering::SeqCst,
        );
        self.durations.lock().unwrap().push(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_sleeper_does_not_sleep() {
        let sleeper = MockSleeper::new();
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_millis(1000));
        let elapsed = start.elapsed();

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
}
