//! System clock implementation.

use std::time::Instant;

use crate::usecases::ports::Clock;

#[derive(Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
