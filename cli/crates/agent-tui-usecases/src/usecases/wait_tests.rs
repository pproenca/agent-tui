use super::*;
use crate::domain::SessionId;
use crate::domain::TerminalSize;
use crate::domain::core::CursorPosition;
use crate::test_support::MockError;
use crate::test_support::MockSession;
use crate::test_support::MockSessionRepository;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SessionHandle;
use crate::usecases::ports::SessionOps;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiter;
use crate::usecases::ports::StreamWaiterHandle;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Instant;

#[derive(Default)]
struct TestClock;

impl Clock for TestClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct ManualClock {
    start: Instant,
    elapsed_ms: AtomicU64,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            elapsed_ms: AtomicU64::new(0),
        }
    }
}

impl ManualClock {
    fn advance(&self, duration: Duration) {
        let millis = duration.as_millis().min(u64::MAX as u128) as u64;
        self.elapsed_ms.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        self.start + Duration::from_millis(self.elapsed_ms.load(Ordering::SeqCst))
    }
}

struct WaitAction {
    advance: Duration,
    wake: bool,
    next_screen: Option<String>,
}

struct ControlledSessionState {
    screen_text: String,
    queued_screens: VecDeque<String>,
    queued_wait_actions: VecDeque<WaitAction>,
    wait_timeouts: Vec<Duration>,
    update_calls: usize,
}

struct ControlledSession {
    id: SessionId,
    state: Arc<Mutex<ControlledSessionState>>,
    clock: Arc<ManualClock>,
}

struct ControlledWaiter {
    state: Arc<Mutex<ControlledSessionState>>,
    clock: Arc<ManualClock>,
}

impl ControlledSession {
    fn new(id: &str, initial_screen: &str, clock: Arc<ManualClock>) -> Self {
        Self {
            id: SessionId::try_new(id).expect("controlled session id should be valid"),
            state: Arc::new(Mutex::new(ControlledSessionState {
                screen_text: initial_screen.to_string(),
                queued_screens: VecDeque::new(),
                queued_wait_actions: VecDeque::new(),
                wait_timeouts: Vec::new(),
                update_calls: 0,
            })),
            clock,
        }
    }

    fn queue_screen(&self, screen_text: &str) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.queued_screens.push_back(screen_text.to_string());
    }

    fn queue_wait_action(&self, advance_ms: u64, wake: bool, next_screen: Option<&str>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.queued_wait_actions.push_back(WaitAction {
            advance: Duration::from_millis(advance_ms),
            wake,
            next_screen: next_screen.map(str::to_string),
        });
    }

    fn wait_timeouts(&self) -> Vec<Duration> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.wait_timeouts.clone()
    }

    fn update_calls(&self) -> usize {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.update_calls
    }
}

impl StreamWaiter for ControlledWaiter {
    fn wait(&self, timeout: Option<Duration>) -> bool {
        let default_advance = timeout.unwrap_or_default();
        let action = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(timeout) = timeout {
                state.wait_timeouts.push(timeout);
            }
            state.queued_wait_actions.pop_front().unwrap_or(WaitAction {
                advance: default_advance,
                wake: false,
                next_screen: None,
            })
        };

        self.clock.advance(action.advance);

        if let Some(next_screen) = action.next_screen {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.screen_text = next_screen;
        }

        action.wake
    }
}

