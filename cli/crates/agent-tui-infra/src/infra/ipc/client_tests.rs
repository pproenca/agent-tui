use super::*;
use crate::common::mutex_lock_or_recover;
use crossbeam_channel as channel;
use std::io::ErrorKind;
use std::sync::Mutex;

#[test]
fn test_request_serializes_to_jsonrpc_2_0() {
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "version".to_string(),
        params: None,
    };
    let json = serde_json::to_string(&request).expect("request should serialize");
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"method\":\"version\""));
    assert!(!json.contains("\"params\""));
}

#[test]
fn test_request_serializes_with_params() {
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: 42,
        method: "spawn".to_string(),
        params: Some(serde_json::json!({"command": "bash", "cols": 80})),
    };
    let json = serde_json::to_string(&request).expect("request should serialize");
    assert!(json.contains("\"params\""));
    assert!(json.contains("\"command\":\"bash\""));
}

#[test]
fn test_response_deserializes_success_result() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
    let response: Response = serde_json::from_str(json).expect("response should parse");
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_response_deserializes_error() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let response: Response = serde_json::from_str(json).expect("response should parse");
    assert!(response.result.is_none());
    assert!(response.error.is_some());
    let error = response.error.expect("error payload should be present");
    assert_eq!(error.code, -32600);
}

#[test]
fn test_client_error_daemon_not_running_display() {
    let err = ClientError::DaemonNotRunning;
    assert_eq!(err.to_string(), "Daemon not running");
}

#[test]
fn test_client_error_invalid_response_display() {
    let err = ClientError::InvalidResponse;
    assert_eq!(err.to_string(), "Invalid response from daemon");
}

#[test]
fn test_client_error_rpc_error_display() {
    let err = ClientError::RpcError {
        code: -32601,
        message: "Method not found".to_string(),
        category: None,
        retryable: false,
        retry_delay_ms: None,
        context: None,
        suggestion: None,
    };
    assert_eq!(err.to_string(), "RPC error (-32601): Method not found");
}

#[test]
fn test_config_default_values() {
    let config = DaemonClientConfig::default();
    assert_eq!(config.read_timeout(), Duration::from_secs(60));
    assert_eq!(config.write_timeout(), Duration::from_secs(10));
    assert_eq!(config.max_retries(), 3);
    assert_eq!(config.initial_retry_delay(), Duration::from_millis(100));
}

#[test]
fn test_config_builder_pattern() {
    let config = DaemonClientConfig::default()
        .with_read_timeout(Duration::from_secs(30))
        .with_write_timeout(Duration::from_secs(5))
        .with_max_retries(5)
        .with_initial_retry_delay(Duration::from_millis(25));
    assert_eq!(config.read_timeout(), Duration::from_secs(30));
    assert_eq!(config.write_timeout(), Duration::from_secs(5));
    assert_eq!(config.max_retries(), 5);
    assert_eq!(config.initial_retry_delay(), Duration::from_millis(25));
}

#[test]
fn test_parse_daemon_lock_file_supports_legacy_pid_only_format() {
    let identity = parse_daemon_lock_file("1234").expect("legacy lock file should parse");
    assert_eq!(
        identity,
        ProcessIdentity {
            pid: 1234,
            started_at: None,
        }
    );
}

