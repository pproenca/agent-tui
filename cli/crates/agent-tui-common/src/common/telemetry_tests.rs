use super::*;

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Test-only environment override.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Test-only environment override.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            // SAFETY: Test-only environment restoration.
            unsafe {
                std::env::set_var(self.key, prev);
            }
        } else {
            // SAFETY: Test-only environment cleanup.
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[test]
fn test_log_format_parsing() {
    let _guard = EnvGuard::set("AGENT_TUI_LOG_FORMAT", "json");
    assert_eq!(log_format_from_env(), LogFormat::Json);

    let _guard = EnvGuard::set("AGENT_TUI_LOG_FORMAT", "text");
    assert_eq!(log_format_from_env(), LogFormat::Text);
}

#[test]
fn test_log_stream_parsing() {
    let _guard = EnvGuard::set("AGENT_TUI_LOG_STREAM", "stdout");
    assert_eq!(log_stream_from_env(), LogStream::Stdout);

    let _guard = EnvGuard::set("AGENT_TUI_LOG_STREAM", "stderr");
    assert_eq!(log_stream_from_env(), LogStream::Stderr);
}

#[test]
fn test_log_format_defaults() {
    let _guard = EnvGuard::remove("AGENT_TUI_LOG_FORMAT");
    assert_eq!(log_format_from_env(), LogFormat::Text);
}

#[test]
fn test_log_stream_defaults() {
    let _guard = EnvGuard::remove("AGENT_TUI_LOG_STREAM");
    assert_eq!(log_stream_from_env(), LogStream::Stderr);
}

#[test]
fn test_diagnostic_filter_falls_back_to_rust_log() {
    let _log_filter = EnvGuard::remove("AGENT_TUI_LOG_FILTER");
    let _rust_log = EnvGuard::set("RUST_LOG", "debug");

    let filter = diagnostic_filter_from_env("info").to_string();

    assert_eq!(filter, "debug");
}

#[test]
fn test_stream_filter_prefers_stream_specific_override() {
    let _stream_filter = EnvGuard::set("AGENT_TUI_LOG_STREAM_FILTER", "warn");
    let _log_filter = EnvGuard::set("AGENT_TUI_LOG_FILTER", "debug");

    let filter = stream_filter_from_env("info").to_string();

    assert_eq!(filter, "warn");
}
