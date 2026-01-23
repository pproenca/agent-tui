use agent_tui_common::ValueExt;
use agent_tui_core::Component;
use agent_tui_core::Element;
use agent_tui_ipc::RpcRequest;
use agent_tui_ipc::RpcResponse;
use agent_tui_ipc::socket_path;

use crate::error::DaemonError;
use crate::error::DomainError;
use serde_json::Value;
use serde_json::json;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use crate::ansi_keys;
use crate::config::DaemonConfig;
use crate::metrics::DaemonMetrics;
use crate::session::{Session, SessionError, SessionManager};
use crate::{LOCK_TIMEOUT, acquire_session_lock, navigate_to_option, strip_ansi_codes};
use crate::{RecordingFrame, StableTracker, WaitCondition, check_condition};

struct SizeLimitedReader<R> {
    inner: R,
    max_size: usize,
    read_count: usize,
}

impl<R> SizeLimitedReader<R> {
    fn new(inner: R, max_size: usize) -> Self {
        Self {
            inner,
            max_size,
            read_count: 0,
        }
    }
}

impl<R: BufRead> Iterator for SizeLimitedReader<R> {
    type Item = std::io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.inner.read_line(&mut line) {
            Ok(0) => None,
            Ok(n) => {
                self.read_count += n;
                if self.read_count > self.max_size {
                    Some(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Request size limit exceeded",
                    )))
                } else {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    Some(Ok(line))
                }
            }
            Err(e) => Some(Err(e)),
        }
    }
}

fn matches_text(haystack: Option<&String>, needle: &str, exact: bool) -> bool {
    match haystack {
        Some(h) if exact => h == needle,
        Some(h) => h.to_lowercase().contains(&needle.to_lowercase()),
        None => false,
    }
}

fn update_with_warning(sess: &mut Session) -> Option<String> {
    let warning = match sess.update() {
        Ok(()) => None,
        Err(e) => {
            eprintln!("Warning: Session update failed: {}", e);
            Some(format!("Element data may be stale: {}", e))
        }
    };
    sess.detect_elements();
    warning
}

fn build_asciicast(session_id: &str, cols: u16, rows: u16, frames: &[RecordingFrame]) -> Value {
    let mut output = Vec::new();

    let duration = frames
        .last()
        .map(|f| f.timestamp_ms as f64 / 1000.0)
        .unwrap_or(0.0);

    let header = json!({
        "version": 2,
        "width": cols,
        "height": rows,
        "timestamp": chrono::Utc::now().timestamp(),
        "duration": duration,
        "title": format!("agent-tui recording - {}", session_id),
        "env": {
            "TERM": "xterm-256color",
            "SHELL": std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    });

    match serde_json::to_string(&header) {
        Ok(s) => output.push(s),
        Err(e) => {
            eprintln!("Error: Failed to serialize asciicast header: {}", e);
            return json!({
                "format": "asciicast",
                "version": 2,
                "error": format!("Failed to serialize recording header: {}", e)
            });
        }
    }

    let mut prev_screen = String::new();
    for frame in frames {
        let time_secs = frame.timestamp_ms as f64 / 1000.0;
        if frame.screen != prev_screen {
            let screen_data = if prev_screen.is_empty() {
                frame.screen.clone()
            } else {
                format!("\x1b[2J\x1b[H{}", frame.screen)
            };
            let event = json!([time_secs, "o", screen_data]);
            match serde_json::to_string(&event) {
                Ok(s) => output.push(s),
                Err(e) => {
                    eprintln!("Error: Failed to serialize asciicast frame: {}", e);
                }
            }
            prev_screen = frame.screen.clone();
        }
    }

    json!({
        "format": "asciicast",
        "version": 2,
        "data": output.join("\n")
    })
}

fn build_wait_suggestion(condition: &WaitCondition) -> String {
    match condition {
        WaitCondition::Text(t) => format!(
            "Text '{}' not found. Check if the app finished loading or try 'snapshot -i' to see current screen.",
            t
        ),
        WaitCondition::Element(e) => format!(
            "Element {} not found. Try 'snapshot -i' to see available elements.",
            e
        ),
        WaitCondition::Focused(e) => format!(
            "Element {} exists but is not focused. Try 'click {}' to focus it.",
            e, e
        ),
        WaitCondition::NotVisible(e) => format!(
            "Element {} is still visible. The app may still be processing.",
            e
        ),
        WaitCondition::Stable => {
            "Screen is still changing. The app may have animations or be loading.".to_string()
        }
        WaitCondition::TextGone(t) => format!(
            "Text '{}' is still visible. The operation may not have completed.",
            t
        ),
        WaitCondition::Value { element, expected } => format!(
            "Element {} does not have value '{}'. Check if input was accepted.",
            element, expected
        ),
    }
}

struct ElementFilter<'a> {
    role: Option<&'a str>,
    name: Option<&'a str>,
    text: Option<&'a str>,
    placeholder: Option<&'a str>,
    focused_only: bool,
    exact: bool,
}

impl ElementFilter<'_> {
    fn matches(&self, el: &Element) -> bool {
        if let Some(r) = self.role {
            if el.element_type.as_str() != r {
                return false;
            }
        }
        if let Some(n) = self.name {
            if !matches_text(el.label.as_ref(), n, self.exact) {
                return false;
            }
        }
        if let Some(t) = self.text {
            let in_label = matches_text(el.label.as_ref(), t, self.exact);
            let in_value = matches_text(el.value.as_ref(), t, self.exact);
            if !in_label && !in_value {
                return false;
            }
        }
        if let Some(p) = self.placeholder {
            if !matches_text(el.hint.as_ref(), p, self.exact) {
                return false;
            }
        }
        if self.focused_only && !el.focused {
            return false;
        }
        true
    }

    fn apply(&self, elements: &[Element]) -> Vec<Value> {
        elements
            .iter()
            .filter(|el| self.matches(el))
            .map(element_to_json)
            .collect()
    }

    fn count(&self, elements: &[Element]) -> usize {
        elements.iter().filter(|el| self.matches(el)).count()
    }
}

const MAX_CONNECTIONS: usize = 64;
const MAX_REQUEST_SIZE: usize = 1_048_576; // 1MB
const CHANNEL_CAPACITY: usize = 256;
const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

pub struct DaemonServer {
    session_manager: Arc<SessionManager>,
    start_time: Instant,
    #[allow(dead_code)] // Stored to keep Arc alive; accessed via clone in start_daemon
    shutdown: Arc<AtomicBool>,
    active_connections: Arc<AtomicUsize>,
    metrics: Arc<DaemonMetrics>,
}

impl Default for DaemonServer {
    fn default() -> Self {
        Self::new()
    }
}

struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: SyncSender<UnixStream>,
}

