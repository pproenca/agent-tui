use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::time::Instant;

use crate::usecases::ports::{MetricsProvider, SessionRepository, ShutdownNotifierHandle};
use crate::usecases::{
    AccessibilitySnapshotUseCaseImpl, AssertUseCaseImpl, AttachUseCaseImpl, CleanupUseCaseImpl,
    ClearUseCaseImpl, ClickUseCaseImpl, CountUseCaseImpl, DoubleClickUseCaseImpl, FillUseCaseImpl,
    FindUseCaseImpl, FocusUseCaseImpl, GetFocusedUseCaseImpl, GetTextUseCaseImpl,
    GetTitleUseCaseImpl, GetValueUseCaseImpl, HealthUseCaseImpl, IsCheckedUseCaseImpl,
    IsEnabledUseCaseImpl, IsFocusedUseCaseImpl, IsVisibleUseCaseImpl, KeydownUseCaseImpl,
    KeystrokeUseCaseImpl, KeyupUseCaseImpl, KillUseCaseImpl, MetricsUseCaseImpl,
    MultiselectUseCaseImpl, PtyReadUseCaseImpl, PtyWriteUseCaseImpl, ResizeUseCaseImpl,
    RestartUseCaseImpl, ScrollIntoViewUseCaseImpl, ScrollUseCaseImpl, SelectAllUseCaseImpl,
    SelectUseCaseImpl, SessionsUseCaseImpl, ShutdownUseCaseImpl, SnapshotUseCaseImpl,
    SpawnUseCaseImpl, ToggleUseCaseImpl, TypeUseCaseImpl, WaitUseCaseImpl,
};

pub struct UseCaseContainer<R: SessionRepository + 'static> {
    pub session: SessionUseCases<R>,
    pub elements: ElementUseCases<R>,
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

pub struct ElementUseCases<R: SessionRepository + 'static> {
    pub snapshot: SnapshotUseCaseImpl<R>,
    pub accessibility_snapshot: AccessibilitySnapshotUseCaseImpl<R>,
    pub click: ClickUseCaseImpl<R>,
    pub dbl_click: DoubleClickUseCaseImpl<R>,
    pub fill: FillUseCaseImpl<R>,
    pub find: FindUseCaseImpl<R>,
    pub scroll: ScrollUseCaseImpl<R>,
    pub count: CountUseCaseImpl<R>,
    pub focus: FocusUseCaseImpl<R>,
    pub clear: ClearUseCaseImpl<R>,
    pub select_all: SelectAllUseCaseImpl<R>,
    pub toggle: ToggleUseCaseImpl<R>,
    pub select: SelectUseCaseImpl<R>,
    pub multiselect: MultiselectUseCaseImpl<R>,

    pub get_text: GetTextUseCaseImpl<R>,
    pub get_value: GetValueUseCaseImpl<R>,
    pub is_visible: IsVisibleUseCaseImpl<R>,
    pub is_focused: IsFocusedUseCaseImpl<R>,
    pub is_enabled: IsEnabledUseCaseImpl<R>,
    pub is_checked: IsCheckedUseCaseImpl<R>,
    pub get_focused: GetFocusedUseCaseImpl<R>,
    pub get_title: GetTitleUseCaseImpl<R>,
    pub scroll_into_view: ScrollIntoViewUseCaseImpl<R>,
}

pub struct InputUseCases<R: SessionRepository + 'static> {
    pub keystroke: KeystrokeUseCaseImpl<R>,
    pub type_text: TypeUseCaseImpl<R>,
    pub keydown: KeydownUseCaseImpl<R>,
    pub keyup: KeyupUseCaseImpl<R>,
}

pub struct DiagnosticsUseCases<R: SessionRepository + 'static> {
    pub pty_read: PtyReadUseCaseImpl<R>,
    pub pty_write: PtyWriteUseCaseImpl<R>,
    pub health: HealthUseCaseImpl<R>,
    pub metrics: MetricsUseCaseImpl<R>,
    pub shutdown: ShutdownUseCaseImpl,
}

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
            elements: ElementUseCases {
                snapshot: SnapshotUseCaseImpl::new(Arc::clone(&repository)),
                accessibility_snapshot: AccessibilitySnapshotUseCaseImpl::new(Arc::clone(
                    &repository,
                )),
                click: ClickUseCaseImpl::new(Arc::clone(&repository)),
                dbl_click: DoubleClickUseCaseImpl::new(Arc::clone(&repository)),
                fill: FillUseCaseImpl::new(Arc::clone(&repository)),
                find: FindUseCaseImpl::new(Arc::clone(&repository)),
                scroll: ScrollUseCaseImpl::new(Arc::clone(&repository)),
                count: CountUseCaseImpl::new(Arc::clone(&repository)),
                focus: FocusUseCaseImpl::new(Arc::clone(&repository)),
                clear: ClearUseCaseImpl::new(Arc::clone(&repository)),
                select_all: SelectAllUseCaseImpl::new(Arc::clone(&repository)),
                toggle: ToggleUseCaseImpl::new(Arc::clone(&repository)),
                select: SelectUseCaseImpl::new(Arc::clone(&repository)),
                multiselect: MultiselectUseCaseImpl::new(Arc::clone(&repository)),
                get_text: GetTextUseCaseImpl::new(Arc::clone(&repository)),
                get_value: GetValueUseCaseImpl::new(Arc::clone(&repository)),
                is_visible: IsVisibleUseCaseImpl::new(Arc::clone(&repository)),
                is_focused: IsFocusedUseCaseImpl::new(Arc::clone(&repository)),
                is_enabled: IsEnabledUseCaseImpl::new(Arc::clone(&repository)),
                is_checked: IsCheckedUseCaseImpl::new(Arc::clone(&repository)),
                get_focused: GetFocusedUseCaseImpl::new(Arc::clone(&repository)),
                get_title: GetTitleUseCaseImpl::new(Arc::clone(&repository)),
                scroll_into_view: ScrollIntoViewUseCaseImpl::new(Arc::clone(&repository)),
            },
            input: InputUseCases {
                keystroke: KeystrokeUseCaseImpl::new(Arc::clone(&repository)),
                type_text: TypeUseCaseImpl::new(Arc::clone(&repository)),
                keydown: KeydownUseCaseImpl::new(Arc::clone(&repository)),
                keyup: KeyupUseCaseImpl::new(Arc::clone(&repository)),
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
