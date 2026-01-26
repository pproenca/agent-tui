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
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    let (writer, guard) = match log_file_path_from_env() {
        Some(path) => match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => {
                let (non_blocking, guard) = tracing_appender::non_blocking(file);
                (BoxMakeWriter::new(non_blocking), Some(guard))
            }
            Err(err) => {
                eprintln!(
                    "Warning: failed to open log file {}: {}",
                    path.display(),
                    err
                );
                (BoxMakeWriter::new(std::io::stderr), None)
            }
        },
        None => (BoxMakeWriter::new(std::io::stderr), None),
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(writer);

    if subscriber.try_init().is_err() {
        return TelemetryGuard::disabled();
    }

    TelemetryGuard { _guard: guard }
}

fn log_file_path_from_env() -> Option<PathBuf> {
    std::env::var("AGENT_TUI_LOG").ok().map(PathBuf::from)
}
