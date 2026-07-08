#![allow(
    clippy::expect_used,
    reason = "Test-only assertions use expect for clarity."
)]

use super::*;
use crate::common::mutex_lock_or_recover;
use crate::test_support::env_lock;
use crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;
use std::io::BufRead as _;
use std::sync::mpsc;
use tempfile::tempdir;
use tracing::Subscriber;
use tracing::field::Visit;
use tracing::span::Attributes;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

#[derive(Clone, Default)]
struct SpanFieldRecorder {
    fields: Arc<std::sync::Mutex<Vec<String>>>,
}

#[derive(Default)]
struct FieldVisitor {
    fields: Vec<String>,
}

impl SpanFieldRecorder {
    fn captured(&self) -> Vec<String> {
        self.fields
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}

impl<S> Layer<S> for SpanFieldRecorder
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        if attrs.metadata().name() != "rpc_request" {
            return;
        }

        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        self.fields
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(visitor.fields.join(" "));
    }
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: Test-only environment override.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = self.prev.take() {
            // SAFETY: Test-only environment restoration.
            unsafe {
                std::env::set_var(self.key, prev);
            }
        } else {
            // SAFETY: Test-only environment cleanup.
            unsafe {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[test]
fn shutdown_connections_closes_idle_client() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
    let server = Arc::new(
        DaemonServer::with_config(DaemonConfig::default(), Arc::clone(&shutdown), notifier)
            .expect("daemon server should initialize"),
    );

    let (client, server_stream) = UnixStream::pair().expect("failed to create unix pair");
    let conn = UnixSocketConnection::new(server_stream).expect("failed to wrap connection");

    let (tx, rx) = mpsc::sync_channel(1);
    let server_clone = Arc::clone(&server);
    std::thread::spawn(move || {
        server_clone.handle_client(conn);
        let _ = tx.send(());
    });

    assert!(
        server.wait_for_registered_connection(Duration::from_secs(1)),
        "connection was not registered"
    );

    server.shutdown_connections();

    assert!(
        rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "client handler did not exit after shutdown"
    );

    drop(client);
}

#[test]
fn join_stream_threads_drains_handles() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
    let server = Arc::new(
        DaemonServer::with_config(DaemonConfig::default(), Arc::clone(&shutdown), notifier)
            .expect("daemon server should initialize"),
    );

    let handle = std::thread::spawn(|| {});
    server.register_stream_thread(handle);
    server.join_stream_threads(Duration::from_secs(1));

    assert!(mutex_lock_or_recover(&server.stream_threads).is_empty());
}

#[test]
fn join_stream_threads_times_out_without_blocking_shutdown() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
    let server = Arc::new(
        DaemonServer::with_config(DaemonConfig::default(), Arc::clone(&shutdown), notifier)
            .expect("daemon server should initialize"),
    );

    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let handle = std::thread::spawn(move || {
        let _ = release_rx.recv();
        let _ = finished_tx.send(());
    });
    server.register_stream_thread(handle);

    server.join_stream_threads(Duration::from_millis(25));
    assert!(
        finished_rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "join should return before the slow stream thread exits"
    );
    assert!(mutex_lock_or_recover(&server.stream_threads).is_empty());
    release_tx
        .send(())
        .expect("slow stream thread should still be waiting");
    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "slow stream thread should finish after release"
    );
}

#[test]
fn join_thread_with_timeout_or_reap_spawns_background_reaper_on_timeout() {
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let handle = std::thread::spawn(move || {
        let _ = release_rx.recv();
        let _ = finished_tx.send(());
    });

    let outcome = crate::common::join_thread_with_timeout_or_reap(
        handle,
        Duration::from_millis(10),
        "test thread",
        "test-thread-reaper",
    );

    assert_eq!(
        outcome,
        crate::common::ThreadJoinOutcome::ReapingInBackground
    );
    assert!(
        finished_rx.recv_timeout(Duration::from_millis(50)).is_err(),
        "reaper handoff should return before the slow thread exits"
    );
    release_tx
        .send(())
        .expect("slow thread should still be waiting");
    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "slow thread should finish after release"
    );
}

#[test]
fn with_config_surfaces_session_manager_startup_failure() {
    let _env_lock = env_lock();
    let _temp_home = tempdir().expect("temp dir should be created");
    let _store_guard = EnvGuard::set("AGENT_TUI_SESSION_STORE", "/dev/null/session-store.jsonl");

    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
    let err = match DaemonServer::with_config(DaemonConfig::default(), shutdown, notifier) {
        Ok(_) => {
            panic!("daemon server startup should surface session manager initialization failures")
        }
        Err(err) => err,
    };

    assert!(matches!(
        err,
        crate::infra::daemon::SessionError::Persistence { .. }
    ));
}

#[test]
fn session_selector_for_log_redacts_explicit_ids() {
    let explicit = crate::adapters::rpc::RpcRequest::new(
        1,
        "version".to_string(),
        Some(serde_json::json!({ "session": "session-123" })),
    );
    let active = crate::adapters::rpc::RpcRequest::new(
        2,
        "version".to_string(),
        Some(serde_json::json!({ "session": "active" })),
    );
    let blank = crate::adapters::rpc::RpcRequest::new(
        3,
        "version".to_string(),
        Some(serde_json::json!({ "session": "   " })),
    );
    let implicit = crate::adapters::rpc::RpcRequest::new(4, "version".to_string(), None);

    assert_eq!(super::session_selector_for_log(&explicit), "explicit");
    assert_eq!(super::session_selector_for_log(&active), "explicit-active");
    assert_eq!(super::session_selector_for_log(&blank), "blank");
    assert_eq!(
        super::session_selector_for_log(&implicit),
        "implicit-active"
    );
}

#[test]
fn handle_client_logs_redacted_session_selector() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle = Arc::new(NoopShutdownNotifier);
    let server = Arc::new(
        DaemonServer::with_config(DaemonConfig::default(), Arc::clone(&shutdown), notifier)
            .expect("daemon server should initialize"),
    );
    let (mut client, server_stream) = UnixStream::pair().expect("failed to create unix pair");
    let conn = UnixSocketConnection::new(server_stream).expect("failed to wrap connection");
    let recorder = SpanFieldRecorder::default();
    let subscriber = tracing_subscriber::registry().with(recorder.clone());

    let (tx, rx) = mpsc::sync_channel(1);
    let server_clone = Arc::clone(&server);
    std::thread::spawn(move || {
        tracing::subscriber::with_default(subscriber, || {
            server_clone.handle_client(conn);
        });
        let _ = tx.send(());
    });

    writeln!(
        client,
        "{}",
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "version",
            "params": {
                "session": "session-123"
            }
        })
    )
    .expect("request should write");
    std::io::Write::flush(&mut client).expect("request should flush");

    let mut response = String::new();
    let mut reader = std::io::BufReader::new(client);
    reader
        .read_line(&mut response)
        .expect("response should read");
    client = reader.into_inner();
    drop(client);

    assert!(
        rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "client handler should exit after client close"
    );

    let captured = recorder.captured().join("\n");
    assert!(captured.contains("session_selector=\"explicit\""));
    assert!(!captured.contains("session-123"));
}
