use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::time::Instant;

use crate::adapters::daemon::usecase_container::{
    DiagnosticsUseCases, InputUseCases, SessionUseCases, SnapshotUseCases, UseCaseContainer,
};
use crate::usecases::ports::{MetricsProvider, SessionRepository, ShutdownNotifierHandle};
use crate::usecases::{
    AssertUseCaseImpl, AttachUseCaseImpl, CleanupUseCaseImpl, HealthUseCaseImpl,
    KeydownUseCaseImpl, KeystrokeUseCaseImpl, KeyupUseCaseImpl, KillUseCaseImpl,
    MetricsUseCaseImpl, PtyReadUseCaseImpl, PtyWriteUseCaseImpl, ResizeUseCaseImpl,
    RestartUseCaseImpl, ScrollUseCaseImpl, SessionsUseCaseImpl, ShutdownUseCaseImpl,
    SnapshotUseCaseImpl, SpawnUseCaseImpl, TypeUseCaseImpl, WaitUseCaseImpl,
};

impl<R: SessionRepository + 'static> UseCaseContainer<R> {
    pub fn new(
        repository: Arc<R>,
        metrics_provider: Arc<dyn MetricsProvider>,
        start_time: Instant,
        active_connections: Arc<AtomicUsize>,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
    ) -> Self {
        Self {
            session: SessionUseCases {
                spawn: SpawnUseCaseImpl::new(Arc::clone(&repository)),
                kill: KillUseCaseImpl::new(Arc::clone(&repository)),
                sessions: SessionsUseCaseImpl::new(Arc::clone(&repository)),
                restart: RestartUseCaseImpl::new(Arc::clone(&repository)),
                attach: AttachUseCaseImpl::new(Arc::clone(&repository)),
                resize: ResizeUseCaseImpl::new(Arc::clone(&repository)),
                cleanup: CleanupUseCaseImpl::new(Arc::clone(&repository)),
                assert: AssertUseCaseImpl::new(Arc::clone(&repository)),
            },
            snapshot: SnapshotUseCases {
                snapshot: SnapshotUseCaseImpl::new(Arc::clone(&repository)),
            },
            input: InputUseCases {
                keystroke: KeystrokeUseCaseImpl::new(Arc::clone(&repository)),
                type_text: TypeUseCaseImpl::new(Arc::clone(&repository)),
                keydown: KeydownUseCaseImpl::new(Arc::clone(&repository)),
                keyup: KeyupUseCaseImpl::new(Arc::clone(&repository)),
                scroll: ScrollUseCaseImpl::new(Arc::clone(&repository)),
            },
            diagnostics: DiagnosticsUseCases {
                pty_read: PtyReadUseCaseImpl::new(Arc::clone(&repository)),
                pty_write: PtyWriteUseCaseImpl::new(Arc::clone(&repository)),
                health: HealthUseCaseImpl::new(
                    Arc::clone(&repository),
                    Arc::clone(&metrics_provider),
                    start_time,
                    Arc::clone(&active_connections),
                ),
                metrics: MetricsUseCaseImpl::new(
                    Arc::clone(&repository),
                    metrics_provider,
                    start_time,
                    active_connections,
                ),
                shutdown: ShutdownUseCaseImpl::new(shutdown_flag, shutdown_notifier),
            },
            wait: WaitUseCaseImpl::new(repository),
        }
    }
}
