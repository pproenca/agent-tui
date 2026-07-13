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
    assert_eq!(config.max_connections, DEFAULT_MAX_CONNECTIONS);
    assert_eq!(
        config.idle_timeout,
        Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
    );
    assert_eq!(config.max_request_bytes, DEFAULT_MAX_REQUEST_BYTES);
    assert_eq!(config.max_sessions, DEFAULT_MAX_SESSIONS);
}

#[test]
fn test_invalid_env_uses_defaults() {
    let _max_conn = EnvGuard::set("AGENT_TUI_MAX_CONNECTIONS", "nope");
    let _idle = EnvGuard::set("AGENT_TUI_IDLE_TIMEOUT", "bad");
    let _max_req = EnvGuard::set("AGENT_TUI_MAX_REQUEST", "bad");
    let _max_sessions = EnvGuard::set("AGENT_TUI_MAX_SESSIONS", "bad");

    let config = DaemonConfig::from_env();
    assert_eq!(config.max_connections, DEFAULT_MAX_CONNECTIONS);
    assert_eq!(
        config.idle_timeout,
        Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
    );
    assert_eq!(config.max_request_bytes, DEFAULT_MAX_REQUEST_BYTES);
    assert_eq!(config.max_sessions, DEFAULT_MAX_SESSIONS);
}
