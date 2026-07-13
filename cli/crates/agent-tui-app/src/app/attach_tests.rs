use super::*;
use crate::infra::ipc::client::DaemonClientConfig;
use crate::test_support::MockClient;
use serde_json::Value;

static PANIC_HOOK_TEST_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

fn lock_panic_hook_tests() -> std::sync::MutexGuard<'static, ()> {
    PANIC_HOOK_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Default)]
struct ResizeErrorClient {
    calls: Vec<(String, Option<Value>)>,
}

impl ResizeErrorClient {
    fn last_resize_params(&self) -> Option<&Value> {
        self.calls
            .iter()
            .rev()
            .find(|(method, _)| method == "resize")
            .and_then(|(_, params)| params.as_ref())
    }
}

impl DaemonClient for ResizeErrorClient {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        self.calls.push((method.to_string(), params));
        if method == "resize" {
            return Err(ClientError::RpcError {
                code: -32001,
                message: "resize failed".to_string(),
                category: None,
                retryable: false,
                retry_delay_ms: None,
                context: None,
                suggestion: Some("check daemon".to_string()),
            });
        }
        Err(ClientError::InvalidResponse)
    }

    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        _config: &DaemonClientConfig,
    ) -> Result<Value, ClientError> {
        self.call(method, params)
    }
}

#[test]
fn test_key_event_to_bytes_char() {
    let event = event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("a"));
}

#[test]
fn test_key_event_to_bytes_ctrl() {
    let event = event::KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("Ctrl+C"));
}

#[test]
fn test_key_event_to_bytes_enter() {
    let event = event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("Enter"));
}

#[test]
fn test_key_event_to_bytes_arrow() {
    let event = event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        key_event_to_bytes(&event),
        key_to_escape_sequence("ArrowUp")
    );
}

#[test]
fn test_key_event_to_bytes_f1() {
    let event = event::KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("F1"));
}

#[test]
fn test_key_event_to_bytes_ignores_release_events() {
    let event = event::KeyEvent::new_with_kind(
        KeyCode::Char('q'),
        KeyModifiers::CONTROL,
        KeyEventKind::Release,
    );
    assert_eq!(key_event_to_bytes(&event), None);
}

#[test]
fn paste_burst_state_flushes_single_char_as_normal_input() {
    let mut state = PasteBurstState::default();
    let now = Instant::now();

    assert_eq!(state.on_plain_char('a', now), None);

    let flushed = state
        .flush_ready(now + ATTACH_PASTE_BURST_CHAR_INTERVAL + Duration::from_millis(1))
        .expect("pending char should flush after the burst interval");
    assert_eq!(flushed, BufferedAttachInput::normal(b"a".to_vec()));
}

#[test]
fn paste_burst_state_groups_fast_chars_into_bypass_buffer() {
    let mut state = PasteBurstState::default();
    let now = Instant::now();

    assert_eq!(state.on_plain_char('a', now), None);
    assert_eq!(
        state.on_plain_char('b', now + Duration::from_millis(1)),
        None
    );

    let flushed = state
        .flush_ready(now + ATTACH_PASTE_BURST_CHAR_INTERVAL + Duration::from_millis(2))
        .expect("buffered paste burst should flush after inactivity");
    assert_eq!(flushed, BufferedAttachInput::bypass(b"ab".to_vec()));
}

#[test]
fn paste_burst_state_flushes_previous_burst_before_holding_next_char() {
    let mut state = PasteBurstState::default();
    let now = Instant::now();

    assert_eq!(state.on_plain_char('a', now), None);
    assert_eq!(
        state.on_plain_char('b', now + Duration::from_millis(1)),
        None
    );

    let flushed = state
        .on_plain_char(
            'c',
            now + ATTACH_PASTE_BURST_CHAR_INTERVAL + Duration::from_millis(2),
        )
        .expect("stale burst should flush before a new pending char is held");
    assert_eq!(flushed, BufferedAttachInput::bypass(b"ab".to_vec()));

    let pending = state
        .flush_all()
        .expect("next char should remain pending after the earlier burst flush");
    assert_eq!(pending, BufferedAttachInput::normal(b"c".to_vec()));
}

#[test]
fn test_render_initial_screen_writes_snapshot() {
    let mut client = MockClient::new_strict();
    client.set_response(
        "snapshot",
        serde_json::from_str(
            r#"{"screenshot":"hello\nworld","cursor":{"row":1,"col":2,"visible":true}}"#,
        )
        .expect("snapshot response should parse"),
    );

    let mut buffer = Vec::new();
    render_initial_screen(&mut client, "sess1", &mut buffer)
        .expect("initial snapshot should render");

    let output = String::from_utf8_lossy(&buffer);
    let mut expected_prefix = Vec::new();
    queue!(
        expected_prefix,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor
    )
    .expect("terminal prefix should render");
    let expected_prefix = String::from_utf8_lossy(&expected_prefix);
    assert!(output.contains(expected_prefix.as_ref()));
    assert!(output.contains("hello\nworld"));
    let mut expected_cursor = Vec::new();
    queue!(expected_cursor, cursor::MoveTo(2, 1), cursor::Show).expect("cursor should render");
    let expected_cursor = String::from_utf8_lossy(&expected_cursor);
    assert!(output.contains(expected_cursor.as_ref()));

    assert_eq!(client.call_count("snapshot"), 1);
    let mut params = client.params_for("snapshot");
    assert_eq!(params.len(), 1);
    let params = params
        .pop()
        .flatten()
        .expect("snapshot params should be recorded");
    assert_eq!(params["session"], "sess1");
    assert_eq!(params["include_cursor"], true);
    assert_eq!(params["include_render"], true);
}

