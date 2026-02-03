use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;

    fn elapsed(&self, start: Instant) -> Duration {
        self.now().duration_since(start)
    }

    fn elapsed_ms(&self, start: Instant) -> u64 {
        self.elapsed(start).as_millis() as u64
    }
}
