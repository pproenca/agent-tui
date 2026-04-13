use super::*;
use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

fn make_request(params_json: Option<&str>) -> RpcRequest {
    let request_json = match params_json {
        Some(params) => {
            format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"live_preview_stream","params":{params}}}"#
            )
        }
        None => r#"{"jsonrpc":"2.0","id":1,"method":"live_preview_stream"}"#.to_string(),
    };
    serde_json::from_str(&request_json).expect("valid rpc request")
}

fn make_flightdeck_request(params_json: Option<&str>) -> RpcRequest {
    let request_json = match params_json {
        Some(params) => {
            format!(r#"{{"jsonrpc":"2.0","id":7,"method":"flightdeck_stream","params":{params}}}"#)
        }
        None => r#"{"jsonrpc":"2.0","id":7,"method":"flightdeck_stream"}"#.to_string(),
    };
    serde_json::from_str(&request_json).expect("valid rpc request")
}

#[derive(Clone, Default)]
struct RecordingWriterHandle {
    values: Arc<(Mutex<Vec<Value>>, std::sync::Condvar)>,
}

impl RecordingWriterHandle {
    fn snapshot(&self) -> Vec<Value> {
        self.values
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn wait_for_event(&self, event: &str, timeout: Duration) -> Option<Value> {
        let (lock, cv) = &*self.values;
        let guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = cv
            .wait_timeout_while(guard, timeout, |values| {
                !values.iter().any(|value| value["result"]["event"] == event)
            })
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        result
            .0
            .iter()
            .find(|value| value["result"]["event"] == event)
            .cloned()
    }
}

struct RecordingWriter {
    handle: RecordingWriterHandle,
}

impl RecordingWriter {
    fn new() -> (Self, RecordingWriterHandle) {
        let handle = RecordingWriterHandle::default();
        (
            Self {
                handle: handle.clone(),
            },
            handle,
        )
    }
}

impl RpcResponseWriter for RecordingWriter {
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError> {
        let value = serde_json::to_value(response)
            .map_err(|err| RpcCoreError::Other(format!("serialize response: {err}")))?;
        let (lock, cv) = &*self.handle.values;
        lock.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(value);
        cv.notify_all();
        Ok(())
    }
}

#[test]
fn live_preview_selector_maps_active_to_none() {
    let request = make_request(Some(r#"{"session":"active"}"#));
    let parsed = parse_live_preview_session_selector(&request).expect("active selector");
    assert!(parsed.is_none());
}

#[test]
fn live_preview_selector_defaults_to_none_when_omitted() {
    let request = make_request(None);
    let parsed = parse_live_preview_session_selector(&request).expect("omitted selector");
    assert!(parsed.is_none());
}

#[test]
fn live_preview_selector_keeps_explicit_session_id() {
    let request = make_request(Some(r#"{"session":"sess-1"}"#));
    let parsed = parse_live_preview_session_selector(&request)
        .expect("valid selector")
        .expect("session id");
    assert_eq!(parsed.as_str(), "sess-1");
}

#[test]
fn live_preview_selector_rejects_blank_explicit_session_id() {
    let request = make_request(Some(r#"{"session":" "}"#));
    let response =
        parse_live_preview_session_selector(&request).expect_err("blank selector should fail");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn live_preview_initial_cursor_uses_snapshot_stream_seq() {
    let snapshot = crate::usecases::ports::LivePreviewSnapshot {
        cols: 80,
        rows: 24,
        seq: String::new(),
        stream_seq: 123,
    };
    let cursor = live_preview_initial_cursor(&snapshot);
    assert_eq!(cursor.seq, 123);
}

struct NotifyingWaiter {
    entered: std::sync::mpsc::SyncSender<()>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl crate::usecases::ports::StreamWaiter for NotifyingWaiter {
    fn wait(&self, timeout: Option<Duration>) -> bool {
        let _ = self.entered.try_send(());
        if timeout.is_some() {
            let _ = self
                .release
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .recv();
        }
        false
    }
}

#[test]
fn stream_wait_exits_early_when_connection_is_cancelled() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = RpcCore::with_test_config(
        crate::infra::daemon::DaemonConfig::default(),
        shutdown,
        notifier,
        RpcCoreTestConfig {
            wait_slice: Duration::from_millis(10),
            ..RpcCoreTestConfig::default()
        },
    )
    .expect("rpc core should initialize");

    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
    let subscription: crate::usecases::ports::StreamWaiterHandle = Arc::new(NotifyingWaiter {
        entered: entered_tx,
        release: std::sync::Mutex::new(release_rx),
    });
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_thread = Arc::clone(&cancelled);
    let join = std::thread::spawn(move || {
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("waiter should block before cancellation");
        cancelled_for_thread.store(true, Ordering::Relaxed);
        release_tx
            .send(())
            .expect("waiter release signal should be delivered");
    });

    let status = core.wait_for_stream_event_or_tick(
        &subscription,
        Duration::from_secs(30),
        Some(cancelled.as_ref()),
    );
    let _ = join.join();

    assert_eq!(status, StreamWaitStatus::Terminated);
}

#[test]
fn stream_kind_recognizes_flightdeck_stream() {
    assert_eq!(
        RpcCore::stream_kind_for_method("flightdeck_stream"),
        Some(StreamKind::Flightdeck)
    );
}

#[cfg(unix)]
#[test]
fn live_preview_stream_does_not_emit_command_events_for_session_input() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = Arc::new(
        RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        )
        .expect("rpc core should initialize"),
    );

    let spawn_result = core.session_manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(
            crate::domain::SessionId::try_new("timeline-session")
                .expect("timeline session id should be valid"),
        ),
        TerminalSize::default(),
    );
    if spawn_result.is_err() {
        return;
    }

    let (mut writer, handle) = RecordingWriter::new();
    let cancelled = Arc::new(AtomicBool::new(false));
    let request = make_request(Some(r#"{"session":"timeline-session"}"#));

    let core_for_stream = Arc::clone(&core);
    let cancelled_for_stream = Arc::clone(&cancelled);
    let join = std::thread::spawn(move || {
        let _ = core_for_stream.handle_stream(
            &mut writer,
            request,
            StreamKind::LivePreview,
            Some(cancelled_for_stream.as_ref()),
        );
    });

    let _ = handle.wait_for_event("ready", Duration::from_secs(2));

    if let Ok(session) = core.session_manager.get(
        &crate::domain::SessionId::try_new("timeline-session")
            .expect("timeline session id should be valid"),
    ) {
        let mut guard = session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = guard.type_text("echo timeline\n");
    }

    let command = handle.wait_for_event("command", Duration::from_millis(300));
    cancelled.store(true, Ordering::Relaxed);
    let _ = join.join();
    core.shutdown_all_sessions();

    assert!(
        command.is_none(),
        "live preview stream should not emit command events"
    );
}

#[cfg(unix)]
#[test]
fn live_preview_stream_emits_resize_event_for_resize() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = Arc::new(
        RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        )
        .expect("rpc core should initialize"),
    );

    let spawn_result = core.session_manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(
            crate::domain::SessionId::try_new("timeline-resize-session")
                .expect("timeline resize session id should be valid"),
        ),
        TerminalSize::default(),
    );
    if spawn_result.is_err() {
        return;
    }

    let (mut writer, handle) = RecordingWriter::new();
    let cancelled = Arc::new(AtomicBool::new(false));
    let request = make_request(Some(r#"{"session":"timeline-resize-session"}"#));

    let core_for_stream = Arc::clone(&core);
    let cancelled_for_stream = Arc::clone(&cancelled);
    let join = std::thread::spawn(move || {
        let _ = core_for_stream.handle_stream(
            &mut writer,
            request,
            StreamKind::LivePreview,
            Some(cancelled_for_stream.as_ref()),
        );
    });

    let _ = handle.wait_for_event("ready", Duration::from_secs(2));

    if let Ok(session) = core.session_manager.get(
        &crate::domain::SessionId::try_new("timeline-resize-session")
            .expect("timeline resize session id should be valid"),
    ) {
        let mut guard = session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = guard.resize(TerminalSize::try_new(120, 40).expect("valid terminal size"));
    }

    let resize = handle.wait_for_event("resize", Duration::from_secs(2));
    cancelled.store(true, Ordering::Relaxed);
    let _ = join.join();
    core.shutdown_all_sessions();

    let Some(resize) = resize else {
        panic!("live preview stream did not emit resize event");
    };
    assert_eq!(resize["result"]["cols"], 120);
    assert_eq!(resize["result"]["rows"], 40);
}

#[test]
fn flightdeck_stream_emits_ready_with_sessions_payload() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = Arc::new(
        RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        )
        .expect("rpc core should initialize"),
    );

    let (mut writer, handle) = RecordingWriter::new();
    let cancelled = Arc::new(AtomicBool::new(false));
    let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

    let core_for_stream = Arc::clone(&core);
    let cancelled_for_stream = Arc::clone(&cancelled);
    let join = std::thread::spawn(move || {
        let _ = core_for_stream.handle_stream(
            &mut writer,
            request,
            StreamKind::Flightdeck,
            Some(cancelled_for_stream.as_ref()),
        );
    });

    let ready = handle.wait_for_event("ready", Duration::from_secs(2));
    cancelled.store(true, Ordering::Relaxed);
    let _ = join.join();

    let Some(ready) = ready else {
        panic!("flightdeck stream did not emit ready event");
    };
    assert!(ready["result"]["sessions"].is_array());
    assert!(
        ready["result"].get("active_session").is_some(),
        "ready payload should include active_session"
    );
}

#[cfg(unix)]
#[test]
fn flightdeck_stream_emits_sessions_event_when_inventory_changes() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = Arc::new(
        RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        )
        .expect("rpc core should initialize"),
    );

    let (mut writer, handle) = RecordingWriter::new();
    let cancelled = Arc::new(AtomicBool::new(false));
    let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

    let core_for_stream = Arc::clone(&core);
    let cancelled_for_stream = Arc::clone(&cancelled);
    let join = std::thread::spawn(move || {
        let _ = core_for_stream.handle_stream(
            &mut writer,
            request,
            StreamKind::Flightdeck,
            Some(cancelled_for_stream.as_ref()),
        );
    });

    let _ = handle.wait_for_event("ready", Duration::from_secs(2));
    let spawn_result = core.session_manager.spawn(
        "sh",
        &[],
        None,
        None,
        Some(
            crate::domain::SessionId::try_new("flightdeck-new")
                .expect("flightdeck session id should be valid"),
        ),
        TerminalSize::default(),
    );
    if spawn_result.is_err() {
        cancelled.store(true, Ordering::Relaxed);
        let _ = join.join();
        return;
    }

    let sessions = handle.wait_for_event("sessions", Duration::from_secs(3));
    cancelled.store(true, Ordering::Relaxed);
    let _ = join.join();
    core.shutdown_all_sessions();

    let Some(sessions) = sessions else {
        panic!("flightdeck stream did not emit sessions event");
    };
    let contains_new = sessions["result"]["sessions"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .any(|item| item.get("id").and_then(|v| v.as_str()) == Some("flightdeck-new"))
        })
        .unwrap_or(false);
    assert!(
        contains_new,
        "sessions event should include newly spawned session"
    );
}

#[test]
fn flightdeck_stream_emits_closed_and_exits_on_cancellation() {
    let shutdown = Arc::new(AtomicBool::new(false));
    let notifier: crate::usecases::ports::ShutdownNotifierHandle =
        Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
    let core = Arc::new(
        RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        )
        .expect("rpc core should initialize"),
    );

    let (mut writer, handle) = RecordingWriter::new();
    let cancelled = Arc::new(AtomicBool::new(false));
    let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

    let core_for_stream = Arc::clone(&core);
    let cancelled_for_stream = Arc::clone(&cancelled);
    let (finished_tx, finished_rx) = std::sync::mpsc::sync_channel(1);
    let join = std::thread::spawn(move || {
        let _ = core_for_stream.handle_stream(
            &mut writer,
            request,
            StreamKind::Flightdeck,
            Some(cancelled_for_stream.as_ref()),
        );
        let _ = finished_tx.send(());
    });

    let _ = handle.wait_for_event("ready", Duration::from_secs(2));
    cancelled.store(true, Ordering::Relaxed);
    assert!(
        finished_rx.recv_timeout(Duration::from_secs(1)).is_ok(),
        "stream thread should finish promptly when cancelled"
    );
    let _ = join.join();

    let closed = handle.wait_for_event("closed", Duration::from_secs(1));
    assert!(closed.is_some(), "expected closed event on cancellation");
}
