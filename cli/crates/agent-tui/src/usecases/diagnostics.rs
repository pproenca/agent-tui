//! Diagnostics use case.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::domain::HealthInput;
use crate::domain::HealthOutput;
use crate::domain::MetricsInput;
use crate::domain::MetricsOutput;
use crate::domain::TerminalReadInput;
use crate::domain::TerminalReadOutput;
use crate::domain::TerminalWriteInput;
use crate::domain::TerminalWriteOutput;
use crate::usecases::ports::MetricsProvider;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::SystemInfoProvider;

pub trait TerminalReadUseCase: Send + Sync {
    fn execute(&self, input: TerminalReadInput) -> Result<TerminalReadOutput, SessionError>;
}

pub struct TerminalReadUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TerminalReadUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TerminalReadUseCase for TerminalReadUseCaseImpl<R> {
    fn execute(&self, input: TerminalReadInput) -> Result<TerminalReadOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let max_bytes = if input.max_bytes == 0 {
            4096
        } else {
            input.max_bytes
        };
        let timeout_ms = if input.timeout_ms == 0 {
            100
        } else {
            input.timeout_ms
        }
        .min(i32::MAX as u64) as i32;
        let mut buf = vec![0u8; max_bytes];
        let bytes_read = session.terminal_try_read(&mut buf, timeout_ms)?;
        buf.truncate(bytes_read);
        Ok(TerminalReadOutput {
            session_id: session.session_id(),
            data: buf,
            bytes_read,
        })
    }
}

pub trait TerminalWriteUseCase: Send + Sync {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError>;
}

pub struct TerminalWriteUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TerminalWriteUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TerminalWriteUseCase for TerminalWriteUseCaseImpl<R> {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let bytes_len = input.data.len();
        session.terminal_write(&input.data)?;
        Ok(TerminalWriteOutput {
            session_id: session.session_id(),
            bytes_written: bytes_len,
            success: true,
        })
    }
}

pub trait HealthUseCase: Send + Sync {
    fn execute(&self, input: HealthInput) -> Result<HealthOutput, SessionError>;
}

pub struct HealthUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
    metrics: Arc<dyn MetricsProvider>,
    system_info: Arc<dyn SystemInfoProvider>,
    active_connections: Arc<AtomicUsize>,
}

impl<R: SessionRepository> HealthUseCaseImpl<R> {
    pub fn new(
        repository: Arc<R>,
        metrics: Arc<dyn MetricsProvider>,
        system_info: Arc<dyn SystemInfoProvider>,
        active_connections: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            repository,
            metrics,
            system_info,
            active_connections,
        }
    }
}

impl<R: SessionRepository> HealthUseCase for HealthUseCaseImpl<R> {
    fn execute(&self, _input: HealthInput) -> Result<HealthOutput, SessionError> {
        Ok(HealthOutput {
            status: "healthy".to_string(),
            pid: self.system_info.pid(),
            uptime_ms: self.system_info.uptime_ms(),
            session_count: self.repository.session_count(),
            version: self.system_info.version(),
            commit: self.system_info.commit(),
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
    metrics: Arc<dyn MetricsProvider>,
    system_info: Arc<dyn SystemInfoProvider>,
    active_connections: Arc<AtomicUsize>,
}

impl<R: SessionRepository> MetricsUseCaseImpl<R> {
    pub fn new(
        repository: Arc<R>,
        metrics: Arc<dyn MetricsProvider>,
        system_info: Arc<dyn SystemInfoProvider>,
        active_connections: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            repository,
            metrics,
            system_info,
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
            uptime_ms: self.system_info.uptime_ms(),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            session_count: self.repository.session_count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::SessionId;
    use crate::test_support::MockError;
    use crate::test_support::MockSessionRepository;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    #[derive(Default)]
    struct TestMetrics {
        requests: AtomicU64,
        errors: AtomicU64,
        lock_timeouts: AtomicU64,
        poison_recoveries: AtomicU64,
    }

    impl TestMetrics {
        fn record_request(&self) {
            self.requests.fetch_add(1, Ordering::Relaxed);
        }

        fn record_error(&self) {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }

        fn record_lock_timeout(&self) {
            self.lock_timeouts.fetch_add(1, Ordering::Relaxed);
        }

        fn record_poison_recovery(&self) {
            self.poison_recoveries.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl MetricsProvider for TestMetrics {
        fn requests(&self) -> u64 {
            self.requests.load(Ordering::Relaxed)
        }

        fn errors(&self) -> u64 {
            self.errors.load(Ordering::Relaxed)
        }

        fn lock_timeouts(&self) -> u64 {
            self.lock_timeouts.load(Ordering::Relaxed)
        }

        fn poison_recoveries(&self) -> u64 {
            self.poison_recoveries.load(Ordering::Relaxed)
        }
    }

    struct TestSystemInfo {
        pid: u32,
        uptime_ms: u64,
        version: String,
        commit: String,
    }

    impl TestSystemInfo {
        fn new() -> Self {
            Self {
                pid: 4242,
                uptime_ms: 1234,
                version: "test-version".to_string(),
                commit: "test-commit".to_string(),
            }
        }
    }

    impl SystemInfoProvider for TestSystemInfo {
        fn pid(&self) -> u32 {
            self.pid
        }

        fn uptime_ms(&self) -> u64 {
            self.uptime_ms
        }

        fn version(&self) -> String {
            self.version.clone()
        }

        fn commit(&self) -> String {
            self.commit.clone()
        }
    }

    #[test]
    fn test_health_usecase_returns_correct_output() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_session_count(5)
                .build(),
        );
        let metrics = Arc::new(TestMetrics::default());
        metrics.record_request();
        metrics.record_request();
        metrics.record_error();

        let system_info = Arc::new(TestSystemInfo::new());
        let active_connections = Arc::new(AtomicUsize::new(3));

        let usecase = HealthUseCaseImpl::new(repo, metrics, system_info, active_connections);

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
        let metrics = Arc::new(TestMetrics::default());
        metrics.record_request();
        metrics.record_lock_timeout();
        metrics.record_poison_recovery();

        let system_info = Arc::new(TestSystemInfo::new());
        let active_connections = Arc::new(AtomicUsize::new(1));

        let usecase = MetricsUseCaseImpl::new(repo, metrics, system_info, active_connections);

        let output = usecase.execute(MetricsInput).unwrap();

        assert_eq!(output.requests_total, 1);
        assert_eq!(output.errors_total, 0);
        assert_eq!(output.lock_timeouts, 1);
        assert_eq!(output.poison_recoveries, 1);
        assert_eq!(output.active_connections, 1);
        assert_eq!(output.session_count, 2);
    }

    #[test]
    fn test_terminal_read_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = TerminalReadUseCaseImpl::new(repo);

        let input = TerminalReadInput {
            session_id: None,
            max_bytes: 4096,
            timeout_ms: 0,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_terminal_read_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = TerminalReadUseCaseImpl::new(repo);

        let input = TerminalReadInput {
            session_id: Some(SessionId::new("missing")),
            max_bytes: 1024,
            timeout_ms: 0,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_terminal_write_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = TerminalWriteUseCaseImpl::new(repo);

        let input = TerminalWriteInput {
            session_id: None,
            data: b"hello".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_terminal_write_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = TerminalWriteUseCaseImpl::new(repo);

        let input = TerminalWriteInput {
            session_id: Some(SessionId::new("missing")),
            data: b"test data".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
