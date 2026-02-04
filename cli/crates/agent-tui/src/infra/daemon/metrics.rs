use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::usecases::ports::MetricsProvider;

pub struct DaemonMetrics {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub lock_timeouts: AtomicU64,
    pub poison_recoveries: AtomicU64,
    start_time: Instant,
}

impl Default for DaemonMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            lock_timeouts: AtomicU64::new(0),
            poison_recoveries: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_lock_timeout(&self) {
        self.lock_timeouts.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_poison_recovery(&self) {
        self.poison_recoveries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn requests(&self) -> u64 {
        self.requests_total.load(Ordering::Relaxed)
    }

    pub fn errors(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    pub fn lock_timeouts(&self) -> u64 {
        self.lock_timeouts.load(Ordering::Relaxed)
    }

    pub fn poison_recoveries(&self) -> u64 {
        self.poison_recoveries.load(Ordering::Relaxed)
    }

    pub fn uptime_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

impl MetricsProvider for DaemonMetrics {
    fn requests(&self) -> u64 {
        self.requests()
    }

    fn errors(&self) -> u64 {
        self.errors()
    }

    fn lock_timeouts(&self) -> u64 {
        self.lock_timeouts()
    }

    fn poison_recoveries(&self) -> u64 {
        self.poison_recoveries()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initial_values() {
        let metrics = DaemonMetrics::new();
        assert_eq!(metrics.requests(), 0);
        assert_eq!(metrics.errors(), 0);
        assert_eq!(metrics.lock_timeouts(), 0);
        assert_eq!(metrics.poison_recoveries(), 0);
    }

    #[test]
    fn test_metrics_increment() {
        let metrics = DaemonMetrics::new();
        metrics.record_request();
        metrics.record_request();
        metrics.record_error();
        assert_eq!(metrics.requests(), 2);
        assert_eq!(metrics.errors(), 1);
    }

    #[test]
    fn test_uptime_increases() {
        let metrics = DaemonMetrics::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(metrics.uptime_ms() >= 10);
    }
}
