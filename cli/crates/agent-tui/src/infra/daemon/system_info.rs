//! System info implementation.

use std::time::Instant;

use crate::usecases::ports::SystemInfoProvider;

pub struct SystemInfo {
    start_time: Instant,
}

impl SystemInfo {
    pub fn new(start_time: Instant) -> Self {
        Self { start_time }
    }
}

impl SystemInfoProvider for SystemInfo {
    fn pid(&self) -> u32 {
        std::process::id()
    }

    fn uptime_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    fn version(&self) -> String {
        env!("AGENT_TUI_VERSION").to_string()
    }

    fn commit(&self) -> String {
        env!("AGENT_TUI_GIT_SHA").to_string()
    }
}