#[test]
fn test_render_initial_screen_prefers_full_rendered_snapshot() {
    let mut client = MockClient::new_strict();
    client.set_response(
        "snapshot",
        serde_json::from_str(
            r#"{
                "screenshot":"hello\nworld",
                "rendered":"hello     \nworld",
                "compact_rendered":"hello\nworld",
                "cursor":{"row":1,"col":2,"visible":true}
            }"#,
        )
        .expect("snapshot response should parse"),
    );

    let mut buffer = Vec::new();
    render_initial_screen(&mut client, "sess1", &mut buffer)
        .expect("initial snapshot should render");

    let output = String::from_utf8_lossy(&buffer);
    assert!(output.contains("hello     \nworld"));
    assert!(!output.contains("hello\nworld"));
}

#[test]
fn test_detach_detector_ctrl_p_ctrl_b_detaches() {
    let detach_keys = DetachKeys::default();
    let mut detector = DetachDetector::new(&detach_keys);
    let (out, detach) = detector.consume(&[0x10]);
    assert!(out.is_empty());
    assert!(!detach);

    let (out, detach) = detector.consume(&[0x02]);
    assert!(out.is_empty());
    assert!(detach);
}

#[test]
fn test_detach_detector_passes_through_non_sequence() {
    let detach_keys = DetachKeys::default();
    let mut detector = DetachDetector::new(&detach_keys);
    let (out, detach) = detector.consume(b"ab");
    assert_eq!(out, b"ab");
    assert!(!detach);
}

#[test]
fn test_detach_detector_ctrl_p_followed_by_key_sends_both() {
    let detach_keys = DetachKeys::default();
    let mut detector = DetachDetector::new(&detach_keys);
    let (out, detach) = detector.consume(&[0x10, b'a']);
    assert_eq!(out, vec![0x10, b'a']);
    assert!(!detach);
}

#[test]
fn test_detach_detector_ctrl_p_ctrl_p_sends_two() {
    let detach_keys = DetachKeys::default();
    let mut detector = DetachDetector::new(&detach_keys);
    let (out, detach) = detector.consume(&[0x10, 0x10]);
    assert_eq!(out, vec![0x10, 0x10]);
    assert!(!detach);
}

#[test]
fn test_detach_detector_cancel_partial_match_returns_pending_prefix() {
    let detach_keys = "a,b"
        .parse::<DetachKeys>()
        .expect("custom detach keys should parse");
    let mut detector = DetachDetector::new(&detach_keys);

    let (out, detach) = detector.consume(b"a");
    assert!(out.is_empty());
    assert!(!detach);
    assert!(detector.is_partial_match());

    assert_eq!(detector.cancel_partial_match(), b"a");
    assert!(!detector.is_partial_match());
    assert!(detector.cancel_partial_match().is_empty());
}

#[test]
fn test_detach_keys_from_str_default() {
    let keys = "ctrl-p,ctrl-b"
        .parse::<DetachKeys>()
        .expect("default detach keys should parse");
    assert_eq!(keys.bytes(), &[0x10, 0x02]);
    assert_eq!(keys.display(), "Ctrl-P Ctrl-B");
}

#[test]
fn test_detach_keys_from_str_none() {
    let keys = "none"
        .parse::<DetachKeys>()
        .expect("disabled detach keys should parse");
    assert!(keys.is_disabled());
}

#[test]
fn test_detach_keys_invalid_token() {
    let err = "ctrl-"
        .parse::<DetachKeys>()
        .expect_err("invalid detach keys should fail");
    assert!(err.contains("ctrl-"));
}

#[test]
fn test_parse_stream_event_output() {
    let payload = STANDARD.encode(b"hello");
    let value = RpcValue::new(
        serde_json::from_str(&format!(
            r#"{{"event":"output","data":"{payload}","dropped_bytes":2}}"#
        ))
        .expect("output event payload should parse"),
    );
    let event = parse_stream_event(value)
        .expect("output event should parse")
        .expect("output event should be emitted");
    match event {
        AttachStreamEvent::Output {
            data,
            dropped_bytes,
        } => {
            assert_eq!(data, b"hello");
            assert_eq!(dropped_bytes, 2);
        }
        _ => panic!("expected output event"),
    }
}

