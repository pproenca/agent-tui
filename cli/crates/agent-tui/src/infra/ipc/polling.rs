//! IPC polling configuration.

use std::time::Duration;

pub const MAX_STARTUP_POLLS: u32 = 50;
pub const INITIAL_POLL_INTERVAL: Duration = Duration::from_millis(50);
pub const MAX_POLL_INTERVAL: Duration = Duration::from_millis(500);
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
