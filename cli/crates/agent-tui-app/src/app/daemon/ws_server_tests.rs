use super::WsConfig;
use super::WsServerHandle;
use super::cancel_stream_task;
use crate::adapters::rpc::RpcRequest;
use crate::app::daemon::rpc_core::RpcCore;
use crate::app::daemon::rpc_core::RpcCoreTestConfig;
use crate::infra::daemon::DaemonConfig;
use crate::test_support::env_lock;
use crate::usecases::ports::ShutdownNotifierHandle;
use crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;
use base64::Engine;
use serde_json::Value;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tracing::Subscriber;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tungstenite::Message as WsMessage;
use tungstenite::client::IntoClientRequest;
use tungstenite::connect;
use tungstenite::handshake::client::Response;

static WS_TEST_SERVER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: test-only env mutation.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: test-only env mutation.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            // SAFETY: test-only env restoration.
            unsafe {
                std::env::set_var(self.key, prev);
            }
        } else {
            // SAFETY: test-only env cleanup.
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[derive(Clone, Default)]
struct EventRecorder {
    events: Arc<(std::sync::Mutex<Vec<String>>, std::sync::Condvar)>,
}

#[derive(Default)]
struct EventVisitor {
    fields: Vec<String>,
}

impl EventRecorder {
    fn captured(&self) -> Vec<String> {
        self.events
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn wait_for_substring(&self, needle: &str, timeout: Duration) -> Option<String> {
        let (lock, cv) = &*self.events;
        let guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = cv
            .wait_timeout_while(guard, timeout, |events| {
                !events.iter().any(|event| event.contains(needle))
            })
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        result
            .0
            .iter()
            .find(|event| event.contains(needle))
            .cloned()
    }
}

impl Visit for EventVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}

impl<S> Layer<S> for EventRecorder
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let (lock, cv) = &*self.events;
        lock.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(visitor.fields.join(" "));
        cv.notify_all();
    }
}

struct TestWsServer {
    _server_lock: std::sync::MutexGuard<'static, ()>,
    _tempdir: TempDir,
    core: Arc<RpcCore>,
    handle: Option<WsServerHandle>,
    ws_url: String,
    ui_url: String,
}

#[derive(Clone, Copy)]
struct TestWsServerOptions {
    stream_max_buffer_bytes: usize,
    live_preview_heartbeat: Duration,
}

impl Default for TestWsServerOptions {
    fn default() -> Self {
        Self {
            stream_max_buffer_bytes: 8 * 1024 * 1024,
            live_preview_heartbeat: Duration::from_secs(5),
        }
    }
}

impl TestWsServer {
    fn start() -> Self {
        Self::start_with_options(TestWsServerOptions::default())
    }

    fn start_with_options(options: TestWsServerOptions) -> Self {
        let server_lock = WS_TEST_SERVER_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tempdir = tempfile::tempdir().expect("tempdir");
        let state_path = tempdir.path().join("api.json");
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let notifier: ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
        let core = Arc::new(
            RpcCore::with_test_config(
                DaemonConfig::default(),
                Arc::clone(&shutdown_flag),
                notifier,
                RpcCoreTestConfig {
                    stream_max_buffer_bytes: options.stream_max_buffer_bytes,
                    live_preview_heartbeat: options.live_preview_heartbeat,
                    ..RpcCoreTestConfig::default()
                },
            )
            .expect("rpc core"),
        );
        let handle = super::start_ws_server(
            Arc::clone(&core),
            shutdown_flag,
            WsConfig {
                enabled: true,
                listen: "127.0.0.1:0".to_string(),
                allow_remote: false,
                state_path: state_path.clone(),
                max_connections: 4,
                ws_queue_capacity: 16,
            },
        )
        .expect("ws server");

        let contents = std::fs::read_to_string(&state_path).expect("ws state file");
        let state: Value = serde_json::from_str(&contents).expect("state json");
        let ws_url = state["ws_url"].as_str().expect("ws_url").to_string();
        let ui_url = state["ui_url"].as_str().expect("ui_url").to_string();

        Self {
            _server_lock: server_lock,
            _tempdir: tempdir,
            core,
            handle: Some(handle),
            ws_url,
            ui_url,
        }
    }

