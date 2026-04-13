#![expect(clippy::print_stderr, reason = "Tracing not initialized yet")]

//! Telemetry and tracing setup.

use std::io::IsTerminal;
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;

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
    let log_format = log_format_from_env();
    let log_stream = log_stream_from_env();
    let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = match log_file_path_from_env() {
        Some(path) => match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                let (operator_writer, operator_ansi) = stream_writer(log_stream);
                let (diagnostic_writer, guard) = tracing_appender::non_blocking(file);
                let operator_filter = stream_filter_from_env(default_level);
                let diagnostic_filter = diagnostic_filter_from_env(default_level);

                let subscriber: Box<dyn tracing::Subscriber + Send + Sync> = match log_format {
                    LogFormat::Json => Box::new(
                        tracing_subscriber::registry()
                            .with(
                                tracing_subscriber::fmt::layer()
                                    .with_target(false)
                                    .with_thread_ids(true)
                                    .with_thread_names(true)
                                    .with_ansi(operator_ansi)
                                    .with_writer(operator_writer)
                                    .with_filter(operator_filter),
                            )
                            .with(
                                tracing_subscriber::fmt::layer()
                                    .with_target(true)
                                    .with_thread_ids(false)
                                    .with_thread_names(false)
                                    .with_ansi(false)
                                    .json()
                                    .with_writer(diagnostic_writer)
                                    .with_filter(diagnostic_filter),
                            ),
                    ),
                    LogFormat::Text => Box::new(
                        tracing_subscriber::registry()
                            .with(
                                tracing_subscriber::fmt::layer()
                                    .with_target(false)
                                    .with_thread_ids(true)
                                    .with_thread_names(true)
                                    .with_ansi(operator_ansi)
                                    .with_writer(operator_writer)
                                    .with_filter(operator_filter),
                            )
                            .with(
                                tracing_subscriber::fmt::layer()
                                    .with_target(true)
                                    .with_thread_ids(true)
                                    .with_thread_names(true)
                                    .with_ansi(false)
                                    .with_writer(diagnostic_writer)
                                    .with_filter(diagnostic_filter),
                            ),
                    ),
                };

                if tracing::subscriber::set_global_default(subscriber).is_err() {
                    return TelemetryGuard::disabled();
                }

                return TelemetryGuard {
                    _guard: Some(guard),
                };
            }
            Err(err) => {
                eprintln!(
                    "Warning: failed to open log file {}: {}",
                    path.display(),
                    err
                );
                single_sink_subscriber(
                    log_stream,
                    log_format,
                    diagnostic_filter_from_env(default_level),
                )
            }
        },
        None => single_sink_subscriber(
            log_stream,
            log_format,
            diagnostic_filter_from_env(default_level),
        ),
    };

    if tracing::subscriber::set_global_default(subscriber).is_err() {
        return TelemetryGuard::disabled();
    }

    TelemetryGuard::disabled()
}

fn single_sink_subscriber(
    log_stream: LogStream,
    log_format: LogFormat,
    env_filter: EnvFilter,
) -> Box<dyn tracing::Subscriber + Send + Sync> {
    let (writer, ansi) = stream_writer(log_stream);

    match log_format {
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
    }
}

fn stream_writer(log_stream: LogStream) -> (BoxMakeWriter, bool) {
    match log_stream {
        LogStream::Stdout => (
            BoxMakeWriter::new(std::io::stdout),
            std::io::stdout().is_terminal(),
        ),
        LogStream::Stderr => (
            BoxMakeWriter::new(std::io::stderr),
            std::io::stderr().is_terminal(),
        ),
    }
}

fn log_file_path_from_env() -> Option<PathBuf> {
    std::env::var("AGENT_TUI_LOG").ok().map(PathBuf::from)
}

fn diagnostic_filter_from_env(default_level: &str) -> EnvFilter {
    env_filter_from_env(&["AGENT_TUI_LOG_FILTER", "RUST_LOG"], default_level)
}

fn stream_filter_from_env(default_level: &str) -> EnvFilter {
    env_filter_from_env(
        &[
            "AGENT_TUI_LOG_STREAM_FILTER",
            "AGENT_TUI_LOG_FILTER",
            "RUST_LOG",
        ],
        default_level,
    )
}

fn env_filter_from_env(keys: &[&str], default_level: &str) -> EnvFilter {
    for key in keys {
        let Ok(value) = std::env::var(key) else {
            continue;
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }

        match EnvFilter::try_new(trimmed) {
            Ok(filter) => return filter,
            Err(err) => eprintln!("Warning: invalid {key} filter '{trimmed}': {err}"),
        }
    }

    EnvFilter::new(default_level)
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
#[path = "telemetry_tests.rs"]
mod tests;