#[test]
fn test_parse_daemon_lock_file_supports_identity_payload() {
    let identity = parse_daemon_lock_file(r#"{"pid":1234,"process_started_at":42}"#)
        .expect("identity lock file should parse");
    assert_eq!(
        identity,
        ProcessIdentity {
            pid: 1234,
            started_at: Some(42),
        }
    );
}

#[test]
fn test_parse_daemon_lock_file_rejects_invalid_payload() {
    let err = parse_daemon_lock_file("not-a-pid").expect_err("invalid payload should fail");
    assert!(err.contains("not a valid daemon identity payload"));
}

static ENV_MUTEX: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

#[test]
fn test_ensure_daemon_starts_when_not_running() {
    let _guard = mutex_lock_or_recover(ENV_MUTEX.get_or_init(|| Mutex::new(())));

    let temp_dir = tempfile::Builder::new()
        .prefix("agent-tui-test-")
        .tempdir_in("/tmp")
        .expect("temp dir should be created");
    let socket_path = temp_dir.path().join("agent-tui.sock");
    let _ = std::fs::remove_file(&socket_path);

    // SAFETY: Test-only environment override to isolate socket path.
    unsafe {
        std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
    }
    crate::infra::ipc::transport::USE_DAEMON_START_STUB
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let result = ensure_daemon();
    match &result {
        Ok(_) => {
            assert!(UnixSocketClient::is_daemon_running());
        }
        Err(ClientError::ConnectionFailed(io_err))
            if io_err.kind() == ErrorKind::PermissionDenied =>
        {
            eprintln!("Skipping ensure_daemon test on restricted socket access: {io_err}");
        }
        Err(err) => {
            panic!(
                "ensure_daemon failed for socket {}: {}",
                socket_path.display(),
                err
            );
        }
    }
    crate::infra::ipc::transport::clear_test_listener();
    let _ = std::fs::remove_file(&socket_path);
    crate::infra::ipc::transport::USE_DAEMON_START_STUB
        .store(false, std::sync::atomic::Ordering::SeqCst);
    // SAFETY: Test-only cleanup of the environment override.
    unsafe {
        std::env::remove_var("AGENT_TUI_SOCKET");
    }
}

#[test]
fn test_in_memory_transport_round_trip() {
    let transport = std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new(
        |request| {
            let value: serde_json::Value =
                serde_json::from_str(request.trim()).expect("request json");
            let id = value
                .get("id")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(1));
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "ok": true }
            })
            .to_string()
        },
    ));

    let mut client = UnixSocketClient::connect_with_transport(transport)
        .expect("transport-backed client should connect");
    let result = client
        .call("version", None)
        .expect("transport-backed call should succeed");
    assert_eq!(result["ok"], true);
}

#[test]
fn test_call_with_config_retries_retryable_rpc_error() {
    let attempts = Arc::new(Mutex::new(0u32));
    let transport = std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new({
        let attempts = Arc::clone(&attempts);
        move |request| {
            let value: serde_json::Value =
                serde_json::from_str(request.trim()).expect("request json");
            let id = value
                .get("id")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(1));
            let mut count = mutex_lock_or_recover(&attempts);
            *count += 1;
            if *count == 1 {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32000,
                        "message": "busy",
                        "data": {
                            "retryable": true,
                            "retry_delay_ms": 0
                        }
                    }
                })
                .to_string()
            } else {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "ok": true }
                })
                .to_string()
            }
        }
    }));

    let mut client = UnixSocketClient::connect_with_transport(transport)
        .expect("transport-backed client should connect");
    let config = DaemonClientConfig::default()
        .with_max_retries(1)
        .with_initial_retry_delay(Duration::ZERO);

    let result = client
        .call_with_config("version", None, &config)
        .expect("retryable RPC error should be retried");

    assert_eq!(result["ok"], true);
    assert_eq!(*mutex_lock_or_recover(&attempts), 2);
}

#[test]
fn test_call_with_config_does_not_retry_non_retryable_rpc_error() {
    let attempts = Arc::new(Mutex::new(0u32));
    let transport = std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new({
        let attempts = Arc::clone(&attempts);
        move |request| {
            let value: serde_json::Value =
                serde_json::from_str(request.trim()).expect("request json");
            let id = value
                .get("id")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(1));
            let mut count = mutex_lock_or_recover(&attempts);
            *count += 1;
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32001,
                    "message": "fatal",
                    "data": {
                        "retryable": false
                    }
                }
            })
            .to_string()
        }
    }));

    let mut client = UnixSocketClient::connect_with_transport(transport)
        .expect("transport-backed client should connect");
    let config = DaemonClientConfig::default()
        .with_max_retries(3)
        .with_initial_retry_delay(Duration::ZERO);

    let err = client
        .call_with_config("version", None, &config)
        .expect_err("non-retryable RPC error should be returned");

    assert!(matches!(
        err,
        ClientError::RpcError {
            message,
            retry_delay_ms: None,
            ..
        } if message == "fatal"
    ));
    assert_eq!(*mutex_lock_or_recover(&attempts), 1);
}

#[test]
fn test_call_stream_with_config_retries_retryable_rpc_error() {
    let attempts = Arc::new(Mutex::new(0u32));
    let transport = std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new({
        let attempts = Arc::clone(&attempts);
        move |request| {
            let value: serde_json::Value =
                serde_json::from_str(request.trim()).expect("request json");
            let id = value
                .get("id")
                .cloned()
                .unwrap_or_else(|| serde_json::json!(1));
            let mut count = mutex_lock_or_recover(&attempts);
            *count += 1;
            if *count == 1 {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32000,
                        "message": "busy",
                        "data": {
                            "retryable": true,
                            "retry_delay_ms": 0
                        }
                    }
                })
                .to_string()
            } else {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "stream": "ready" }
                })
                .to_string()
            }
        }
    }));

    let mut client = UnixSocketClient::connect_with_transport(transport)
        .expect("transport-backed client should connect");
    let config = DaemonClientConfig::default()
        .with_max_retries(1)
        .with_initial_retry_delay(Duration::ZERO);

    let _stream = client
        .call_stream_with_config("live_preview_stream", None, &config)
        .expect("retryable stream handshake should be retried");

    assert_eq!(*mutex_lock_or_recover(&attempts), 2);
}

