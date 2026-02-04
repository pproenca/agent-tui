//! Daemon use case wiring.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;

use crate::adapters::daemon::usecase_container::DiagnosticsUseCases;
use crate::adapters::daemon::usecase_container::InputUseCases;
use crate::adapters::daemon::usecase_container::SessionUseCases;
use crate::adapters::daemon::usecase_container::SnapshotUseCases;
use crate::adapters::daemon::usecase_container::UseCaseContainer;
use crate::usecases::AssertUseCaseImpl;
use crate::usecases::AttachUseCaseImpl;
use crate::usecases::CleanupUseCaseImpl;
use crate::usecases::HealthUseCaseImpl;
use crate::usecases::KeydownUseCaseImpl;
use crate::usecases::KeystrokeUseCaseImpl;
use crate::usecases::KeyupUseCaseImpl;
use crate::usecases::KillUseCaseImpl;
use crate::usecases::MetricsUseCaseImpl;
use crate::usecases::ResizeUseCaseImpl;
use crate::usecases::RestartUseCaseImpl;
use crate::usecases::ScrollUseCaseImpl;
use crate::usecases::SessionsUseCaseImpl;
use crate::usecases::ShutdownUseCaseImpl;
use crate::usecases::SnapshotUseCaseImpl;
use crate::usecases::SpawnUseCaseImpl;
use crate::usecases::TerminalReadUseCaseImpl;
use crate::usecases::TerminalWriteUseCaseImpl;
use crate::usecases::TypeUseCaseImpl;
use crate::usecases::WaitUseCaseImpl;
use crate::usecases::ports::Clock;
use crate::usecases::ports::MetricsProvider;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifierHandle;
use crate::usecases::ports::SystemInfoProvider;

impl<R: SessionRepository + 'static> UseCaseContainer<R> {
    pub fn new(
        repository: Arc<R>,
        metrics_provider: Arc<dyn MetricsProvider>,
        system_info: Arc<dyn SystemInfoProvider>,
        clock: Arc<dyn Clock>,
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
                terminal_read: TerminalReadUseCaseImpl::new(Arc::clone(&repository)),
                terminal_write: TerminalWriteUseCaseImpl::new(Arc::clone(&repository)),
                health: HealthUseCaseImpl::new(
                    Arc::clone(&repository),
                    Arc::clone(&metrics_provider),
                    Arc::clone(&system_info),
                    Arc::clone(&active_connections),
                ),
                metrics: MetricsUseCaseImpl::new(
                    Arc::clone(&repository),
                    metrics_provider,
                    Arc::clone(&system_info),
                    active_connections,
                ),
                shutdown: ShutdownUseCaseImpl::new(shutdown_flag, shutdown_notifier),
            },
            wait: WaitUseCaseImpl::new(repository, clock),
        }
    }
}