    fn ws_addr(&self) -> SocketAddr {
        let parsed = url::Url::parse(&self.ws_url).expect("ws url");
        let host = parsed.host_str().expect("host");
        let port = parsed.port_or_known_default().expect("port");
        format!("{host}:{port}").parse().expect("socket addr")
    }

    fn browser_origin(&self) -> String {
        let parsed = url::Url::parse(&self.ui_url).expect("ui url");
        let host = parsed.host_str().expect("host");
        let port = parsed.port_or_known_default().expect("port");
        format!("{}://{}:{}", parsed.scheme(), host, port)
    }

    fn ws_alias_url(&self, path: &str) -> String {
        let mut url = url::Url::parse(&self.ws_url).expect("ws url");
        url.set_path(path);
        url.to_string()
    }
}

impl Drop for TestWsServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.shutdown();
        }
    }
}

fn http_request(addr: SocketAddr, path: &str, extra_headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    for (name, value) in extra_headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).expect("write request");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read response");
    response
}

fn websocket_connect(
    url: &str,
    origin: Option<&str>,
) -> (
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    Response,
) {
    let mut request = url.into_client_request().expect("request");
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("Origin", origin.parse().expect("origin header"));
    }
    connect(request).expect("websocket connect")
}

fn set_websocket_read_timeout(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) {
    match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(stream) => {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout");
        }
        _ => panic!("test websocket should use a plain tcp stream"),
    }
}

fn assert_websocket_close_frame(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) {
    match socket.read() {
        Ok(WsMessage::Close(_)) => {}
        other => panic!("expected websocket close frame after JSON closed event, got {other:?}"),
    }
}

fn read_text_responding_to_pings(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) -> String {
    for _ in 0..16 {
        match socket.read().expect("read websocket frame") {
            WsMessage::Text(text) => return text,
            WsMessage::Ping(payload) => socket
                .send(WsMessage::Pong(payload))
                .expect("pong should send"),
            WsMessage::Pong(_) => {}
            WsMessage::Close(frame) => panic!("unexpected close frame: {frame:?}"),
            other => panic!("expected text or control frame, got {other:?}"),
        }
    }

    panic!("timed out waiting for text frame");
}

fn read_until_close_without_pong(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
) {
    let deadline = Instant::now()
        + super::WS_KEEPALIVE_INTERVAL
        + super::WS_PONG_TIMEOUT
        + Duration::from_secs(2);
    loop {
        match socket.read() {
            Ok(WsMessage::Close(_)) => return,
            Ok(WsMessage::Ping(_)) | Ok(WsMessage::Pong(_)) | Ok(WsMessage::Text(_)) => {}
            Ok(other) => panic!("unexpected websocket frame while waiting for close: {other:?}"),
            Err(tungstenite::Error::ConnectionClosed) | Err(tungstenite::Error::AlreadyClosed) => {
                return;
            }
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                if Instant::now() >= deadline {
                    panic!("timed out waiting for websocket close after missing pong");
                }
            }
            Err(err) => panic!("unexpected websocket error while waiting for close: {err}"),
        }
    }
}

#[test]
fn ws_config_reads_ws_env() {
    let _env = env_lock();
    let _listen = EnvGuard::set("AGENT_TUI_WS_LISTEN", "127.0.0.1:7777");

    let config = WsConfig::from_env();
    assert_eq!(config.listen, "127.0.0.1:7777");
}

#[test]
fn ws_config_ignores_deprecated_api_aliases() {
    let _env = env_lock();
    let _ws_listen = EnvGuard::remove("AGENT_TUI_WS_LISTEN");
    let _ws_state = EnvGuard::remove("AGENT_TUI_WS_STATE");
    let _api_listen = EnvGuard::set("AGENT_TUI_API_LISTEN", "127.0.0.1:9999");
    let _api_state = EnvGuard::set("AGENT_TUI_API_STATE", "/tmp/deprecated-state.json");

    let config = WsConfig::from_env();
    assert_eq!(config.listen, super::DEFAULT_WS_LISTEN);
    assert_eq!(config.state_path, super::default_state_path());
}

