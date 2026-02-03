use crate::usecases::ports::SessionRepository;
use crate::usecases::{
    AssertUseCaseImpl, AttachUseCaseImpl, CleanupUseCaseImpl, HealthUseCaseImpl,
    KeydownUseCaseImpl, KeystrokeUseCaseImpl, KeyupUseCaseImpl, KillUseCaseImpl,
    MetricsUseCaseImpl, ResizeUseCaseImpl, RestartUseCaseImpl, ScrollUseCaseImpl,
    SessionsUseCaseImpl, ShutdownUseCaseImpl, SnapshotUseCaseImpl, SpawnUseCaseImpl,
    TerminalReadUseCaseImpl, TerminalWriteUseCaseImpl, TypeUseCaseImpl, WaitUseCaseImpl,
};

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
    pub scroll: ScrollUseCaseImpl<R>,
}

pub struct DiagnosticsUseCases<R: SessionRepository + 'static> {
    pub terminal_read: TerminalReadUseCaseImpl<R>,
    pub terminal_write: TerminalWriteUseCaseImpl<R>,
    pub health: HealthUseCaseImpl<R>,
    pub metrics: MetricsUseCaseImpl<R>,
    pub shutdown: ShutdownUseCaseImpl,
}
