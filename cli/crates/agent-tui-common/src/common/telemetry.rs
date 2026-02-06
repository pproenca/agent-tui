#![expect(clippy::print_stderr, reason = "Tracing not initialized yet")]

//! Telemetry and tracing setup.

use std::io::IsTerminal;
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

#[derive(Debug)]
pub struct TelemetryGuard {
    _guard: Option<WorkerGuard>,
}

impl TelemetryGuard {
    fn disabled() -> Self {
        Self { _guard: None }
    }
}

pub fn init_tracing(default_level: &str) -> TelemetryGuard {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let log_format = log_format_from_env();
    let log_stream = log_stream_from_env();
    let (writer, guard, ansi) = match log_file_path_from_env() {
        Some(path) => match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                let (non_blocking, guard) = tracing_appender::non_blocking(file);
                (BoxMakeWriter::new(non_blocking), Some(guard), false)
            }
            Err(err) => {
                eprintln!(
                    "Warning: failed to open log file {}: {}",
                    path.display(),
                    err
                );
                (
                    BoxMakeWriter::new(std::io::stderr),
                    None,
                    std::io::stderr().is_terminal(),
                )
            }
        },
        None => match log_stream {
            LogStream::Stdout => (
                BoxMakeWriter::new(std::io::stdout),
                None,
                std::io::stdout().is_terminal(),
            ),
            LogStream::Stderr => (
                BoxMakeWriter::new(std::io::stderr),
                None,
                std::io::stderr().is_terminal(),
            ),
        },
    };

    let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = match log_format {
        LogFormat::Json => Box::new(
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_ansi(false)
                .json()
                .with_writer(writer)
                .finish(),
        ),
        LogFormat::Text => Box::new(
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_ansi(ansi)
                .with_writer(writer)
                .finish(),
        ),
    };

    if tracing::subscriber::set_global_default(subscriber).is_err() {
        return TelemetryGuard::disabled();
    }

    TelemetryGuard { _guard: guard }
}

fn log_file_path_from_env() -> Option<PathBuf> {
    std::env::var("AGENT_TUI_LOG").ok().map(PathBuf::from)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LogStream {
    Stderr,
    Stdout,
}

fn log_format_from_env() -> LogFormat {
    match std::env::var("AGENT_TUI_LOG_FORMAT")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("json") => LogFormat::Json,
        _ => LogFormat::Text,
    }
}

fn log_stream_from_env() -> LogStream {
    match std::env::var("AGENT_TUI_LOG_STREAM")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("stdout") => LogStream::Stdout,
        _ => LogStream::Stderr,
    }
}

#[cfg(test)]
mod tests {
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
}