impl ThreadPool {
    fn new(
        size: usize,
        server: Arc<DaemonServer>,
        shutdown: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<UnixStream>(CHANNEL_CAPACITY);
        let receiver = Arc::new(std::sync::Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let receiver = Arc::clone(&receiver);
            let server = Arc::clone(&server);
            let shutdown = Arc::clone(&shutdown);

            let handle =
                match thread::Builder::new()
                    .name(format!("worker-{}", id))
                    .spawn(move || {
                        loop {
                            if shutdown.load(Ordering::Relaxed) {
                                break;
                            }

                            let stream = {
                                let lock = match receiver.lock() {
                                    Ok(l) => l,
                                    Err(e) => {
                                        eprintln!("Worker {} receiver lock poisoned: {}", id, e);
                                        break;
                                    }
                                };
                                match lock.recv_timeout(Duration::from_millis(100)) {
                                    Ok(stream) => stream,
                                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            };

                            server.active_connections.fetch_add(1, Ordering::Relaxed);
                            server.handle_client(stream);
                            server.active_connections.fetch_sub(1, Ordering::Relaxed);
                        }
                    }) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("Failed to spawn worker {}: {}", id, e);
                        continue;
                    }
                };

            workers.push(handle);
        }

        if workers.is_empty() {
            return Err(std::io::Error::other("Failed to spawn any worker threads"));
        }

        if workers.len() < size {
            eprintln!(
                "Warning: Only spawned {}/{} worker threads",
                workers.len(),
                size
            );
        }

        Ok(ThreadPool { workers, sender })
    }

    fn execute(&self, stream: UnixStream) -> Result<(), UnixStream> {
        self.sender.try_send(stream).map_err(|e| match e {
            mpsc::TrySendError::Full(s) | mpsc::TrySendError::Disconnected(s) => s,
        })
    }

    fn shutdown(self) {
        drop(self.sender);
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

fn combine_warnings(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(format!("{}. {}", x, y)),
        (w @ Some(_), None) | (None, w @ Some(_)) => w,
        (None, None) => None,
    }
}

/// Convert a DomainError into an RpcResponse with structured error data.
fn domain_error_response(id: u64, err: &DomainError) -> RpcResponse {
    RpcResponse::domain_error(
        id,
        err.code(),
        &err.to_string(),
        err.category().as_str(),
        Some(err.context()),
        Some(err.suggestion()),
    )
}

/// Create a lock timeout error response.
fn lock_timeout_response(id: u64, session_id: Option<&str>) -> RpcResponse {
    let err = DomainError::LockTimeout {
        session_id: session_id.map(String::from),
    };
    domain_error_response(id, &err)
}

impl DaemonServer {
    pub fn new() -> Self {
        Self::with_config(DaemonConfig::default())
    }

