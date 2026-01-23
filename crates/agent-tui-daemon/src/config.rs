use std::env;
use std::time::Duration;

use crate::session::DEFAULT_MAX_SESSIONS;

const DEFAULT_MAX_CONNECTIONS: usize = 64;
const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 5;
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_REQUEST_BYTES: usize = 1_048_576; // 1MB

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub max_connections: usize,
    pub lock_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_request_bytes: usize,
    pub max_sessions: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl DaemonConfig {
    pub fn from_env() -> Self {
        Self {
            max_connections: env::var("AGENT_TUI_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_CONNECTIONS),
            lock_timeout: Duration::from_secs(
                env::var("AGENT_TUI_LOCK_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_LOCK_TIMEOUT_SECS),
            ),
            idle_timeout: Duration::from_secs(
                env::var("AGENT_TUI_IDLE_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS),
            ),
            max_request_bytes: env::var("AGENT_TUI_MAX_REQUEST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_REQUEST_BYTES),
            max_sessions: env::var("AGENT_TUI_MAX_SESSIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MAX_SESSIONS),
        }
    }

    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }

    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    pub fn with_max_request_bytes(mut self, max: usize) -> Self {
        self.max_request_bytes = max;
        self
    }

    pub fn with_max_sessions(mut self, max: usize) -> Self {
        self.max_sessions = max;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.max_connections, DEFAULT_MAX_CONNECTIONS);
        assert_eq!(
            config.lock_timeout,
            Duration::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
        assert_eq!(
            config.idle_timeout,
            Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
        );
        assert_eq!(config.max_request_bytes, DEFAULT_MAX_REQUEST_BYTES);
        assert_eq!(config.max_sessions, DEFAULT_MAX_SESSIONS);
    }

    #[test]
    fn test_builder_pattern() {
        let config = DaemonConfig::default()
            .with_max_connections(128)
            .with_lock_timeout(Duration::from_secs(10))
            .with_idle_timeout(Duration::from_secs(600))
            .with_max_request_bytes(2_097_152)
            .with_max_sessions(32);

        assert_eq!(config.max_connections, 128);
        assert_eq!(config.lock_timeout, Duration::from_secs(10));
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert_eq!(config.max_request_bytes, 2_097_152);
        assert_eq!(config.max_sessions, 32);
    }
}
