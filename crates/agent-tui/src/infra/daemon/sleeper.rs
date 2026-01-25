use std::thread;
use std::time::Duration;

use crate::usecases::ports::Sleeper;

#[derive(Debug, Clone, Copy, Default)]
pub struct RealSleeper;

impl Sleeper for RealSleeper {
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
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

        assert!(elapsed >= Duration::from_millis(5));
    }
}