    pub fn with_config(config: DaemonConfig) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::with_max_sessions(config.max_sessions)),
            start_time: Instant::now(),
            shutdown: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(DaemonMetrics::new()),
        }
    }

    pub fn with_shutdown_and_config(shutdown: Arc<AtomicBool>, config: DaemonConfig) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::with_max_sessions(config.max_sessions)),
            start_time: Instant::now(),
            shutdown,
            active_connections: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(DaemonMetrics::new()),
        }
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(e) = self.session_manager.kill(info.id.as_str()) {
                eprintln!("Warning: Failed to kill session {}: {}", info.id, e);
            }
        }
    }

    fn with_session<F>(&self, request: &RpcRequest, session_id: Option<&str>, f: F) -> RpcResponse
    where
        F: FnOnce(&mut Session) -> RpcResponse,
    {
        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    self.metrics.record_lock_timeout();
                    return lock_timeout_response(request.id, session_id);
                };
                f(&mut sess)
            }
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn with_session_and_ref<F>(&self, request: &RpcRequest, f: F) -> RpcResponse
    where
        F: FnOnce(&mut Session, &str) -> RpcResponse,
    {
        let element_ref = match request.require_str("ref") {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        let session_id = request.param_str("session");

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    self.metrics.record_lock_timeout();
                    return lock_timeout_response(request.id, session_id);
                };
                f(&mut sess, element_ref)
            }
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn with_detected_session_and_ref<F>(&self, request: &RpcRequest, f: F) -> RpcResponse
    where
        F: FnOnce(&mut Session, &str, Option<String>) -> RpcResponse,
    {
        self.with_session_and_ref(request, |sess, element_ref| {
            let update_warning = update_with_warning(sess);
            f(sess, element_ref, update_warning)
        })
    }

    fn with_detected_session<F>(&self, request: &RpcRequest, f: F) -> RpcResponse
    where
        F: FnOnce(&mut Session, Option<String>) -> RpcResponse,
    {
        let session_id = request.param_str("session");
        self.with_session(request, session_id, |sess| {
            let update_warning = update_with_warning(sess);
            f(sess, update_warning)
        })
    }

    fn with_session_action<F>(&self, request: &RpcRequest, param: &str, f: F) -> RpcResponse
    where
        F: FnOnce(&mut Session, &str) -> Result<(), Box<dyn std::error::Error>>,
    {
        let req_id = request.id;
        let value = match request.require_str(param) {
            Ok(v) => v.to_string(),
            Err(resp) => return resp,
        };
        let session_id = request.param_str("session");
        self.with_session(request, session_id, |sess| match f(sess, &value) {
            Ok(()) => RpcResponse::action_success(req_id),
            Err(e) => {
                let err_str = e.to_string();
                let domain_err = if err_str.contains("Invalid key") {
                    DomainError::InvalidKey { key: value.clone() }
                } else {
                    DomainError::PtyError {
                        operation: param.to_string(),
                        reason: err_str,
                    }
                };
                domain_error_response(req_id, &domain_err)
            }
        })
    }

    fn element_property<F, T>(
        &self,
        request: &RpcRequest,
        field_name: &str,
        extract: F,
    ) -> RpcResponse
    where
        F: FnOnce(&Element) -> T,
        T: serde::Serialize,
    {
        let req_id = request.id;
        self.with_detected_session_and_ref(request, |sess, element_ref, update_warning| {
            let mut response = match sess.find_element(element_ref) {
                Some(el) => json!({
                    "ref": element_ref,
                    field_name: extract(el),
                    "found": true
                }),
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.to_string(),
                        session_id: Some(sess.id.to_string()),
                    };
                    json!({
                        "ref": element_ref,
                        field_name: serde_json::Value::Null,
                        "found": false,
                        "message": err.suggestion()
                    })
                }
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn element_action(&self, request: &RpcRequest, pty_bytes: &[u8]) -> RpcResponse {
        let req_id = request.id;
        self.with_detected_session_and_ref(request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.to_string(),
                    session_id: Some(sess.id.to_string()),
                };
                return domain_error_response(req_id, &err);
            }
            if let Err(e) = sess.pty_write(pty_bytes) {
                let err = DomainError::PtyError {
                    operation: "write".to_string(),
                    reason: e.to_string(),
                };
                return domain_error_response(req_id, &err);
            }
            let mut response = json!({ "success": true, "ref": element_ref });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_request(&self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "ping" => RpcResponse::success(request.id, json!({ "pong": true })),

            "health" => {
                let uptime_ms = self.start_time.elapsed().as_millis() as u64;
                RpcResponse::success(
                    request.id,
                    json!({
                        "status": "healthy",
                        "pid": std::process::id(),
                        "uptime_ms": uptime_ms,
                        "session_count": self.session_manager.session_count(),
                        "version": env!("CARGO_PKG_VERSION"),
                        "active_connections": self.active_connections.load(Ordering::Relaxed),
                        "total_requests": self.metrics.requests(),
                        "error_count": self.metrics.errors()
                    }),
                )
            }

            "metrics" => self.handle_metrics(request),

            "spawn" => self.handle_spawn(request),
            "snapshot" => self.handle_snapshot(request),

            "screen" => RpcResponse::error(
                request.id,
                -32601,
                "Method 'screen' is deprecated. Use 'snapshot' with strip_ansi=true instead.",
            ),
            "click" => self.handle_click(request),
            "dbl_click" => self.handle_dbl_click(request),
            "fill" => self.handle_fill(request),
            "keystroke" => self.handle_keystroke(request),
            "keydown" => self.handle_keydown(request),
            "keyup" => self.handle_keyup(request),
            "type" => self.handle_type(request),
            "wait" => self.handle_wait(request),
            "kill" => self.handle_kill(request),
            "restart" => self.handle_restart(request),
            "sessions" => self.handle_sessions(request),
            "resize" => self.handle_resize(request),
            "find" => self.handle_find(request),
            "get_text" => self.handle_get_text(request),
            "get_value" => self.handle_get_value(request),
            "is_visible" => self.handle_is_visible(request),
            "is_focused" => self.handle_is_focused(request),
            "is_enabled" => self.handle_is_enabled(request),
            "is_checked" => self.handle_is_checked(request),
            "count" => self.handle_count(request),
            "scroll" => self.handle_scroll(request),
            "scroll_into_view" => self.handle_scroll_into_view(request),
            "focus" => self.handle_focus(request),
            "get_focused" => self.handle_get_focused(request),
            "get_title" => self.handle_get_title(request),
            "clear" => self.handle_clear(request),
            "select_all" => self.handle_select_all(request),
            "toggle" => self.handle_toggle(request),
            "select" => self.handle_select(request),
            "multiselect" => self.handle_multiselect(request),
            "attach" => self.handle_attach(request),
            "record_start" => self.handle_record_start(request),
            "record_stop" => self.handle_record_stop(request),
            "record_status" => self.handle_record_status(request),
            "trace" => self.handle_trace(request),
            "console" => self.handle_console(request),
            "errors" => self.handle_errors(request),
            "pty_read" => self.handle_pty_read(request),
            "pty_write" => self.handle_pty_write(request),

            _ => RpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_metrics(&self, request: RpcRequest) -> RpcResponse {
        RpcResponse::success(
            request.id,
            json!({
                "requests_total": self.metrics.requests(),
                "errors_total": self.metrics.errors(),
                "lock_timeouts": self.metrics.lock_timeouts(),
                "poison_recoveries": self.metrics.poison_recoveries(),
                "uptime_ms": self.start_time.elapsed().as_millis() as u64,
                "active_connections": self.active_connections.load(Ordering::Relaxed),
                "session_count": self.session_manager.session_count()
            }),
        )
    }

    fn handle_spawn(&self, request: RpcRequest) -> RpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => return RpcResponse::error(request.id, -32602, "Missing params"),
        };

        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("bash");

        let args: Vec<String> = params
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let cwd = params.get("cwd").and_then(|v| v.as_str());

        let session_id = params
            .get("session")
            .and_then(|v| v.as_str())
            .map(String::from);

        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;

        let cols = cols.clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS);
        let rows = rows.clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS);

        match self
            .session_manager
            .spawn(command, &args, cwd, None, session_id, cols, rows)
        {
            Ok((session_id, pid)) => RpcResponse::success(
                request.id,
                json!({
                    "session_id": session_id,
                    "pid": pid
                }),
            ),
            Err(SessionError::LimitReached(max)) => {
                let err = DomainError::SessionLimitReached { max };
                domain_error_response(request.id, &err)
            }
            Err(e) => {
                let err_str = e.to_string();
                let domain_err =
                    if err_str.contains("No such file") || err_str.contains("not found") {
                        DomainError::CommandNotFound {
                            command: command.to_string(),
                        }
                    } else if err_str.contains("Permission denied") {
                        DomainError::PermissionDenied {
                            command: command.to_string(),
                        }
                    } else {
                        DomainError::PtyError {
                            operation: "spawn".to_string(),
                            reason: err_str,
                        }
                    };
                domain_error_response(request.id, &domain_err)
            }
        }
    }

    fn handle_snapshot(&self, request: RpcRequest) -> RpcResponse {
        let params = request.params.as_ref().cloned().unwrap_or(json!({}));
        let session_id = request.param_str("session");
        let include_elements = params.bool_or("include_elements", false);
        let should_strip_ansi = params.bool_or("strip_ansi", false);
        let include_cursor = params.bool_or("include_cursor", false);
        let req_id = request.id;

        self.with_session(&request, session_id, |sess| {

            let update_warning = match sess.update() {
                Ok(()) => None,
                Err(e) => {
                    eprintln!("Warning: Session update failed during snapshot: {}", e);
                    Some(format!(
                        "Screen data may be stale. Session update failed: {}. Try 'agent-tui sessions' to check session status.",
                        e
                    ))
                }
            };

            let mut screen = sess.screen_text();
            let cursor = sess.cursor();
            let (cols, rows) = sess.size();

            if should_strip_ansi {
                screen = strip_ansi_codes(&screen);
            }

            let (elements, stats) = if include_elements {
                let vom_components = sess.analyze_screen();
                filter_interactive_components(&vom_components, &screen)
            } else {
                (
                    None,
                    json!({
                        "lines": screen.lines().count(),
                        "chars": screen.len(),
                        "elements_total": 0,
                        "elements_interactive": 0,
                        "elements_shown": 0
                    }),
                )
            };

            let mut response = json!({
                "session_id": sess.id,
                "screen": screen,
                "elements": elements,
                "size": {
                    "cols": cols,
                    "rows": rows
                },
                "stats": stats
            });


            if include_cursor || include_elements {
                response["cursor"] = json!({
                    "row": cursor.row,
                    "col": cursor.col,
                    "visible": cursor.visible
                });
            }

            if let Some(warning) = update_warning {
                response["warning"] = serde_json::Value::String(warning);
            }

            RpcResponse::success(req_id, response)
        })
    }

    fn handle_click(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_session_and_ref(&request, |sess, element_ref| {
            match sess.click(element_ref) {
                Ok(()) => RpcResponse::action_success(req_id),
                Err(e) => domain_error_response(req_id, &DomainError::from(e)),
            }
        })
    }

    fn handle_dbl_click(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_session_and_ref(&request, |sess, element_ref| {
            if let Err(e) = sess.click(element_ref) {
                return domain_error_response(req_id, &DomainError::from(e));
            }
            thread::sleep(Duration::from_millis(50));
            match sess.click(element_ref) {
                Ok(()) => RpcResponse::action_success(req_id),
                Err(e) => domain_error_response(req_id, &DomainError::from(e)),
            }
        })
    }

    fn handle_fill(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let value = match request.require_str("value") {
            Ok(v) => v.to_string(),
            Err(resp) => return resp,
        };
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let type_warning = match sess.find_element(element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "input" {
                        Some(format!(
                            "Warning: '{}' is a {} not an input field. Fill may not work as expected. \
                             Use 'snapshot -i' to see element types.",
                            element_ref, el_type
                        ))
                    } else {
                        None
                    }
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.to_string(),
                        session_id: Some(sess.id.to_string()),
                    };
                    return domain_error_response(req_id, &err);
                }
            };

            if let Err(e) = sess.pty_write(value.as_bytes()) {
                let err = DomainError::PtyError {
                    operation: "fill".to_string(),
                    reason: e.to_string(),
                };
                return domain_error_response(req_id, &err);
            }

            let mut response = json!({
                "success": true,
                "ref": element_ref,
                "value": value
            });

            if let Some(warn_msg) = combine_warnings(update_warning, type_warning) {
                response["warning"] = json!(warn_msg);
            }

            RpcResponse::success(req_id, response)
        })
    }

    fn handle_keystroke(&self, request: RpcRequest) -> RpcResponse {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keystroke(key).map_err(|e| e.into())
        })
    }

    fn handle_keydown(&self, request: RpcRequest) -> RpcResponse {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keydown(key).map_err(|e| e.into())
        })
    }

    fn handle_keyup(&self, request: RpcRequest) -> RpcResponse {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keyup(key).map_err(|e| e.into())
        })
    }

    fn handle_type(&self, request: RpcRequest) -> RpcResponse {
        self.with_session_action(&request, "text", |sess, text| {
            sess.type_text(text).map_err(|e| e.into())
        })
    }

    fn handle_wait(&self, request: RpcRequest) -> RpcResponse {
        let session_id = request.param_str("session");
        let text = request.param_str("text");
        let condition_str = request.param_str("condition");
        let target = request.param_str("target");
        let timeout_ms = request.param_u64("timeout_ms", 30000);

        let condition = match WaitCondition::parse(condition_str, target, text) {
            Some(c) => c,
            None => {
                if text.is_none() {
                    return RpcResponse::error(
                        request.id,
                        -32602,
                        "Missing condition: provide 'text' or 'condition' with 'target'",
                    );
                }
                WaitCondition::Text(text.expect("verified Some above").to_string())
            }
        };

        let session = match self.session_manager.resolve(session_id) {
            Ok(s) => s,
            Err(e) => {
                return domain_error_response(request.id, &DomainError::from(e));
            }
        };

        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let mut found = false;
        let mut stable_tracker = StableTracker::new(3);
        let mut matched_text: Option<String> = None;
        let mut element_ref: Option<String> = None;

        while start.elapsed() < timeout {
            if let Some(mut sess) = acquire_session_lock(&session, Duration::from_millis(100)) {
                if check_condition(&mut sess, &condition, &mut stable_tracker) {
                    found = true;
                    matched_text = condition.matched_text();
                    element_ref = condition.element_ref();
                    break;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        let mut response = json!({
            "found": found,
            "elapsed_ms": elapsed_ms,
            "condition": condition.description()
        });

        if found {
            if let Some(text) = matched_text {
                response["matched_text"] = json!(text);
            }
            if let Some(el_ref) = element_ref {
                response["element_ref"] = json!(el_ref);
            }
        } else if let Some(sess) = acquire_session_lock(&session, Duration::from_millis(100)) {
            let screen = sess.screen_text();
            let screen_preview: String = screen.chars().take(200).collect();
            let screen_context = if screen.len() > 200 {
                format!("{}...", screen_preview)
            } else {
                screen_preview
            };
            response["screen_context"] = json!(screen_context);
            response["suggestion"] = json!(build_wait_suggestion(&condition));
        }

        RpcResponse::success(request.id, response)
    }

    fn handle_kill(&self, request: RpcRequest) -> RpcResponse {
        let session_id = request.param_str("session");

        let session_to_kill = match session_id {
            Some(id) => id.to_string(),
            None => match self.session_manager.active_session_id() {
                Some(id) => id.to_string(),
                None => return domain_error_response(request.id, &DomainError::NoActiveSession),
            },
        };

        match self.session_manager.kill(&session_to_kill) {
            Ok(()) => RpcResponse::success(
                request.id,
                json!({
                    "success": true,
                    "session_id": session_to_kill
                }),
            ),
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_restart(&self, request: RpcRequest) -> RpcResponse {
        let session_id = request.param_str("session");

        let (old_session_id, command, cols, rows) = match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let (cols, rows) = sess.size();
                (sess.id.to_string(), sess.command.clone(), cols, rows)
            }
            Err(e) => {
                return domain_error_response(request.id, &DomainError::from(e));
            }
        };

        if let Err(e) = self.session_manager.kill(&old_session_id) {
            return domain_error_response(request.id, &DomainError::from(e));
        }

        match self
            .session_manager
            .spawn(&command, &[], None, None, None, cols, rows)
        {
            Ok((new_session_id, pid)) => RpcResponse::success(
                request.id,
                json!({
                    "success": true,
                    "old_session_id": old_session_id,
                    "new_session_id": new_session_id,
                    "command": command,
                    "pid": pid
                }),
            ),
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_sessions(&self, request: RpcRequest) -> RpcResponse {
        let sessions = self.session_manager.list();
        let active_id = self.session_manager.active_session_id();

        RpcResponse::success(
            request.id,
            json!({
                "sessions": sessions.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
                "active_session": active_id
            }),
        )
    }

    fn handle_resize(&self, request: RpcRequest) -> RpcResponse {
        let cols = request
            .param_u16("cols", 80)
            .clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS);
        let rows = request
            .param_u16("rows", 24)
            .clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS);
        let session_id = request.param_str("session");

        let req_id = request.id;
        self.with_session(&request, session_id, |sess| match sess.resize(cols, rows) {
            Ok(()) => RpcResponse::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": sess.id,
                    "size": { "cols": cols, "rows": rows }
                }),
            ),
            Err(e) => {
                let err = DomainError::PtyError {
                    operation: "resize".to_string(),
                    reason: e.to_string(),
                };
                domain_error_response(req_id, &err)
            }
        })
    }

    fn handle_find(&self, request: RpcRequest) -> RpcResponse {
        let params = request.params.as_ref().cloned().unwrap_or(json!({}));
        let filter = ElementFilter {
            role: params.get("role").and_then(|v| v.as_str()),
            name: params.get("name").and_then(|v| v.as_str()),
            text: params.get("text").and_then(|v| v.as_str()),
            placeholder: params.get("placeholder").and_then(|v| v.as_str()),
            focused_only: params.bool_or("focused", false),
            exact: params.bool_or("exact", false),
        };
        let nth = params
            .get("nth")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let req_id = request.id;
        self.with_detected_session(&request, |sess, update_warning| {
            let matches = filter.apply(sess.cached_elements());
            let final_matches = match nth {
                Some(n) if n < matches.len() => vec![matches[n].clone()],
                Some(_) => vec![],
                None => matches,
            };

            let mut response = json!({
                "session_id": sess.id,
                "elements": final_matches,
                "count": final_matches.len()
            });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_get_text(&self, request: RpcRequest) -> RpcResponse {
        self.element_property(&request, "text", |el| {
            el.label.clone().or_else(|| el.value.clone())
        })
    }

    fn handle_get_value(&self, request: RpcRequest) -> RpcResponse {
        self.element_property(&request, "value", |el| el.value.clone())
    }

    fn handle_is_visible(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let visible = sess.find_element(element_ref).is_some();
            let mut response = json!({ "ref": element_ref, "visible": visible });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_is_focused(&self, request: RpcRequest) -> RpcResponse {
        self.element_property(&request, "focused", |el| el.focused)
    }

    fn handle_is_enabled(&self, request: RpcRequest) -> RpcResponse {
        self.element_property(&request, "enabled", |el| !el.disabled.unwrap_or(false))
    }

    fn handle_is_checked(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let mut response = match sess.find_element(element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "checkbox" && el_type != "radio" {
                        json!({
                            "ref": element_ref, "checked": false, "found": true,
                            "message": format!(
                                "Element {} is a {} not a checkbox/radio. Run 'snapshot -i' to see element types.",
                                element_ref, el_type
                            )
                        })
                    } else {
                        let checked = el.checked.unwrap_or(false);
                        json!({ "ref": element_ref, "checked": checked, "found": true })
                    }
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.to_string(),
                        session_id: Some(sess.id.to_string()),
                    };
                    json!({
                        "ref": element_ref, "checked": false, "found": false,
                        "message": err.suggestion()
                    })
                }
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_count(&self, request: RpcRequest) -> RpcResponse {
        let filter = ElementFilter {
            role: request.param_str("role"),
            name: request.param_str("name"),
            text: request.param_str("text"),
            placeholder: None,
            focused_only: false,
            exact: false,
        };
        let req_id = request.id;

        self.with_detected_session(&request, |sess, update_warning| {
            let count = filter.count(sess.cached_elements());
            let mut response = json!({ "count": count });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_scroll(&self, request: RpcRequest) -> RpcResponse {
        let direction = match request.require_str("direction") {
            Ok(d) => d,
            Err(resp) => return resp,
        };
        let amount = request.param_u64("amount", 5) as usize;
        let session_id = request.param_str("session");

        let key_seq: &[u8] = match direction {
            "up" => ansi_keys::UP,
            "down" => ansi_keys::DOWN,
            "left" => ansi_keys::LEFT,
            "right" => ansi_keys::RIGHT,
            _ => {
                return RpcResponse::error(
                    request.id,
                    -32602,
                    "Invalid direction. Use: up, down, left, right.",
                );
            }
        };

        let req_id = request.id;
        self.with_session(&request, session_id, |sess| {
            for _ in 0..amount {
                if let Err(e) = sess.pty_write(key_seq) {
                    let err = DomainError::PtyError {
                        operation: "scroll".to_string(),
                        reason: e.to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
            }
            RpcResponse::success(
                req_id,
                json!({ "success": true, "direction": direction, "amount": amount }),
            )
        })
    }

    fn handle_scroll_into_view(&self, request: RpcRequest) -> RpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => return RpcResponse::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return RpcResponse::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());
        let max_scrolls = 50;

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                for scroll_count in 0..max_scrolls {
                    let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                        return lock_timeout_response(request.id, session_id);
                    };
                    if let Err(e) = sess.update() {
                        eprintln!(
                            "Warning: Session update failed during scroll_into_view: {}",
                            e
                        );
                    }
                    sess.detect_elements();

                    if sess.find_element(element_ref).is_some() {
                        return RpcResponse::success(
                            request.id,
                            json!({
                                "success": true,
                                "ref": element_ref,
                                "scrolls_needed": scroll_count
                            }),
                        );
                    }

                    if let Err(e) = sess.pty_write(ansi_keys::DOWN) {
                        let err = DomainError::PtyError {
                            operation: "scroll".to_string(),
                            reason: e.to_string(),
                        };
                        return domain_error_response(request.id, &err);
                    }

                    drop(sess);
                    thread::sleep(Duration::from_millis(50));
                }

                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.to_string(),
                    session_id: session_id.map(String::from),
                };
                RpcResponse::success(
                    request.id,
                    json!({
                        "success": false,
                        "message": err.suggestion()
                    }),
                )
            }
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_get_focused(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_detected_session(&request, |sess, update_warning| {
            let elements = sess.cached_elements();
            let mut response = if let Some(focused_el) = elements.iter().find(|e| e.focused) {
                json!({
                    "ref": focused_el.element_ref,
                    "type": focused_el.element_type.as_str(),
                    "label": focused_el.label,
                    "value": focused_el.value,
                    "found": true
                })
            } else {
                json!({
                    "found": false,
                    "message": "No focused element found. Run 'snapshot -i' to see all elements."
                })
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_get_title(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            RpcResponse::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "title": sess.command,
                    "command": sess.command
                }),
            )
        })
    }

    fn handle_focus(&self, request: RpcRequest) -> RpcResponse {
        self.element_action(&request, b"\t")
    }

    fn handle_clear(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.to_string(),
                    session_id: Some(sess.id.to_string()),
                };
                return domain_error_response(req_id, &err);
            }

            if let Err(e) = sess.pty_write(b"\x15") {
                let err = DomainError::PtyError {
                    operation: "clear".to_string(),
                    reason: e.to_string(),
                };
                return domain_error_response(req_id, &err);
            }
            let mut response = json!({ "success": true, "ref": element_ref });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_select_all(&self, request: RpcRequest) -> RpcResponse {
        self.element_action(&request, b"\x01")
    }

    fn handle_toggle(&self, request: RpcRequest) -> RpcResponse {
        let force_state = request.param_bool("state");
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let current_checked = match sess.find_element(element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "checkbox" && el_type != "radio" {
                        let err = DomainError::WrongElementType {
                            element_ref: element_ref.to_string(),
                            actual: el_type.to_string(),
                            expected: "checkbox/radio".to_string(),
                        };
                        return domain_error_response(req_id, &err);
                    }
                    el.checked.unwrap_or(false)
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.to_string(),
                        session_id: Some(sess.id.to_string()),
                    };
                    return domain_error_response(req_id, &err);
                }
            };

            let should_toggle = force_state != Some(current_checked);
            let new_checked = if should_toggle {
                if let Err(e) = sess.pty_write(b" ") {
                    let err = DomainError::PtyError {
                        operation: "toggle".to_string(),
                        reason: e.to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                !current_checked
            } else {
                current_checked
            };

            let mut response =
                json!({ "success": true, "ref": element_ref, "checked": new_checked });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_select(&self, request: RpcRequest) -> RpcResponse {
        let option = match request.require_str("option") {
            Ok(o) => o.to_owned(),
            Err(resp) => return resp,
        };
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            match sess.find_element(element_ref) {
                Some(el) if el.element_type.as_str() != "select" => {
                    let err = DomainError::WrongElementType {
                        element_ref: element_ref.to_string(),
                        actual: el.element_type.as_str().to_string(),
                        expected: "select".to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.to_string(),
                        session_id: Some(sess.id.to_string()),
                    };
                    return domain_error_response(req_id, &err);
                }
                _ => {}
            }

            let screen_text = sess.screen_text();

            let result =
                navigate_to_option(sess, &option, &screen_text).and_then(|_| sess.pty_write(b"\r"));

            if let Err(e) = result {
                let err = DomainError::PtyError {
                    operation: "select".to_string(),
                    reason: e.to_string(),
                };
                return domain_error_response(req_id, &err);
            }

            let mut response = json!({ "success": true, "ref": element_ref, "option": option });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_multiselect(&self, request: RpcRequest) -> RpcResponse {
        let options: Vec<String> = match request.require_array("options") {
            Ok(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            Err(resp) => return resp,
        };
        if options.is_empty() {
            return RpcResponse::error(request.id, -32602, "Options array cannot be empty");
        }
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.to_string(),
                    session_id: Some(sess.id.to_string()),
                };
                return RpcResponse::success(
                    req_id,
                    json!({
                        "success": false,
                        "message": err.suggestion(),
                        "selected_options": []
                    }),
                );
            }

            let mut selected = Vec::new();
            for option in &options {
                if let Err(e) = sess.pty_write(option.as_bytes()) {
                    let err = DomainError::PtyError {
                        operation: "multiselect".to_string(),
                        reason: e.to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Err(e) = sess.pty_write(b" ") {
                    let err = DomainError::PtyError {
                        operation: "multiselect".to_string(),
                        reason: e.to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                if let Err(e) = sess.pty_write(&[0x15]) {
                    let err = DomainError::PtyError {
                        operation: "multiselect".to_string(),
                        reason: e.to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                selected.push(option.clone());
            }

            if let Err(e) = sess.pty_write(b"\r") {
                let err = DomainError::PtyError {
                    operation: "multiselect".to_string(),
                    reason: e.to_string(),
                };
                return domain_error_response(req_id, &err);
            }

            let mut response =
                json!({ "success": true, "ref": element_ref, "selected_options": selected });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        })
    }

    fn handle_attach(&self, request: RpcRequest) -> RpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => return RpcResponse::error(request.id, -32602, "Missing params"),
        };

        let session_id = match params.get("session").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return RpcResponse::error(request.id, -32602, "Missing 'session' param"),
        };

        match self.session_manager.set_active(session_id) {
            Ok(()) => RpcResponse::success(
                request.id,
                json!({
                    "success": true,
                    "session_id": session_id,
                    "message": format!("Now attached to session {}", session_id)
                }),
            ),
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_record_start(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            sess.start_recording();
            RpcResponse::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": sess.id,
                    "recording": true
                }),
            )
        })
    }

    fn handle_record_stop(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        let format = request.param_str("format").unwrap_or("json");

        self.with_session(&request, session_id, |sess| {
            let frames = sess.stop_recording();
            let (cols, rows) = sess.size();

            let data = if format == "asciicast" {
                build_asciicast(sess.id.as_str(), cols, rows, &frames)
            } else {
                json!({
                    "format": "json",
                    "frames": frames.iter().map(|f| json!({
                        "timestamp_ms": f.timestamp_ms,
                        "screen": f.screen
                    })).collect::<Vec<_>>()
                })
            };

            RpcResponse::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": sess.id,
                    "frame_count": frames.len(),
                    "data": data
                }),
            )
        })
    }

    fn handle_record_status(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            let status = sess.recording_status();
            RpcResponse::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "recording": status.is_recording,
                    "frame_count": status.frame_count,
                    "duration_ms": status.duration_ms
                }),
            )
        })
    }

    fn handle_trace(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        let start = request.param_bool("start").unwrap_or(false);
        let stop = request.param_bool("stop").unwrap_or(false);
        let count = request.param_u64("count", 10) as usize;

        self.with_session(&request, session_id, |sess| {
            if start {
                sess.start_trace();
                return RpcResponse::success(
                    req_id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "tracing": true
                    }),
                );
            }

            if stop {
                sess.stop_trace();
                return RpcResponse::success(
                    req_id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "tracing": false
                    }),
                );
            }

            let entries = sess.get_trace_entries(count);
            RpcResponse::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "tracing": sess.is_tracing(),
                    "entries": entries.iter().map(|e| json!({
                        "timestamp_ms": e.timestamp_ms,
                        "action": e.action,
                        "details": e.details
                    })).collect::<Vec<_>>()
                }),
            )
        })
    }

    fn handle_console(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let params = request.params.as_ref().cloned().unwrap_or(json!({}));
        let session_id = request.param_str("session");
        let line_count = params
            .get("count")
            .or_else(|| params.get("lines"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;
        let clear = request.param_bool("clear").unwrap_or(false);

        self.with_session(&request, session_id, |sess| {
            if let Err(e) = sess.update() {
                eprintln!("Warning: Session update failed during console: {}", e);
            }
            let screen = sess.screen_text();

            let all_lines: Vec<&str> = screen.lines().collect();
            let start = all_lines.len().saturating_sub(line_count);
            let output_lines: Vec<&str> = all_lines[start..].to_vec();

            let mut result = json!({
                "session_id": sess.id,
                "lines": output_lines,
                "total_lines": all_lines.len()
            });

            if clear {
                sess.clear_console();
                result["cleared"] = json!(true);
            }

            RpcResponse::success(req_id, result)
        })
    }

    fn handle_errors(&self, request: RpcRequest) -> RpcResponse {
        let req_id = request.id;
        let session_id = request.param_str("session");
        let count = request.param_u64("count", 50) as usize;
        let clear = request.param_bool("clear").unwrap_or(false);

        self.with_session(&request, session_id, |sess| {
            let errors = sess.get_errors(count);
            let total = sess.error_count();

            let mut result = json!({
                "session_id": sess.id,
                "errors": errors.iter().map(|e| json!({
                    "timestamp": e.timestamp,
                    "message": e.message,
                    "source": e.source
                })).collect::<Vec<_>>(),
                "total_count": total
            });

            if clear {
                sess.clear_errors();
                result["cleared"] = json!(true);
            }

            RpcResponse::success(req_id, result)
        })
    }

    fn handle_pty_read(&self, request: RpcRequest) -> RpcResponse {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let session_id = match request.require_str("session") {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        let timeout_ms = request.param_i32("timeout_ms", 50);

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    self.metrics.record_lock_timeout();
                    return lock_timeout_response(request.id, Some(session_id));
                };
                let mut buf = [0u8; 4096];
                match sess.pty_try_read(&mut buf, timeout_ms) {
                    Ok(n) => {
                        let data = STANDARD.encode(&buf[..n]);
                        RpcResponse::success(
                            request.id,
                            json!({
                                "session_id": session_id,
                                "data": data,
                                "bytes_read": n
                            }),
                        )
                    }
                    Err(e) => {
                        let err = DomainError::PtyError {
                            operation: "read".to_string(),
                            reason: e.to_string(),
                        };
                        domain_error_response(request.id, &err)
                    }
                }
            }
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_pty_write(&self, request: RpcRequest) -> RpcResponse {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let session_id = match request.require_str("session") {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        let data_b64 = match request.require_str("data") {
            Ok(d) => d,
            Err(resp) => return resp,
        };
        let data = match STANDARD.decode(data_b64) {
            Ok(d) => d,
            Err(_) => return RpcResponse::error(request.id, -32602, "Invalid base64 data"),
        };

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    self.metrics.record_lock_timeout();
                    return lock_timeout_response(request.id, Some(session_id));
                };
                match sess.pty_write(&data) {
                    Ok(()) => RpcResponse::success(
                        request.id,
                        json!({
                            "success": true,
                            "session_id": session_id
                        }),
                    ),
                    Err(e) => {
                        let err = DomainError::PtyError {
                            operation: "write".to_string(),
                            reason: e.to_string(),
                        };
                        domain_error_response(request.id, &err)
                    }
                }
            }
            Err(e) => domain_error_response(request.id, &DomainError::from(e)),
        }
    }

    fn handle_client(&self, stream: UnixStream) {
        let idle_timeout = DaemonConfig::from_env().idle_timeout;

        if let Err(e) = stream.set_read_timeout(Some(idle_timeout)) {
            eprintln!("Failed to set read timeout: {}", e);
            return;
        }

        if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(30))) {
            eprintln!("Failed to set write timeout: {}", e);
            return;
        }

        let reader_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to clone stream for reading: {}", e);
                return;
            }
        };
        let reader = SizeLimitedReader::new(BufReader::new(reader_stream), MAX_REQUEST_SIZE);
        let mut writer = stream;

        for line_result in reader {
            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::InvalidData {
                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32700,
                                "message": "Parse error: request size limit exceeded (1MB max)"
                            }
                        });
                        let _ = writeln!(writer, "{}", error_response);
                    } else if e.kind() != std::io::ErrorKind::UnexpectedEof
                        && e.kind() != std::io::ErrorKind::WouldBlock
                    {
                        eprintln!("Client connection error: {}", e);
                    }
                    break;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            let request: RpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    self.metrics.record_error();
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e)
                        }
                    });
                    let _ = writeln!(writer, "{}", error_response);
                    continue;
                }
            };

            self.metrics.record_request();
            let response = self.handle_request(request);
            let response_json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to serialize response: {}", e);

                    r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal error: failed to serialize response"}}"#.to_string()
                }
            };

            if let Err(e) = writeln!(writer, "{}", response_json) {
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    eprintln!("Client write error: {}", e);
                }
                break;
            }
        }
    }
}