#[test]
fn bind_listener_rejects_non_loopback_without_allow_remote() {
    let config = WsConfig {
        enabled: true,
        listen: "0.0.0.0:0".to_string(),
        allow_remote: false,
        state_path: std::path::PathBuf::from("/tmp/agent-tui-ws-test-state.json"),
        max_connections: 1,
        ws_queue_capacity: 1,
    };

    let err = super::bind_listener(&config).expect_err("expected non-loopback bind rejection");
    let message = err.to_string();
    assert!(message.contains("AGENT_TUI_WS_ALLOW_REMOTE=1"), "{message}");
}

#[test]
fn format_ws_url_embeds_auth_token() {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().expect("valid addr");
    let url = super::format_ws_url(&addr, "secret-token");
    assert_eq!(url, "ws://127.0.0.1:12345/ws?token=secret-token");
}

#[test]
fn format_ui_url_embeds_encoded_ws_url() {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().expect("valid addr");
    let ws_url = "ws://127.0.0.1:12345/ws?token=secret-token";
    let ui_url = super::format_ui_url(&addr, ws_url);
    assert!(
        ui_url.contains("ws=ws%3A%2F%2F127.0.0.1%3A12345%2Fws%3Ftoken%3Dsecret-token"),
        "{ui_url}"
    );
}

#[test]
fn ws_server_startup_log_uses_current_dispatch_and_stays_token_free() {
    let recorder = EventRecorder::default();
    let subscriber = tracing_subscriber::registry().with(recorder.clone());

    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        let server = TestWsServer::start();
        let output = recorder
            .wait_for_substring("message=WS server listening", Duration::from_secs(10))
            .unwrap_or_else(|| {
                panic!(
                    "timed out waiting for ws startup log: {}",
                    recorder.captured().join("\n")
                )
            });
        assert!(output.contains("ui_path=\"/ui\""), "{output}");
        assert!(output.contains("ws_path=\"/ws\""), "{output}");
        assert!(!output.contains("token="), "{output}");
        assert!(!output.contains(&server.ws_url), "{output}");
        tracing::callsite::rebuild_interest_cache();
    });
}

#[test]
fn ui_root_redirects_to_tokenized_ui_url() {
    let server = TestWsServer::start();
    let response = http_request(server.ws_addr(), "/", &[]);
    let encoded_ws = super::encode_url_query_value(&server.ws_url);

    assert!(
        response.starts_with("HTTP/1.1 307 Temporary Redirect"),
        "{response}"
    );
    assert!(
        response.contains(&format!("location: /ui?ws={encoded_ws}\r\n"))
            || response.contains(&format!("Location: /ui?ws={encoded_ws}\r\n")),
        "{response}"
    );
}

#[test]
fn ws_upgrade_rejects_missing_token() {
    let server = TestWsServer::start();
    let mut url = url::Url::parse(&server.ws_url).expect("ws url");
    url.set_query(None);

    let err = tungstenite::connect(url.as_str())
        .expect_err("missing token should reject the websocket upgrade");
    let response = match err {
        tungstenite::Error::Http(response) => response,
        other => panic!("expected http error, got {other}"),
    };
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[test]
fn ws_upgrade_rejects_cross_origin_browser_requests() {
    let server = TestWsServer::start();
    let err = {
        let mut request = server
            .ws_url
            .clone()
            .into_client_request()
            .expect("request");
        request
            .headers_mut()
            .insert("Origin", "https://example.com".parse().expect("origin"));
        connect(request).expect_err("cross-origin browser request should be rejected")
    };
    let response = match err {
        tungstenite::Error::Http(response) => response,
        other => panic!("expected http error, got {other}"),
    };
    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[test]
fn ws_idle_connection_receives_server_keepalive_ping() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);

    match socket.read().expect("read keepalive frame") {
        WsMessage::Ping(_) => {}
        other => panic!("expected server keepalive ping, got {other:?}"),
    }
}

#[test]
fn ws_missing_pong_disconnects_idle_connection() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);

    std::thread::park_timeout(
        super::WS_KEEPALIVE_INTERVAL + super::WS_PONG_TIMEOUT + Duration::from_millis(50),
    );

    read_until_close_without_pong(&mut socket);
}

#[test]
fn ws_pong_keeps_connection_alive() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);

    for _ in 0..3 {
        match socket.read().expect("read keepalive frame") {
            WsMessage::Ping(payload) => socket
                .send(WsMessage::Pong(payload))
                .expect("pong should send"),
            other => panic!("expected keepalive ping, got {other:?}"),
        }
    }

    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "after-pong",
                "method": "ping",
            })
            .to_string(),
        ))
        .expect("send ping request");

    let payload: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("json");
    assert_eq!(payload["id"], "after-pong");
    assert_eq!(payload["result"]["pong"], true);
}

