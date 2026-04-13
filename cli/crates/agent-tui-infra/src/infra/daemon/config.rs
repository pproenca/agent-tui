//! Daemon configuration.

use std::env;
use std::time::Duration;

use crate::infra::daemon::session::DEFAULT_MAX_SESSIONS;
use tracing::warn;

const DEFAULT_MAX_CONNECTIONS: usize = 64;
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_REQUEST_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    max_connections: usize,
    idle_timeout: Duration,
    max_request_bytes: usize,
    max_sessions: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl DaemonConfig {
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    pub fn idle_timeout(&self) -> Duration {
        self.idle_timeout
    }

    pub fn max_request_bytes(&self) -> usize {
        self.max_request_bytes
    }

    pub fn max_sessions(&self) -> usize {
        self.max_sessions
    }

    pub fn from_env() -> Self {
        Self {
            max_connections: parse_env_usize("AGENT_TUI_MAX_CONNECTIONS", DEFAULT_MAX_CONNECTIONS),
            idle_timeout: Duration::from_secs(parse_env_u64(
                "AGENT_TUI_IDLE_TIMEOUT",
                DEFAULT_IDLE_TIMEOUT_SECS,
            )),
            max_request_bytes: parse_env_usize("AGENT_TUI_MAX_REQUEST", DEFAULT_MAX_REQUEST_BYTES),
            max_sessions: parse_env_usize("AGENT_TUI_MAX_SESSIONS", DEFAULT_MAX_SESSIONS),
        }
    }
}

fn parse_env_usize(key: &str, default: usize) -> usize {
    let value = match env::var(key) {
        Ok(value) => value,
        Err(_) => return default,
    };
    if value.trim().is_empty() {
        return default;
    }
    match value.parse::<usize>() {
        Ok(parsed) => parsed,
        Err(_) => {
            warn!(value = %value, key, "Invalid numeric config; using default");
            default
        }
    }
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    let value = match env::var(key) {
        Ok(value) => value,
        Err(_) => return default,
    };
    if value.trim().is_empty() {
        return default;
    }
    match value.parse::<u64>() {
        Ok(parsed) => parsed,
        Err(_) => {
            warn!(value = %value, key, "Invalid numeric config; using default");
            default
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
