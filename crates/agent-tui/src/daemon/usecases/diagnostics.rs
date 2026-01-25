use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::common::mutex_lock_or_recover;
use crate::terminal::PtyError;

use crate::daemon::domain::{
    ConsoleInput, ConsoleOutput, ErrorsInput, ErrorsOutput, HealthInput, HealthOutput,
    MetricsInput, MetricsOutput, PtyReadInput, PtyReadOutput, PtyWriteInput, PtyWriteOutput,
    TraceInput, TraceOutput,
};
use crate::daemon::error::SessionError;
use crate::daemon::metrics::DaemonMetrics;
use crate::daemon::repository::SessionRepository;

pub trait TraceUseCase: Send + Sync {
    fn execute(&self, input: TraceInput) -> Result<TraceOutput, SessionError>;
}

pub struct TraceUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TraceUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TraceUseCase for TraceUseCaseImpl<R> {
    fn execute(&self, input: TraceInput) -> Result<TraceOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let count = if input.count == 0 { 1000 } else { input.count };
        let entries = session_guard.get_trace_entries(count);

        Ok(TraceOutput {
            tracing: true,
            entries,
        })
    }
}

pub trait ConsoleUseCase: Send + Sync {
    fn execute(&self, input: ConsoleInput) -> Result<ConsoleOutput, SessionError>;
}

pub struct ConsoleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ConsoleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ConsoleUseCase for ConsoleUseCaseImpl<R> {
    fn execute(&self, input: ConsoleInput) -> Result<ConsoleOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        let _ = session_guard.update();

        let screen_text = session_guard.screen_text();
        let lines: Vec<String> = screen_text.lines().map(String::from).collect();

        Ok(ConsoleOutput { lines })
    }
}

pub trait ErrorsUseCase: Send + Sync {
    fn execute(&self, input: ErrorsInput) -> Result<ErrorsOutput, SessionError>;
}

pub struct ErrorsUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ErrorsUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ErrorsUseCase for ErrorsUseCaseImpl<R> {
    fn execute(&self, input: ErrorsInput) -> Result<ErrorsOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let count = if input.count == 0 { 1000 } else { input.count };
        let errors = session_guard.get_errors(count);

        Ok(ErrorsOutput {
            total_count: errors.len(),
            errors,
        })
    }
}

pub trait PtyReadUseCase: Send + Sync {
    fn execute(&self, input: PtyReadInput) -> Result<PtyReadOutput, SessionError>;
}

pub struct PtyReadUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> PtyReadUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> PtyReadUseCase for PtyReadUseCaseImpl<R> {
    fn execute(&self, input: PtyReadInput) -> Result<PtyReadOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let max_bytes = if input.max_bytes == 0 {
            4096
        } else {
            input.max_bytes
        };
        let mut buf = vec![0u8; max_bytes];

        match session_guard.pty_try_read(&mut buf, 100) {
            Ok(bytes_read) => {
                buf.truncate(bytes_read);
                Ok(PtyReadOutput {
                    session_id: session_guard.id.clone(),
                    data: buf,
                    bytes_read,
                })
            }
            Err(e) => Err(SessionError::Pty(PtyError::Read(e.to_string()))),
        }
    }
}

pub trait PtyWriteUseCase: Send + Sync {
    fn execute(&self, input: PtyWriteInput) -> Result<PtyWriteOutput, SessionError>;
}

pub struct PtyWriteUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> PtyWriteUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> PtyWriteUseCase for PtyWriteUseCaseImpl<R> {
    fn execute(&self, input: PtyWriteInput) -> Result<PtyWriteOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let bytes_len = input.data.len();
        match session_guard.pty_write(&input.data) {
            Ok(()) => Ok(PtyWriteOutput {
                session_id: session_guard.id.clone(),
                bytes_written: bytes_len,
                success: true,
            }),
            Err(e) => Err(SessionError::Pty(PtyError::Write(e.to_string()))),
        }
    }
}

pub trait HealthUseCase: Send + Sync {
    fn execute(&self, input: HealthInput) -> Result<HealthOutput, SessionError>;
}

pub struct HealthUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
    metrics: Arc<DaemonMetrics>,
    start_time: Instant,
    active_connections: Arc<AtomicUsize>,
}

impl<R: SessionRepository> HealthUseCaseImpl<R> {
    pub fn new(
        repository: Arc<R>,
        metrics: Arc<DaemonMetrics>,
        start_time: Instant,
        active_connections: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            repository,
            metrics,
            start_time,
            active_connections,
        }
    }
}