#[test]
fn ws_rpc_echoes_string_request_id() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);

    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "req-1",
                "method": "ping",
            })
            .to_string(),
        ))
        .expect("send ping request");

    for _ in 0..8 {
        match socket.read().expect("read rpc response") {
            WsMessage::Text(text) => {
                let payload: Value = serde_json::from_str(&text).expect("json");
                assert_eq!(payload["id"], "req-1");
                assert_eq!(payload["result"]["pong"], true);
                return;
            }
            WsMessage::Ping(payload) => socket
                .send(WsMessage::Pong(payload))
                .expect("pong should send"),
            WsMessage::Pong(_) => {}
            other => panic!("expected text response or keepalive frame, got {other:?}"),
        }
    }

    panic!("timed out waiting for string-id ping response");
}

#[test]
fn ws_missing_pong_disconnects_stream_connection() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(
        &server.ws_alias_url("/api/v1/stream"),
        Some(&browser_origin),
    );
    set_websocket_read_timeout(&mut socket);

    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 77,
                "method": "flightdeck_stream",
                "params": {
                    "interval_ms": 1000,
                }
            })
            .to_string(),
        ))
        .expect("send flightdeck request");

    std::thread::park_timeout(
        super::WS_KEEPALIVE_INTERVAL + super::WS_PONG_TIMEOUT + Duration::from_millis(50),
    );

    read_until_close_without_pong(&mut socket);
}

#[test]
fn api_stream_alias_accepts_authenticated_flightdeck_stream() {
    let server = TestWsServer::start();
    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(
        &server.ws_alias_url("/api/v1/stream"),
        Some(&browser_origin),
    );
    set_websocket_read_timeout(&mut socket);
    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "flightdeck_stream",
                "params": {
                    "interval_ms": 1000,
                }
            })
            .to_string(),
        ))
        .expect("send flightdeck request");

    let payload: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("json");
    assert_eq!(payload["id"], 7);
    assert_eq!(payload["result"]["event"], "ready");
    assert!(payload["result"]["sessions"].is_array());
}

#[test]
fn live_preview_stream_over_ws_emits_ready_init_output_and_closed() {
    let server = TestWsServer::start();
    let spawn_response = server.core.route(RpcRequest::new(
        1,
        "spawn".to_string(),
        Some(serde_json::json!({
            "command": "sh",
            "args": [],
            "session": "ws-test-session",
        })),
    ));
    let spawn_payload = serde_json::to_value(spawn_response).expect("spawn response value");
    let session_id = spawn_payload["result"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string();

    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);
    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "live_preview_stream",
                "params": {
                    "session": session_id,
                }
            })
            .to_string(),
        ))
        .expect("send live preview request");

    let ready: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("ready json");
    assert_eq!(ready["result"]["event"], "ready");

    let init: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("init json");
    assert_eq!(init["result"]["event"], "init");

    let write_response = server.core.route(RpcRequest::new(
        2,
        "pty_write".to_string(),
        Some(serde_json::json!({
            "session": session_id,
            "data": base64::engine::general_purpose::STANDARD.encode("printf hello\nexit\n"),
        })),
    ));
    let write_payload = serde_json::to_value(write_response).expect("write response value");
    assert_eq!(write_payload["result"]["success"], true);

    let mut saw_output = false;
    let mut saw_closed = false;
    for _ in 0..64 {
        if saw_closed {
            break;
        }
        let frame = socket.read().expect("stream frame");
        let text = match frame {
            WsMessage::Text(text) => text,
            WsMessage::Ping(payload) => {
                socket
                    .send(WsMessage::Pong(payload))
                    .expect("pong should send");
                continue;
            }
            WsMessage::Pong(_) => continue,
            WsMessage::Close(_) => {
                break;
            }
            other => panic!("expected text or close stream frame, got {other:?}"),
        };
        let payload: Value = serde_json::from_str(&text).expect("json");
        match payload["result"]["event"].as_str() {
            Some("output") => {
                let data = payload["result"]["data_b64"]
                    .as_str()
                    .expect("output data_b64");
                let decoded = String::from_utf8(
                    base64::engine::general_purpose::STANDARD
                        .decode(data)
                        .expect("decode output"),
                )
                .expect("utf8 output");
                if decoded.contains("hello") {
                    saw_output = true;
                }
            }
            Some("closed") => {
                saw_closed = true;
            }
            Some("command") => panic!("live preview must not emit command events"),
            _ => {}
        }
    }

    assert!(
        saw_output,
        "expected websocket output frame containing shell output"
    );
    assert!(
        saw_closed,
        "expected websocket JSON closed event after shell exit"
    );
    assert_websocket_close_frame(&mut socket);
}

