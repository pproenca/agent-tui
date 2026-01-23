use std::sync::Arc;

use crate::repository::SessionRepository;
use crate::usecases::{
    AttachUseCaseImpl, ClickUseCaseImpl, CountUseCaseImpl, FillUseCaseImpl, FindUseCaseImpl,
    KeystrokeUseCaseImpl, KillUseCaseImpl, ResizeUseCaseImpl, RestartUseCaseImpl,
    ScrollUseCaseImpl, SessionsUseCaseImpl, SnapshotUseCaseImpl, SpawnUseCaseImpl, TypeUseCaseImpl,
    WaitUseCaseImpl,
};

/// Container holding all use case implementations.
///
/// This enables dependency injection and makes handlers testable
/// by allowing different use case implementations to be injected.
pub struct UseCaseContainer<R: SessionRepository + 'static> {
    pub session: SessionUseCases<R>,
    pub elements: ElementUseCases<R>,
    pub input: InputUseCases<R>,
    pub wait: WaitUseCaseImpl<R>,
}

/// Session-related use cases.
pub struct SessionUseCases<R: SessionRepository + 'static> {
    pub spawn: SpawnUseCaseImpl<R>,
    pub kill: KillUseCaseImpl<R>,
    pub sessions: SessionsUseCaseImpl<R>,
    pub restart: RestartUseCaseImpl<R>,
    pub attach: AttachUseCaseImpl<R>,
    pub resize: ResizeUseCaseImpl<R>,
}

/// Element-related use cases.
pub struct ElementUseCases<R: SessionRepository + 'static> {
    pub snapshot: SnapshotUseCaseImpl<R>,
    pub click: ClickUseCaseImpl<R>,
    pub fill: FillUseCaseImpl<R>,
    pub find: FindUseCaseImpl<R>,
    pub scroll: ScrollUseCaseImpl<R>,
    pub count: CountUseCaseImpl<R>,
}

/// Input-related use cases.
pub struct InputUseCases<R: SessionRepository + 'static> {
    pub keystroke: KeystrokeUseCaseImpl<R>,
    pub type_text: TypeUseCaseImpl<R>,
}

impl<R: SessionRepository + 'static> UseCaseContainer<R> {
    /// Create a new UseCaseContainer with all use cases initialized.
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            session: SessionUseCases {
                spawn: SpawnUseCaseImpl::new(Arc::clone(&repository)),
                kill: KillUseCaseImpl::new(Arc::clone(&repository)),
                sessions: SessionsUseCaseImpl::new(Arc::clone(&repository)),
                restart: RestartUseCaseImpl::new(Arc::clone(&repository)),
                attach: AttachUseCaseImpl::new(Arc::clone(&repository)),
                resize: ResizeUseCaseImpl::new(Arc::clone(&repository)),
            },
            elements: ElementUseCases {
                snapshot: SnapshotUseCaseImpl::new(Arc::clone(&repository)),
                click: ClickUseCaseImpl::new(Arc::clone(&repository)),
                fill: FillUseCaseImpl::new(Arc::clone(&repository)),
                find: FindUseCaseImpl::new(Arc::clone(&repository)),
                scroll: ScrollUseCaseImpl::new(Arc::clone(&repository)),
                count: CountUseCaseImpl::new(Arc::clone(&repository)),
            },
            input: InputUseCases {
                keystroke: KeystrokeUseCaseImpl::new(Arc::clone(&repository)),
                type_text: TypeUseCaseImpl::new(Arc::clone(&repository)),
            },
            wait: WaitUseCaseImpl::new(repository),
        }
    }
}
