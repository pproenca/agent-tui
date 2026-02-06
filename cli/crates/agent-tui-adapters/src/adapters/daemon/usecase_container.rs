//! Adapter use case wiring.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::usecases::AssertUseCaseImpl;
use crate::usecases::AttachUseCaseImpl;
use crate::usecases::CleanupUseCaseImpl;
use crate::usecases::KeydownUseCaseImpl;
use crate::usecases::KeystrokeUseCaseImpl;
use crate::usecases::KeyupUseCaseImpl;
use crate::usecases::KillUseCaseImpl;
use crate::usecases::ResizeUseCaseImpl;
use crate::usecases::RestartUseCaseImpl;
use crate::usecases::SessionsUseCaseImpl;
use crate::usecases::ShutdownUseCaseImpl;
use crate::usecases::SnapshotUseCaseImpl;
use crate::usecases::SpawnUseCaseImpl;
use crate::usecases::TerminalWriteUseCaseImpl;
use crate::usecases::TypeUseCaseImpl;
use crate::usecases::WaitUseCaseImpl;
use crate::usecases::ports::Clock;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifierHandle;

pub struct UseCaseContainer<R: SessionRepository + 'static> {
    pub session: SessionUseCases<R>,
    pub snapshot: SnapshotUseCases<R>,
    pub input: InputUseCases<R>,
    pub diagnostics: DiagnosticsUseCases<R>,
    pub wait: WaitUseCaseImpl<R>,
}

pub struct SessionUseCases<R: SessionRepository + 'static> {
    pub spawn: SpawnUseCaseImpl<R>,
    pub kill: KillUseCaseImpl<R>,
    pub sessions: SessionsUseCaseImpl<R>,
    pub restart: RestartUseCaseImpl<R>,
    pub attach: AttachUseCaseImpl<R>,
    pub resize: ResizeUseCaseImpl<R>,
    pub cleanup: CleanupUseCaseImpl<R>,
    pub assert: AssertUseCaseImpl<R>,
}

pub struct SnapshotUseCases<R: SessionRepository + 'static> {
    pub snapshot: SnapshotUseCaseImpl<R>,
}

pub struct InputUseCases<R: SessionRepository + 'static> {
    pub keystroke: KeystrokeUseCaseImpl<R>,
    pub type_text: TypeUseCaseImpl<R>,
    pub keydown: KeydownUseCaseImpl<R>,
    pub keyup: KeyupUseCaseImpl<R>,
}

pub struct DiagnosticsUseCases<R: SessionRepository + 'static> {
    pub terminal_write: TerminalWriteUseCaseImpl<R>,
    pub shutdown: ShutdownUseCaseImpl,
}

impl<R: SessionRepository + 'static> UseCaseContainer<R> {
    pub fn new(
        repository: Arc<R>,
        clock: Arc<dyn Clock>,
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
            },
            diagnostics: DiagnosticsUseCases {
                terminal_write: TerminalWriteUseCaseImpl::new(Arc::clone(&repository)),
                shutdown: ShutdownUseCaseImpl::new(shutdown_flag, shutdown_notifier),
            },
            wait: WaitUseCaseImpl::new(repository, clock),
        }
    }
}