impl<R: SessionRepository> HealthUseCase for HealthUseCaseImpl<R> {
    fn execute(&self, _input: HealthInput) -> Result<HealthOutput, SessionError> {
        Ok(HealthOutput {
            status: "healthy".to_string(),
            pid: std::process::id(),
            uptime_ms: self.start_time.elapsed().as_millis() as u64,
            session_count: self.repository.session_count(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_requests: self.metrics.requests(),
            error_count: self.metrics.errors(),
        })
    }
}

pub trait MetricsUseCase: Send + Sync {
    fn execute(&self, input: MetricsInput) -> Result<MetricsOutput, SessionError>;
}

pub struct MetricsUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
    metrics: Arc<DaemonMetrics>,
    start_time: Instant,
    active_connections: Arc<AtomicUsize>,
}

impl<R: SessionRepository> MetricsUseCaseImpl<R> {
    pub fn new(
        repository: Arc<R>,
        metrics: Arc<DaemonMetrics>,
        start_time: Instant,
        active_connections: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            repository,
            metrics,
            start_time,
            active_connections,
        }
    }
}

impl<R: SessionRepository> MetricsUseCase for MetricsUseCaseImpl<R> {
    fn execute(&self, _input: MetricsInput) -> Result<MetricsOutput, SessionError> {
        Ok(MetricsOutput {
            requests_total: self.metrics.requests(),
            errors_total: self.metrics.errors(),
            lock_timeouts: self.metrics.lock_timeouts(),
            poison_recoveries: self.metrics.poison_recoveries(),
            uptime_ms: self.start_time.elapsed().as_millis() as u64,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            session_count: self.repository.session_count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::daemon::domain::SessionId;
    use crate::daemon::test_support::{MockError, MockSessionRepository};

    #[test]
    fn test_health_usecase_returns_correct_output() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_session_count(5)
                .build(),
        );
        let metrics = Arc::new(DaemonMetrics::new());
        metrics.record_request();
        metrics.record_request();
        metrics.record_error();

        let active_connections = Arc::new(AtomicUsize::new(3));
        let start_time = Instant::now();

        let usecase = HealthUseCaseImpl::new(repo, metrics, start_time, active_connections);

        let output = usecase.execute(HealthInput).unwrap();

        assert_eq!(output.status, "healthy");
        assert_eq!(output.session_count, 5);
        assert_eq!(output.active_connections, 3);
        assert_eq!(output.total_requests, 2);
        assert_eq!(output.error_count, 1);
        assert!(!output.version.is_empty());
    }

    #[test]
    fn test_metrics_usecase_returns_correct_output() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_session_count(2)
                .build(),
        );
        let metrics = Arc::new(DaemonMetrics::new());
        metrics.record_request();
        metrics.record_lock_timeout();
        metrics.record_poison_recovery();

        let active_connections = Arc::new(AtomicUsize::new(1));
        let start_time = Instant::now();

        let usecase = MetricsUseCaseImpl::new(repo, metrics, start_time, active_connections);

        let output = usecase.execute(MetricsInput).unwrap();

        assert_eq!(output.requests_total, 1);
        assert_eq!(output.errors_total, 0);
        assert_eq!(output.lock_timeouts, 1);
        assert_eq!(output.poison_recoveries, 1);
        assert_eq!(output.active_connections, 1);
        assert_eq!(output.session_count, 2);
    }

    #[test]
    fn test_trace_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = TraceUseCaseImpl::new(repo);

        let input = TraceInput {
            session_id: None,
            start: false,
            stop: false,
            count: 100,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_trace_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = TraceUseCaseImpl::new(repo);

        let input = TraceInput {
            session_id: Some(SessionId::new("missing")),
            start: true,
            stop: false,
            count: 50,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_console_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ConsoleUseCaseImpl::new(repo);

        let input = ConsoleInput {
            session_id: None,
            count: 100,
            clear: false,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_console_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ConsoleUseCaseImpl::new(repo);

        let input = ConsoleInput {
            session_id: Some(SessionId::new("missing")),
            count: 50,
            clear: true,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_errors_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ErrorsUseCaseImpl::new(repo);

        let input = ErrorsInput {
            session_id: None,
            count: 100,
            clear: false,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_errors_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ErrorsUseCaseImpl::new(repo);

        let input = ErrorsInput {
            session_id: Some(SessionId::new("missing")),
            count: 50,
            clear: false,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_pty_read_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = PtyReadUseCaseImpl::new(repo);

        let input = PtyReadInput {
            session_id: None,
            max_bytes: 4096,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_pty_read_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = PtyReadUseCaseImpl::new(repo);

        let input = PtyReadInput {
            session_id: Some(SessionId::new("missing")),
            max_bytes: 1024,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_pty_write_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = PtyWriteUseCaseImpl::new(repo);

        let input = PtyWriteInput {
            session_id: None,
            data: b"hello".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_pty_write_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = PtyWriteUseCaseImpl::new(repo);

        let input = PtyWriteInput {
            session_id: Some(SessionId::new("missing")),
            data: b"test data".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