pub fn start_daemon() -> Result<(), DaemonError> {
    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| DaemonError::LockFailed(format!("failed to open lock file: {}", e)))?;

    let fd = lock_file.as_raw_fd();

    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(DaemonError::AlreadyRunning);
    }

    use std::io::Write as _;
    lock_file
        .set_len(0)
        .map_err(|e| DaemonError::LockFailed(format!("failed to truncate lock file: {}", e)))?;
    let mut lock_file = lock_file;
    writeln!(lock_file, "{}", std::process::id())
        .map_err(|e| DaemonError::LockFailed(format!("failed to write PID to lock file: {}", e)))?;

    if socket_path.exists() {
        std::fs::remove_file(&socket_path).map_err(|e| {
            DaemonError::SocketBind(format!("failed to remove stale socket: {}", e))
        })?;
    }

    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| DaemonError::SocketBind(format!("failed to bind socket: {}", e)))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| DaemonError::SocketBind(format!("failed to set non-blocking: {}", e)))?;

    eprintln!("agent-tui daemon started on {}", socket_path.display());
    eprintln!("PID: {}", std::process::id());

    let shutdown = Arc::new(AtomicBool::new(false));
    let config = DaemonConfig::from_env();
    let server = Arc::new(DaemonServer::with_shutdown_and_config(
        Arc::clone(&shutdown),
        config,
    ));

    let mut signals =
        Signals::new([SIGINT, SIGTERM]).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;
    let shutdown_signal = Arc::clone(&shutdown);
    thread::Builder::new()
        .name("signal-handler".to_string())
        .spawn(move || {
            if let Some(sig) = signals.forever().next() {
                eprintln!("\nReceived signal {}, initiating graceful shutdown...", sig);
                shutdown_signal.store(true, Ordering::SeqCst);
            }
        })
        .map_err(|e| DaemonError::SignalSetup(format!("failed to spawn signal handler: {}", e)))?;

    let pool = ThreadPool::new(MAX_CONNECTIONS, Arc::clone(&server), Arc::clone(&shutdown))
        .map_err(|e| DaemonError::ThreadPool(e.to_string()))?;

    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(stream) = pool.execute(stream) {
                    eprintln!("Thread pool channel closed, dropping connection");
                    drop(stream);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                if !shutdown.load(Ordering::Relaxed) {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    eprintln!("Shutting down daemon...");

    eprintln!(
        "Waiting for {} active connections to complete...",
        server.active_connections.load(Ordering::Relaxed)
    );
    let shutdown_deadline = Instant::now() + Duration::from_secs(5);
    while server.active_connections.load(Ordering::Relaxed) > 0 {
        if Instant::now() > shutdown_deadline {
            eprintln!("Shutdown timeout, forcing close");
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    eprintln!("Cleaning up sessions...");
    server.shutdown_all_sessions();

    eprintln!("Stopping thread pool...");
    pool.shutdown();

    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    if lock_path.exists() {
        let _ = std::fs::remove_file(&lock_path);
    }

    eprintln!("Daemon shutdown complete");
    Ok(())
}

fn element_to_json(el: &Element) -> Value {
    json!({
        "ref": el.element_ref,
        "type": el.element_type.as_str(),
        "label": el.label,
        "value": el.value,
        "position": {
            "row": el.position.row,
            "col": el.position.col,
            "width": el.position.width,
            "height": el.position.height
        },
        "focused": el.focused,
        "selected": el.selected,
        "checked": el.checked,
        "disabled": el.disabled,
        "hint": el.hint
    })
}

fn vom_component_to_json(comp: &Component, index: usize) -> Value {
    json!({
        "ref": format!("@e{}", index + 1),
        "type": comp.role.to_string(),
        "label": comp.text_content.trim(),
        "value": null,
        "position": {
            "row": comp.bounds.y,
            "col": comp.bounds.x,
            "width": comp.bounds.width,
            "height": comp.bounds.height
        },
        "focused": false,
        "selected": false,
        "checked": null,
        "disabled": false,
        "hint": null,
        "vom_id": comp.id.to_string(),
        "visual_hash": comp.visual_hash
    })
}

fn filter_interactive_components(
    vom_components: &[Component],
    screen: &str,
) -> (Option<Vec<Value>>, Value) {
    let elements_total = vom_components.len();

    let interactive: Vec<_> = vom_components
        .iter()
        .filter(|c| c.role.is_interactive())
        .collect();
    let elements_interactive = interactive.len();

    let filtered_elements: Vec<_> = interactive
        .iter()
        .enumerate()
        .map(|(i, comp)| vom_component_to_json(comp, i))
        .collect();

    let elements_shown = filtered_elements.len();

    (
        Some(filtered_elements),
        json!({
            "lines": screen.lines().count(),
            "chars": screen.len(),
            "elements_total": elements_total,
            "elements_interactive": elements_interactive,
            "elements_shown": elements_shown,
            "detection": "vom"
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::DaemonServer;
    use super::SizeLimitedReader;
    use super::combine_warnings;
    use std::fs::OpenOptions;
    use std::io::{BufReader, Write};
    use std::os::unix::io::AsRawFd;
    use std::sync::atomic::Ordering;
    use tempfile::tempdir;

    #[test]
    fn test_combine_warnings_both_some() {
        let result = combine_warnings(
            Some("First warning".to_string()),
            Some("Second warning".to_string()),
        );
        assert_eq!(result, Some("First warning. Second warning".to_string()));
    }

    #[test]
    fn test_combine_warnings_first_only() {
        let result = combine_warnings(Some("Only warning".to_string()), None);
        assert_eq!(result, Some("Only warning".to_string()));
    }

    #[test]
    fn test_combine_warnings_second_only() {
        let result = combine_warnings(None, Some("Only warning".to_string()));
        assert_eq!(result, Some("Only warning".to_string()));
    }

    #[test]
    fn test_combine_warnings_none() {
        let result = combine_warnings(None, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_daemon_singleton_lock() {
        let tmp_dir = tempdir().expect("Failed to create temp dir");
        let lock_path = tmp_dir.path().join("agent-tui.lock");

        let lock_file1 = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .expect("Failed to create lock file");

        let fd1 = lock_file1.as_raw_fd();
        let result1 = unsafe { libc::flock(fd1, libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(result1, 0, "First lock acquisition should succeed");

        let mut lock_file1 = lock_file1;
        writeln!(lock_file1, "{}", std::process::id()).expect("Failed to write PID");

        let lock_file2 = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .expect("Failed to open lock file");

        let fd2 = lock_file2.as_raw_fd();
        let result2 = unsafe { libc::flock(fd2, libc::LOCK_EX | libc::LOCK_NB) };
        assert_ne!(result2, 0, "Second lock acquisition should fail");

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        assert!(
            errno == libc::EWOULDBLOCK || errno == libc::EAGAIN,
            "Expected EWOULDBLOCK or EAGAIN, got errno {}",
            errno
        );
    }

    #[test]
    fn test_daemon_lock_released_on_drop() {
        let tmp_dir = tempdir().expect("Failed to create temp dir");
        let lock_path = tmp_dir.path().join("agent-tui.lock");

        {
            let lock_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&lock_path)
                .expect("Failed to create lock file");

            let fd = lock_file.as_raw_fd();
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            assert_eq!(result, 0, "Lock acquisition should succeed");
        }

        let lock_file2 = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .expect("Failed to open lock file");

        let fd2 = lock_file2.as_raw_fd();
        let result2 = unsafe { libc::flock(fd2, libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(
            result2, 0,
            "Lock should be available after first holder dropped"
        );
    }

    #[test]
    fn test_size_limited_reader_within_limit() {
        use std::io::Cursor;
        let data = "hello\nworld\n";
        let reader = BufReader::new(Cursor::new(data));
        let limited = SizeLimitedReader::new(reader, 100);
        let lines: Vec<_> = limited.collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_ref().unwrap(), "hello");
        assert_eq!(lines[1].as_ref().unwrap(), "world");
    }

    #[test]
    fn test_size_limited_reader_exceeds_limit() {
        use std::io::Cursor;
        let data = "this is a very long line that exceeds the limit\n";
        let reader = BufReader::new(Cursor::new(data));
        let limited = SizeLimitedReader::new(reader, 10);
        let lines: Vec<_> = limited.collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_err());
        let err = lines[0].as_ref().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_size_limited_reader_empty_input() {
        use std::io::Cursor;
        let data = "";
        let reader = BufReader::new(Cursor::new(data));
        let limited = SizeLimitedReader::new(reader, 100);
        let lines: Vec<_> = limited.collect();
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_daemon_server_new() {
        let server = DaemonServer::new();
        assert_eq!(server.active_connections.load(Ordering::Relaxed), 0);
        assert_eq!(server.metrics.requests(), 0);
        assert_eq!(server.metrics.errors(), 0);
    }

    #[test]
    fn test_size_limited_reader_at_exact_limit() {
        use std::io::Cursor;
        // Line of exactly 10 bytes including newline
        let data = "123456789\n";
        assert_eq!(data.len(), 10);
        let reader = BufReader::new(Cursor::new(data));
        let limited = SizeLimitedReader::new(reader, 10);
        let lines: Vec<_> = limited.collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].as_ref().unwrap(), "123456789");
    }

    #[test]
    fn test_element_filter_matches_all_criteria() {
        use super::ElementFilter;
        use agent_tui_core::{Element, ElementType, Position};

        let element = Element {
            element_ref: "@btn1".to_string(),
            element_type: ElementType::Button,
            label: Some("Submit".to_string()),
            value: Some("click me".to_string()),
            hint: Some("Press to submit".to_string()),
            position: Position {
                row: 0,
                col: 0,
                width: Some(10),
                height: Some(1),
            },
            focused: true,
            selected: false,
            checked: None,
            disabled: None,
        };

        // Filter matching all criteria
        let filter = ElementFilter {
            role: Some("button"),
            name: Some("Submit"),
            text: Some("click"),
            placeholder: Some("submit"),
            focused_only: true,
            exact: false,
        };
        assert!(filter.matches(&element));

        // Filter with wrong role should not match
        let filter_wrong_role = ElementFilter {
            role: Some("input"),
            name: None,
            text: None,
            placeholder: None,
            focused_only: false,
            exact: false,
        };
        assert!(!filter_wrong_role.matches(&element));

        // Filter with focused_only on non-focused element
        let unfocused_element = Element {
            focused: false,
            ..element.clone()
        };
        let filter_focused = ElementFilter {
            role: None,
            name: None,
            text: None,
            placeholder: None,
            focused_only: true,
            exact: false,
        };
        assert!(!filter_focused.matches(&unfocused_element));

        // Exact matching
        let filter_exact = ElementFilter {
            role: None,
            name: Some("Submit"),
            text: None,
            placeholder: None,
            focused_only: false,
            exact: true,
        };
        assert!(filter_exact.matches(&element));

        // Exact matching fails on partial
        let filter_exact_partial = ElementFilter {
            role: None,
            name: Some("Sub"),
            text: None,
            placeholder: None,
            focused_only: false,
            exact: true,
        };
        assert!(!filter_exact_partial.matches(&element));
    }
}