#[test]
fn test_parse_stream_event_dropped() {
    let value = RpcValue::new(
        serde_json::from_str(r#"{"event":"dropped","dropped_bytes":128}"#)
            .expect("dropped event payload should parse"),
    );
    let event = parse_stream_event(value)
        .expect("dropped event should parse")
        .expect("dropped event should be emitted");
    match event {
        AttachStreamEvent::Dropped(bytes) => assert_eq!(bytes, 128),
        _ => panic!("expected dropped event"),
    }
}

#[test]
fn test_parse_stream_event_closed() {
    let value = RpcValue::new(
        serde_json::from_str(r#"{"event":"closed"}"#).expect("closed event payload should parse"),
    );
    let event = parse_stream_event(value)
        .expect("closed event should parse")
        .expect("closed event should be emitted");
    match event {
        AttachStreamEvent::Closed => {}
        _ => panic!("expected closed event"),
    }
}

#[test]
fn prepare_terminal_with_rollback_restores_terminal_after_failure() {
    let mut buffer = Vec::new();
    let err = prepare_terminal_with_rollback(&mut buffer, |stdout| {
        stdout.write_all(b"partial")?;
        Err(io::Error::other("boom"))
    })
    .expect_err("failing setup should trigger rollback");
    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert_eq!(err.to_string(), "boom");

    let mut reset = Vec::new();
    reset_terminal_modes(&mut reset).expect("terminal reset sequence should render");
    assert!(
        buffer.ends_with(&reset),
        "terminal reset sequence should be written after setup failure"
    );
}

#[test]
fn attach_output_worker_shutdown_aborts_before_joining() {
    let aborted = Arc::new(AtomicBool::new(false));
    let aborted_for_signal = Arc::clone(&aborted);
    let (shutdown_tx, shutdown_rx) = channel::bounded(1);
    let shutdown_signal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        aborted_for_signal.store(true, Ordering::Relaxed);
        let _ = shutdown_tx.send(());
    });
    let (tx, rx) = channel::bounded(1);
    let join = thread::spawn(move || {
        let _ = shutdown_rx.recv();
        let _ = tx.send(Ok(()));
    });

    let mut worker = AttachOutputWorker {
        done_rx: rx,
        join: Some(join),
        shutdown_signal: Some(shutdown_signal),
    };

    worker.shutdown(Duration::from_millis(250));

    assert!(aborted.load(Ordering::Relaxed));
    assert!(worker.join.is_none());
    assert!(worker.shutdown_signal.is_none());
}

#[test]
fn sync_attach_resize_surfaces_structured_warning_text() {
    let mut client = ResizeErrorClient::default();
    let size = TerminalSize::try_new(120, 40).expect("terminal size should validate");

    let error = sync_attach_resize(&mut client, "sess1", size)
        .expect_err("resize error should be returned to the caller");
    let warning = attach_resize_warning(&error);

    assert!(warning.contains("Resize sync failed: RPC error (-32001): resize failed"));
    assert!(warning.contains("check daemon"));

    let params = client
        .last_resize_params()
        .expect("resize params should be recorded");
    assert_eq!(params["session"], "sess1");
    assert_eq!(params["cols"], 120);
    assert_eq!(params["rows"], 40);
}

#[test]
fn terminal_panic_hook_guard_chains_previous_hook() {
    let _hook_lock = lock_panic_hook_tests();
    let original_hook = panic::take_hook();
    let previous_hook_called = Arc::new(AtomicBool::new(false));
    let previous_hook_called_for_hook = Arc::clone(&previous_hook_called);
    panic::set_hook(Box::new(move |_| {
        previous_hook_called_for_hook.store(true, Ordering::Relaxed);
    }));

    {
        let _guard = TerminalPanicHookGuard::install();
        let result = panic::catch_unwind(|| panic!("attach panic"));
        assert!(result.is_err());
    }

    assert!(previous_hook_called.load(Ordering::Relaxed));

    let current_hook = panic::take_hook();
    drop(current_hook);
    panic::set_hook(original_hook);
}

#[test]
fn terminal_panic_hook_guard_restore_reinstates_previous_hook() {
    let _hook_lock = lock_panic_hook_tests();
    let original_hook = panic::take_hook();
    let previous_hook_called = Arc::new(AtomicBool::new(false));
    let previous_hook_called_for_hook = Arc::clone(&previous_hook_called);
    panic::set_hook(Box::new(move |_| {
        previous_hook_called_for_hook.store(true, Ordering::Relaxed);
    }));

    let mut guard = TerminalPanicHookGuard::install();
    assert!(guard.has_previous_hook());
    guard.restore();
    assert!(!guard.has_previous_hook());

    let result = panic::catch_unwind(|| panic!("attach panic after restore"));
    assert!(result.is_err());
    assert!(previous_hook_called.load(Ordering::Relaxed));

    let current_hook = panic::take_hook();
    drop(current_hook);
    panic::set_hook(original_hook);
}

#[test]
fn stdin_reader_worker_shutdown_is_bounded() {
    let mut worker = spawn_stdin_reader();
    worker.shutdown(Duration::from_millis(250));
    assert!(worker.join.is_none());
}

#[test]
fn event_reader_worker_shutdown_is_bounded() {
    let mut worker = spawn_event_reader();
    worker.shutdown(Duration::from_millis(250));
    assert!(worker.join.is_none());
}
