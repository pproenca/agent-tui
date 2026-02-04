//! Daemon configuration.

use std::env;
use std::time::Duration;

use crate::infra::daemon::session::DEFAULT_MAX_SESSIONS;
use tracing::warn;

const DEFAULT_MAX_CONNECTIONS: usize = 64;
const DEFAULT_LOCK_TIMEOUT_SECS: u64 = 5;
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_REQUEST_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    max_connections: usize,
    lock_timeout: Duration,
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

    pub fn lock_timeout(&self) -> Duration {
        self.lock_timeout
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
            lock_timeout: Duration::from_secs(parse_env_u64(
                "AGENT_TUI_LOCK_TIMEOUT",
                DEFAULT_LOCK_TIMEOUT_SECS,
            )),
            idle_timeout: Duration::from_secs(parse_env_u64(
                "AGENT_TUI_IDLE_TIMEOUT",
                DEFAULT_IDLE_TIMEOUT_SECS,
            )),
            max_request_bytes: parse_env_usize("AGENT_TUI_MAX_REQUEST", DEFAULT_MAX_REQUEST_BYTES),
            max_sessions: parse_env_usize("AGENT_TUI_MAX_SESSIONS", DEFAULT_MAX_SESSIONS),
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
mod tests {
    use super::*;
    use std::env;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = env::var(key).ok();
            // SAFETY: Test-only environment override.
            unsafe {
                env::set_var(key, value);
            }
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = self.prev.take() {
                // SAFETY: Test-only environment restoration.
                unsafe {
                    env::set_var(self.key, prev);
                }
            } else {
                // SAFETY: Test-only environment cleanup.
                unsafe {
                    env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.max_connections(), DEFAULT_MAX_CONNECTIONS);
        assert_eq!(
            config.lock_timeout(),
            Duration::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
        assert_eq!(
            config.idle_timeout(),
            Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
        );
        assert_eq!(config.max_request_bytes(), DEFAULT_MAX_REQUEST_BYTES);
        assert_eq!(config.max_sessions(), DEFAULT_MAX_SESSIONS);
    }

    #[test]
    fn test_builder_pattern() {
        let config = DaemonConfig::default()
            .with_max_connections(128)
            .with_lock_timeout(Duration::from_secs(10))
            .with_idle_timeout(Duration::from_secs(600))
            .with_max_request_bytes(2_097_152)
            .with_max_sessions(32);

        assert_eq!(config.max_connections(), 128);
        assert_eq!(config.lock_timeout(), Duration::from_secs(10));
        assert_eq!(config.idle_timeout(), Duration::from_secs(600));
        assert_eq!(config.max_request_bytes(), 2_097_152);
        assert_eq!(config.max_sessions(), 32);
    }

    #[test]
    fn test_invalid_env_uses_defaults() {
        let _max_conn = EnvGuard::set("AGENT_TUI_MAX_CONNECTIONS", "nope");
        let _lock = EnvGuard::set("AGENT_TUI_LOCK_TIMEOUT", "bad");
        let _idle = EnvGuard::set("AGENT_TUI_IDLE_TIMEOUT", "bad");
        let _max_req = EnvGuard::set("AGENT_TUI_MAX_REQUEST", "bad");
        let _max_sessions = EnvGuard::set("AGENT_TUI_MAX_SESSIONS", "bad");

        let config = DaemonConfig::from_env();
        assert_eq!(config.max_connections(), DEFAULT_MAX_CONNECTIONS);
        assert_eq!(
            config.lock_timeout(),
            Duration::from_secs(DEFAULT_LOCK_TIMEOUT_SECS)
        );
        assert_eq!(
            config.idle_timeout(),
            Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
        );
        assert_eq!(config.max_request_bytes(), DEFAULT_MAX_REQUEST_BYTES);
        assert_eq!(config.max_sessions(), DEFAULT_MAX_SESSIONS);
    }
}