impl SessionOps for ControlledSession {
    fn update(&self) -> Result<(), SessionError> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.update_calls += 1;
        if let Some(next_screen) = state.queued_screens.pop_front() {
            state.screen_text = next_screen;
        }
        Ok(())
    }

    fn screen_text(&self) -> String {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.screen_text.clone()
    }

    fn screen_render(&self) -> String {
        self.screen_text()
    }

    fn screen_render_compact(&self) -> String {
        self.screen_text()
    }

    fn terminal_write(&self, _data: &[u8]) -> Result<(), SessionError> {
        Ok(())
    }

    fn terminal_try_read(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize, SessionError> {
        Ok(0)
    }

    fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        _max_bytes: usize,
        _timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        Ok(StreamRead {
            data: Vec::new(),
            next_cursor: *cursor,
            latest_cursor: *cursor,
            dropped_bytes: 0,
            closed: false,
        })
    }

    fn stream_subscribe(&self) -> StreamWaiterHandle {
        Arc::new(ControlledWaiter {
            state: Arc::clone(&self.state),
            clock: Arc::clone(&self.clock),
        })
    }

    fn keystroke(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn type_text(&self, _text: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn keydown(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn keyup(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn mouse_click(&self, _col: u16, _row: u16, _button: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn mouse_move(&self, _col: u16, _row: u16) -> Result<(), SessionError> {
        Ok(())
    }

    fn mouse_down(&self, _col: u16, _row: u16, _button: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn mouse_up(&self, _col: u16, _row: u16, _button: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }

    fn resize(&self, _size: TerminalSize) -> Result<(), SessionError> {
        Ok(())
    }

    fn cursor(&self) -> CursorPosition {
        CursorPosition {
            row: 0,
            col: 0,
            visible: false,
        }
    }

    fn session_id(&self) -> SessionId {
        self.id.clone()
    }

    fn command(&self) -> String {
        "controlled".to_string()
    }

    fn size(&self) -> TerminalSize {
        TerminalSize::default()
    }

    fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        LivePreviewSnapshot {
            cols: TerminalSize::default().cols(),
            rows: TerminalSize::default().rows(),
            seq: self.screen_text(),
            stream_seq: 0,
        }
    }
}

#[test]
fn test_wait_usecase_can_be_constructed_with_mock_clock() {
    let repo = Arc::new(MockSessionRepository::new());
    let clock = Arc::new(TestClock);
    let _usecase = WaitUseCaseImpl::new(repo, clock);
}

#[test]
fn test_wait_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let clock = Arc::new(TestClock);
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let input = WaitInput {
        session_id: None,
        text: Some("loading".to_string()),
        timeout_ms: 5000,
        condition: None,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_wait_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let clock = Arc::new(TestClock);
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let input = WaitInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        text: Some("ready".to_string()),
        timeout_ms: 1000,
        condition: None,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_wait_usecase_returns_error_with_stable_condition() {
    let repo = Arc::new(MockSessionRepository::new());
    let clock = Arc::new(TestClock);
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let input = WaitInput {
        session_id: None,
        text: None,
        timeout_ms: 5000,
        condition: Some(crate::domain::WaitConditionType::Stable),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_wait_usecase_returns_found_when_condition_matches() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(Arc::new(
                MockSession::builder("ready-session")
                    .with_screen_text("system ready")
                    .build(),
            ))
            .build(),
    );
    let clock = Arc::new(TestClock);
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let input = WaitInput {
        session_id: Some(SessionId::try_new("ready-session").expect("valid session id")),
        text: Some("ready".to_string()),
        timeout_ms: 1000,
        condition: None,
    };

    let result = usecase.execute(input).expect("wait should succeed");
    assert!(result.found);
}

#[test]
fn test_wait_usecase_limits_last_poll_to_remaining_timeout() {
    let clock = Arc::new(ManualClock::default());
    let session = Arc::new(ControlledSession::new(
        "timed-session",
        "booting",
        Arc::clone(&clock),
    ));
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone() as SessionHandle)
            .build(),
    );
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let result = usecase
        .execute(WaitInput {
            session_id: Some(SessionId::try_new("timed-session").expect("valid session id")),
            text: Some("ready".to_string()),
            timeout_ms: 120,
            condition: None,
        })
        .expect("wait timeout should succeed");

    assert!(!result.found);
    assert_eq!(result.elapsed_ms, 120);
    assert_eq!(
        session.wait_timeouts(),
        vec![
            Duration::from_millis(50),
            Duration::from_millis(50),
            Duration::from_millis(20),
        ]
    );
    assert_eq!(session.update_calls(), 4);
}

#[test]
fn test_wait_usecase_returns_on_stream_wakeup_before_poll_interval() {
    let clock = Arc::new(ManualClock::default());
    let session = Arc::new(ControlledSession::new(
        "wakeup-session",
        "booting",
        Arc::clone(&clock),
    ));
    session.queue_wait_action(5, true, Some("system ready"));
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone() as SessionHandle)
            .build(),
    );
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let result = usecase
        .execute(WaitInput {
            session_id: Some(SessionId::try_new("wakeup-session").expect("valid session id")),
            text: Some("ready".to_string()),
            timeout_ms: 1000,
            condition: None,
        })
        .expect("wait should succeed after wakeup");

    assert!(result.found);
    assert_eq!(result.elapsed_ms, 5);
    assert_eq!(session.wait_timeouts(), vec![Duration::from_millis(50)]);
    assert_eq!(session.update_calls(), 2);
}

#[test]
fn test_wait_usecase_stable_condition_requires_three_matching_updates_over_time() {
    let clock = Arc::new(ManualClock::default());
    let session = Arc::new(ControlledSession::new(
        "stable-session",
        "booting",
        Arc::clone(&clock),
    ));
    session.queue_screen("warming");
    session.queue_screen("ready");
    session.queue_screen("ready");
    session.queue_screen("ready");
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone() as SessionHandle)
            .build(),
    );
    let usecase = WaitUseCaseImpl::new(repo, clock);

    let result = usecase
        .execute(WaitInput {
            session_id: Some(SessionId::try_new("stable-session").expect("valid session id")),
            text: None,
            timeout_ms: 300,
            condition: Some(crate::domain::WaitConditionType::Stable),
        })
        .expect("stable wait should succeed");

    assert!(result.found);
    assert_eq!(result.elapsed_ms, 150);
    assert_eq!(
        session.wait_timeouts(),
        vec![
            Duration::from_millis(50),
            Duration::from_millis(50),
            Duration::from_millis(50),
        ]
    );
    assert_eq!(session.update_calls(), 4);
}

// WaitCondition parsing is covered in wait_condition.rs tests.
