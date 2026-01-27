use crate::adapters::ipc::{RpcRequest, RpcResponse};
use serde_json::json;

use super::usecase_container::UseCaseContainer;
use crate::adapters::daemon::handlers;
use crate::usecases::ports::SessionRepository;

pub struct Router<'a, R: SessionRepository + 'static> {
    usecases: &'a UseCaseContainer<R>,
}

impl<'a, R: SessionRepository + 'static> Router<'a, R> {
    pub fn new(usecases: &'a UseCaseContainer<R>) -> Self {
        Self { usecases }
    }

    pub fn route(&self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "ping" => RpcResponse::success(request.id, json!({ "pong": true })),

            "health" => {
                handlers::diagnostics::handle_health_uc(&self.usecases.diagnostics.health, request)
            }

            "metrics" => handlers::diagnostics::handle_metrics_uc(
                &self.usecases.diagnostics.metrics,
                request,
            ),

            "spawn" => handlers::session::handle_spawn(&self.usecases.session.spawn, request),
            "kill" => handlers::session::handle_kill(&self.usecases.session.kill, request),
            "restart" => handlers::session::handle_restart(&self.usecases.session.restart, request),
            "sessions" => {
                handlers::session::handle_sessions(&self.usecases.session.sessions, request)
            }
            "resize" => handlers::session::handle_resize(&self.usecases.session.resize, request),
            "attach" => handlers::session::handle_attach(&self.usecases.session.attach, request),
            "cleanup" => handlers::session::handle_cleanup(&self.usecases.session.cleanup, request),
            "assert" => handlers::session::handle_assert(&self.usecases.session.assert, request),
            "snapshot" => {
                handlers::elements::handle_snapshot_uc(&self.usecases.elements.snapshot, request)
            }
            "accessibility_snapshot" => handlers::elements::handle_accessibility_snapshot_uc(
                &self.usecases.elements.accessibility_snapshot,
                request,
            ),
            "click" => handlers::elements::handle_click_uc(&self.usecases.elements.click, request),
            "dbl_click" => {
                handlers::elements::handle_dbl_click_uc(&self.usecases.elements.dbl_click, request)
            }
            "fill" => handlers::elements::handle_fill_uc(&self.usecases.elements.fill, request),
            "find" => handlers::elements::handle_find_uc(&self.usecases.elements.find, request),
            "count" => handlers::elements::handle_count_uc(&self.usecases.elements.count, request),
            "scroll" => {
                handlers::elements::handle_scroll_uc(&self.usecases.elements.scroll, request)
            }
            "scroll_into_view" => handlers::elements::handle_scroll_into_view_uc(
                &self.usecases.elements.scroll_into_view,
                request,
            ),
            "get_text" => {
                handlers::elements::handle_get_text_uc(&self.usecases.elements.get_text, request)
            }
            "get_value" => {
                handlers::elements::handle_get_value_uc(&self.usecases.elements.get_value, request)
            }
            "is_visible" => handlers::elements::handle_is_visible_uc(
                &self.usecases.elements.is_visible,
                request,
            ),
            "is_focused" => handlers::elements::handle_is_focused_uc(
                &self.usecases.elements.is_focused,
                request,
            ),
            "is_enabled" => handlers::elements::handle_is_enabled_uc(
                &self.usecases.elements.is_enabled,
                request,
            ),
            "is_checked" => handlers::elements::handle_is_checked_uc(
                &self.usecases.elements.is_checked,
                request,
            ),
            "get_focused" => handlers::elements::handle_get_focused_uc(
                &self.usecases.elements.get_focused,
                request,
            ),
            "get_title" => {
                handlers::elements::handle_get_title_uc(&self.usecases.elements.get_title, request)
            }
            "focus" => handlers::elements::handle_focus_uc(&self.usecases.elements.focus, request),
            "clear" => handlers::elements::handle_clear_uc(&self.usecases.elements.clear, request),
            "select_all" => handlers::elements::handle_select_all_uc(
                &self.usecases.elements.select_all,
                request,
            ),
            "toggle" => {
                handlers::elements::handle_toggle_uc(&self.usecases.elements.toggle, request)
            }
            "select" => {
                handlers::elements::handle_select_uc(&self.usecases.elements.select, request)
            }
            "multiselect" => handlers::elements::handle_multiselect_uc(
                &self.usecases.elements.multiselect,
                request,
            ),

            "keystroke" => {
                handlers::input::handle_keystroke_uc(&self.usecases.input.keystroke, request)
            }
            "keydown" => handlers::input::handle_keydown_uc(&self.usecases.input.keydown, request),
            "keyup" => handlers::input::handle_keyup_uc(&self.usecases.input.keyup, request),
            "type" => handlers::input::handle_type_uc(&self.usecases.input.type_text, request),

            "wait" => handlers::wait::handle_wait_uc(&self.usecases.wait, request),

            "pty_read" => handlers::diagnostics::handle_pty_read_uc(
                &self.usecases.diagnostics.pty_read,
                request,
            ),
            "pty_write" => handlers::diagnostics::handle_pty_write_uc(
                &self.usecases.diagnostics.pty_write,
                request,
            ),
            "shutdown" => handlers::diagnostics::handle_shutdown_uc(
                &self.usecases.diagnostics.shutdown,
                request,
            ),

            _ => RpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::core::{Component, CursorPosition, Element};
    use crate::domain::{SessionId, SessionInfo};
    use crate::usecases::ports::{
        LivePreviewSnapshot, MetricsProvider, NoopShutdownNotifier, SessionError, SessionHandle,
        SessionOps, SessionRepository, StreamCursor, StreamRead, StreamSubscription,
    };
    use crossbeam_channel as channel;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::time::Instant;

    #[derive(Default)]
    struct TestMetrics;

    impl MetricsProvider for TestMetrics {
        fn requests(&self) -> u64 {
            0
        }

        fn errors(&self) -> u64 {
            0
        }

        fn lock_timeouts(&self) -> u64 {
            0
        }

        fn poison_recoveries(&self) -> u64 {
            0
        }
    }

    struct TestSession {
        id: SessionId,
    }

    impl SessionOps for TestSession {
        fn update(&self) -> Result<(), SessionError> {
            Ok(())
        }

        fn screen_text(&self) -> String {
            String::new()
        }

        fn screen_render(&self) -> String {
            String::new()
        }

        fn detect_elements(&self) -> Vec<Element> {
            Vec::new()
        }

        fn find_element(&self, _element_ref: &str) -> Option<Element> {
            None
        }

        fn pty_write(&self, _data: &[u8]) -> Result<(), SessionError> {
            Ok(())
        }

        fn pty_try_read(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize, SessionError> {
            Ok(0)
        }

        fn stream_read(
            &self,
            cursor: &mut StreamCursor,
            _max_bytes: usize,
            _timeout_ms: i32,
        ) -> Result<StreamRead, SessionError> {
            cursor.seq = cursor.seq.saturating_add(1);
            Ok(StreamRead {
                data: Vec::new(),
                next_cursor: *cursor,
                latest_cursor: *cursor,
                dropped_bytes: 0,
                closed: true,
            })
        }

        fn stream_subscribe(&self) -> StreamSubscription {
            let (_sender, receiver) = channel::unbounded();
            StreamSubscription::new(receiver)
        }

        fn analyze_screen(&self) -> Vec<Component> {
            Vec::new()
        }

        fn click(&self, _element_ref: &str) -> Result<(), SessionError> {
            Ok(())
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

        fn is_running(&self) -> bool {
            true
        }

        fn resize(&self, _cols: u16, _rows: u16) -> Result<(), SessionError> {
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
            "test".to_string()
        }

        fn size(&self) -> (u16, u16) {
            (80, 24)
        }

        fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
            LivePreviewSnapshot {
                cols: 80,
                rows: 24,
                seq: String::new(),
                stream_seq: 0,
            }
        }
    }

    #[derive(Default)]
    struct TestRepository {
        sessions: Vec<SessionInfo>,
        active: Option<SessionId>,
    }

    impl SessionRepository for TestRepository {
        fn spawn(
            &self,
            _command: &str,
            _args: &[String],
            _cwd: Option<&str>,
            _env: Option<&HashMap<String, String>>,
            session_id: Option<String>,
            _cols: u16,
            _rows: u16,
        ) -> Result<(SessionId, u32), SessionError> {
            let id = session_id.unwrap_or_else(|| "test-session".to_string());
            Ok((SessionId::new(id), 42))
        }

        fn get(&self, session_id: &str) -> Result<SessionHandle, SessionError> {
            Ok(Arc::new(TestSession {
                id: SessionId::new(session_id),
            }))
        }

        fn active(&self) -> Result<SessionHandle, SessionError> {
            let id = self
                .active
                .clone()
                .unwrap_or_else(|| SessionId::new("active"));
            Ok(Arc::new(TestSession { id }))
        }

        fn resolve(&self, session_id: Option<&str>) -> Result<SessionHandle, SessionError> {
            let id = session_id.unwrap_or("active");
            Ok(Arc::new(TestSession {
                id: SessionId::new(id),
            }))
        }

        fn set_active(&self, _session_id: &str) -> Result<(), SessionError> {
            Ok(())
        }

        fn list(&self) -> Vec<SessionInfo> {
            self.sessions.clone()
        }

        fn kill(&self, _session_id: &str) -> Result<(), SessionError> {
            Ok(())
        }

        fn session_count(&self) -> usize {
            self.sessions.len()
        }

        fn active_session_id(&self) -> Option<SessionId> {
            self.active.clone()
        }
    }

    fn create_test_usecases() -> UseCaseContainer<TestRepository> {
        let session_repo = Arc::new(TestRepository::default());
        let metrics = Arc::new(TestMetrics);
        let start_time = Instant::now();
        let active_connections = Arc::new(AtomicUsize::new(0));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_notifier = Arc::new(NoopShutdownNotifier);
        UseCaseContainer::new(
            session_repo,
            metrics,
            start_time,
            active_connections,
            shutdown_flag,
            shutdown_notifier,
        )
    }

    #[test]
    fn test_router_ping_returns_pong() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "ping".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert_eq!(parsed["result"]["pong"], true);
    }

    #[test]
    fn test_router_unknown_method_returns_error() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "nonexistent_method".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_some());
        assert_eq!(parsed["error"]["code"], -32601);
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("nonexistent_method")
        );
    }

    #[test]
    fn test_router_health_returns_success() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "health".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert!(parsed.get("result").is_some());
        assert_eq!(parsed["result"]["status"], "healthy");
    }

    #[test]
    fn test_router_sessions_returns_empty_list() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "sessions".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert!(parsed["result"]["sessions"].is_array());
    }

    #[test]
    fn test_router_cleanup_returns_success() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "cleanup".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert!(parsed.get("result").is_some());
        assert_eq!(parsed["result"]["sessions_cleaned"], 0);
        assert_eq!(parsed["result"]["sessions_failed"], 0);
        assert!(parsed["result"]["failures"].is_array());
    }

    #[test]
    fn test_router_assert_invalid_condition_returns_error() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(
            1,
            "assert".to_string(),
            Some(json!({ "condition": "invalid" })),
        );
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_some());
        assert_eq!(parsed["error"]["code"], -32602);
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Invalid condition format")
        );
    }

    #[test]
    fn test_router_assert_session_condition_not_found() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(
            1,
            "assert".to_string(),
            Some(json!({ "condition": "session:nonexistent" })),
        );
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert_eq!(parsed["result"]["passed"], false);
        assert_eq!(parsed["result"]["condition"], "session:nonexistent");
    }

    #[test]
    fn test_router_shutdown_returns_acknowledged() {
        let usecases = create_test_usecases();
        let router = Router::new(&usecases);

        let request = RpcRequest::new(1, "shutdown".to_string(), None);
        let response = router.route(request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert_eq!(parsed["result"]["acknowledged"], true);
    }
}
