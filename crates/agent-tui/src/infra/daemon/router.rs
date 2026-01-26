use crate::infra::ipc::{RpcRequest, RpcResponse};
use serde_json::json;

use super::usecase_container::UseCaseContainer;
use crate::infra::daemon::handlers;
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
            "live_preview_start" => handlers::live_preview::handle_live_preview_start(
                &self.usecases.live_preview.start,
                request,
            ),
            "live_preview_stop" => handlers::live_preview::handle_live_preview_stop(
                &self.usecases.live_preview.stop,
                request,
            ),
            "live_preview_status" => handlers::live_preview::handle_live_preview_status(
                &self.usecases.live_preview.status,
                request,
            ),

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
    use crate::infra::daemon::DaemonMetrics;
    use crate::infra::daemon::SessionManager;
    use crate::usecases::ports::SessionRepository;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::time::Instant;

    fn create_test_usecases() -> UseCaseContainer<SessionManager> {
        let session_manager = Arc::new(SessionManager::new());
        let metrics = Arc::new(DaemonMetrics::new());
        let session_repo: Arc<dyn SessionRepository> = session_manager.clone();
        let live_preview = Arc::new(crate::infra::daemon::LivePreviewManager::new(session_repo));
        let start_time = Instant::now();
        let active_connections = Arc::new(AtomicUsize::new(0));
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        UseCaseContainer::new(
            session_manager,
            metrics,
            start_time,
            active_connections,
            shutdown_flag,
            live_preview,
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