#[test]
fn live_preview_stream_over_ws_emits_heartbeat_when_idle() {
    let server = TestWsServer::start_with_options(TestWsServerOptions {
        live_preview_heartbeat: Duration::from_millis(100),
        ..TestWsServerOptions::default()
    });
    let spawn_response = server.core.route(RpcRequest::new(
        1,
        "spawn".to_string(),
        Some(serde_json::json!({
            "command": "sh",
            "args": [],
            "session": "ws-heartbeat-session",
        })),
    ));
    let spawn_payload = serde_json::to_value(spawn_response).expect("spawn response value");
    let session_id = spawn_payload["result"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string();

    let browser_origin = server.browser_origin();
    let (mut socket, _response) = websocket_connect(&server.ws_url, Some(&browser_origin));
    set_websocket_read_timeout(&mut socket);
    socket
        .send(WsMessage::Text(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "live_preview_stream",
                "params": {
                    "session": session_id,
                }
            })
            .to_string(),
        ))
        .expect("send live preview request");

    let ready: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("ready json");
    assert_eq!(ready["result"]["event"], "ready");

    let init: Value =
        serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("init json");
    assert_eq!(init["result"]["event"], "init");

    let heartbeat = loop {
        let payload: Value =
            serde_json::from_str(&read_text_responding_to_pings(&mut socket)).expect("stream json");
        if payload["result"]["event"] == "heartbeat" {
            break payload;
        }
    };
    assert!(
        heartbeat["result"]["time"].as_f64().unwrap_or_default() >= 0.0,
        "heartbeat should carry a non-negative relative timestamp"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_stream_task_drops_receiver_to_release_backpressure() {
    let (tx, rx) = mpsc::channel::<String>(1);
    tx.send("first".to_string()).await.expect("seed channel");

    let mut stream_task = tokio::task::spawn_blocking(move || {
        let _ = tx.blocking_send("second".to_string());
    });

    let stream_cancelled = Arc::new(AtomicBool::new(false));
    let mut rx = Some(rx);
    let _ = cancel_stream_task(&stream_cancelled, &mut rx, &mut stream_task).await;

    assert!(stream_cancelled.load(Ordering::Relaxed));
    assert!(
        rx.is_none(),
        "receiver should be dropped during cancellation"
    );
    assert!(stream_task.is_finished(), "stream task should be finished");
}

#[test]
fn ws_handle_shutdown_remains_bounded_on_slow_thread() {
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = std::sync::mpsc::sync_channel(1);
    let join = thread::spawn(move || {
        let _ = release_rx.recv();
        let _ = finished_tx.send(());
    });
    let handle = WsServerHandle {
        shutdown_tx: None,
        join: Some(join),
        state_path: PathBuf::new(),
    };

    handle.shutdown();
    assert!(
        finished_rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "shutdown should return before the slow ws thread exits"
    );
    release_tx
        .send(())
        .expect("slow ws thread should still be waiting");
    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "slow ws thread should finish after release"
    );
}

#[test]
fn ws_handle_shutdown_keeps_state_file_until_thread_exits() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let state_path = tempdir.path().join("api.json");
    std::fs::write(&state_path, b"state").expect("seed state file");

    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let join = thread::spawn(move || {
        let _ = release_rx.recv();
    });
    let handle = WsServerHandle {
        shutdown_tx: None,
        join: Some(join),
        state_path: state_path.clone(),
    };

    handle.shutdown();

    assert!(
        state_path.exists(),
        "state file should remain while the ws thread is still owned by the background reaper"
    );
    release_tx
        .send(())
        .expect("slow ws thread should still be waiting");
}
