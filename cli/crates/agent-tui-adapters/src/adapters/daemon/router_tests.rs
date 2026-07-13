use super::*;
use crate::domain::SessionId;
use crate::domain::SessionInfo;
use crate::domain::TerminalSize;
use crate::domain::core::CursorPosition;
use crate::usecases::ports::Clock;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionHandle;
use crate::usecases::ports::SessionOps;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiter;
use crate::usecases::ports::StreamWaiterHandle;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::time::Instant;

#[derive(Default)]
struct TestClock;

impl Clock for TestClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct TestSession {
    id: SessionId,
}

struct TestStreamWaiter;

impl StreamWaiter for TestStreamWaiter {
    fn wait(&self, _timeout: Option<Duration>) -> bool {
        true
    }
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

    fn screen_render_compact(&self) -> String {
        String::new()
    }

    fn terminal_write(&self, _data: &[u8]) -> Result<(), SessionError> {
        Ok(())
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

    fn stream_subscribe(&self) -> StreamWaiterHandle {
        Arc::new(TestStreamWaiter)
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

    fn size(&self) -> TerminalSize {
        TerminalSize::default()
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
        session_id: Option<SessionId>,
        _size: TerminalSize,
    ) -> Result<(SessionId, u32), SessionError> {
        let id = session_id.unwrap_or_else(|| {
            SessionId::try_new("test-session").expect("default test session id should be valid")
        });
        Ok((id, 42))
    }

    fn resolve(&self, session_id: Option<&SessionId>) -> Result<SessionHandle, SessionError> {
        let id = session_id
            .cloned()
            .unwrap_or_else(|| SessionId::try_new("active").expect("active id should be valid"));
        Ok(Arc::new(TestSession { id }))
    }

    fn set_active(&self, _session_id: &SessionId) -> Result<(), SessionError> {
        Ok(())
    }

    fn list(&self) -> Vec<SessionInfo> {
        self.sessions.clone()
    }

    fn kill(&self, _session_id: &SessionId) -> Result<(), SessionError> {
        Ok(())
    }

    fn restart(
        &self,
        session_id: Option<&SessionId>,
    ) -> Result<crate::domain::RestartOutput, SessionError> {
        let old_session_id = session_id
            .cloned()
            .unwrap_or_else(|| SessionId::try_new("active").expect("active id should be valid"));
        Ok(crate::domain::RestartOutput {
            old_session_id,
            new_session_id: SessionId::try_new("restarted").expect("restart id should be valid"),
            command: "bash".to_string(),
            pid: 42,
        })
    }

    fn active_session_id(&self) -> Option<SessionId> {
        self.active.clone()
    }
}

fn route_test_request(request: RpcRequest) -> RpcResponse {
    let repository = TestRepository::default();
    let clock = TestClock;
    let shutdown_flag = AtomicBool::new(false);
    let shutdown_notifier = crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;
    Router::new(&repository, &clock, &shutdown_flag, &shutdown_notifier).route(request)
}

#[test]
fn test_router_ping_returns_pong() {
    let request = RpcRequest::new(1, "ping".to_string(), None);
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response).expect("ping response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("ping response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert_eq!(parsed["result"]["pong"], true);
}

#[test]
fn test_router_unknown_method_returns_error() {
    let request = RpcRequest::new(1, "nonexistent_method".to_string(), None);
    let response = route_test_request(request);

    let json_str =
        serde_json::to_string(&response).expect("unknown-method response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("unknown-method response should parse");

    assert!(parsed.get("error").is_some());
    assert_eq!(parsed["error"]["code"], -32601);
    assert!(matches!(
        parsed["error"]["message"].as_str(),
        Some(message) if message.contains("nonexistent_method")
    ));
}

#[test]
fn test_router_version_returns_success() {
    let request = RpcRequest::new(1, "version".to_string(), None);
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response).expect("version response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("version response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert!(parsed.get("result").is_some());
    assert!(parsed["result"]["daemon_version"].is_string());
}

#[test]
fn test_router_sessions_returns_empty_list() {
    let request = RpcRequest::new(1, "sessions".to_string(), None);
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response).expect("sessions response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("sessions response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert!(parsed["result"]["sessions"].is_array());
}

#[test]
fn test_router_cleanup_returns_success() {
    let request = RpcRequest::new(1, "cleanup".to_string(), None);
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response).expect("cleanup response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("cleanup response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert!(parsed.get("result").is_some());
    assert_eq!(parsed["result"]["cleaned"], 0);
    assert!(parsed["result"]["failures"].is_array());
}

#[test]
fn test_router_assert_invalid_condition_returns_error() {
    let request = RpcRequest::new(
        1,
        "assert".to_string(),
        Some(json!({ "type": "invalid", "value": "nope" })),
    );
    let response = route_test_request(request);

    let json_str =
        serde_json::to_string(&response).expect("assert error response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("assert error response should parse");

    assert!(parsed.get("error").is_some());
    assert_eq!(parsed["error"]["code"], -32602);
    assert!(matches!(
        parsed["error"]["message"].as_str(),
        Some(message) if message.contains("Invalid type")
    ));
}

#[test]
fn test_router_assert_session_condition_not_found() {
    let request = RpcRequest::new(
        1,
        "assert".to_string(),
        Some(json!({ "type": "session", "value": "nonexistent" })),
    );
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response)
        .expect("assert session condition response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("assert session condition response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert_eq!(parsed["result"]["passed"], false);
    assert_eq!(parsed["result"]["condition"], "session:nonexistent");
}

#[test]
fn test_router_shutdown_returns_null_success() {
    let request = RpcRequest::new(1, "shutdown".to_string(), None);
    let response = route_test_request(request);

    let json_str = serde_json::to_string(&response).expect("shutdown response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("shutdown response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert!(parsed["result"].is_null());
}