#[test]
fn test_client_error_to_json_includes_retry_delay() {
    let err = ClientError::RpcError {
        code: -32000,
        message: "busy".to_string(),
        category: None,
        retryable: true,
        retry_delay_ms: Some(250),
        context: None,
        suggestion: None,
    };

    let json = err.to_json();
    assert_eq!(json["retry_delay_ms"], 250);
}

#[test]
fn test_unix_socket_client_round_trip_over_real_socket() {
    use std::io::BufRead;
    use std::io::BufReader;
    use std::io::Write;
    use std::os::unix::net::UnixListener;

    let _guard = mutex_lock_or_recover(ENV_MUTEX.get_or_init(|| Mutex::new(())));
    let temp_dir = tempfile::Builder::new()
        .prefix("agent-tui-ipc-")
        .tempdir_in("/tmp")
        .expect("temp dir should be created");
    let socket_path = temp_dir.path().join("daemon.sock");

    let listener = UnixListener::bind(&socket_path).expect("listener should bind");
    // SAFETY: Test-only environment override to isolate the Unix socket path.
    unsafe {
        std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
    }

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("server should accept");
        let reader_stream = stream.try_clone().expect("server stream should clone");
        let mut reader = BufReader::new(reader_stream);
        let mut writer = stream;

        let mut line = String::new();
        reader.read_line(&mut line).expect("request should read");
        let request: serde_json::Value =
            serde_json::from_str(line.trim()).expect("request JSON should parse");
        let id = request
            .get("id")
            .cloned()
            .expect("request should include id");
        assert_eq!(request["method"], "version");

        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "ok": true }
        });
        writeln!(writer, "{response}").expect("response should write");
        writer.flush().expect("response should flush");
    });

    let mut client = UnixSocketClient {
        transport: std::sync::Arc::new(crate::infra::ipc::transport::UnixSocketTransport),
    };
    let result = client
        .call("version", None)
        .expect("real socket call should succeed");

    assert_eq!(result["ok"], true);
    server.join().expect("server thread should join");

    // SAFETY: Test-only cleanup of the Unix socket override.
    unsafe {
        std::env::remove_var("AGENT_TUI_SOCKET");
    }
}

#[test]
fn test_call_with_config_times_out_over_real_socket() {
    use std::io::BufRead;
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;

    let _guard = mutex_lock_or_recover(ENV_MUTEX.get_or_init(|| Mutex::new(())));
    let temp_dir = tempfile::Builder::new()
        .prefix("agent-tui-ipc-timeout-")
        .tempdir_in("/tmp")
        .expect("temp dir should be created");
    let socket_path = temp_dir.path().join("daemon.sock");

    let listener = UnixListener::bind(&socket_path).expect("listener should bind");
    // SAFETY: Test-only environment override to isolate the Unix socket path.
    unsafe {
        std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
    }
    let (release_tx, release_rx) = channel::bounded(1);

    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("server should accept");
        let reader_stream = stream.try_clone().expect("server stream should clone");
        let mut reader = BufReader::new(reader_stream);

        let mut line = String::new();
        reader.read_line(&mut line).expect("request should read");
        let _ = release_rx.recv();
        drop(stream);
    });

    let mut client = UnixSocketClient {
        transport: std::sync::Arc::new(crate::infra::ipc::transport::UnixSocketTransport),
    };
    let config = DaemonClientConfig::default()
        .with_read_timeout(Duration::from_millis(25))
        .with_write_timeout(Duration::from_millis(25))
        .with_max_retries(0);

    let err = client
        .call_with_config("version", None, &config)
        .expect_err("read timeout should be surfaced");

    assert!(matches!(
        err,
        ClientError::ConnectionFailed(io_err)
            if matches!(
                io_err.kind(),
                ErrorKind::TimedOut | ErrorKind::WouldBlock
            )
    ));
    release_tx
        .send(())
        .expect("server release signal should send");
    server.join().expect("server thread should join");

    // SAFETY: Test-only cleanup of the Unix socket override.
    unsafe {
        std::env::remove_var("AGENT_TUI_SOCKET");
    }
}
