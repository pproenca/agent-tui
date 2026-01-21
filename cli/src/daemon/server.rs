//! JSON-RPC server over Unix socket

use crate::detection::{detect_framework, Element, Framework};
use crate::session::{Session, SessionManager};
use crate::wait::{check_condition, StableTracker, WaitCondition};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// AI-Friendly Error Messages
// ============================================================================

/// Convert technical errors into AI-friendly messages with actionable hints.
/// This follows agent-browser's pattern of always suggesting a next action.
fn ai_friendly_error(error: &str, context: Option<&str>) -> String {
    let ctx = context.unwrap_or("unknown");

    // Element not found errors
    if error.contains("not found") || error.contains("NotFound") {
        return format!(
            "Element \"{}\" not found. Run 'snapshot -i' to see current elements and their refs.",
            ctx
        );
    }

    // Session not found errors
    if error.contains("Session not found") || error.contains("No active session") {
        return "No active session. Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one.".to_string();
    }

    // Timeout errors
    if error.contains("timeout") || error.contains("Timeout") {
        return "Timeout waiting for condition. The app may still be loading. Try 'wait --stable' or increase timeout with '-t'.".to_string();
    }

    // Lock acquisition errors
    if error.contains("lock") {
        return "Session is busy. Try again in a moment, or run 'sessions' to check session status.".to_string();
    }

    // Invalid key errors
    if error.contains("Invalid key") {
        return format!(
            "Invalid key '{}'. Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+",
            ctx
        );
    }

    // Element type mismatch
    if error.contains("not toggleable") || error.contains("not a select") {
        return format!(
            "Element {} cannot perform this action. Run 'snapshot -i' to see element types.",
            ctx
        );
    }

    // PTY errors
    if error.contains("PTY") || error.contains("Pty") {
        return "Terminal communication error. The session may have ended. Run 'sessions' to check status.".to_string();
    }

    // Default: return original error with generic hint
    format!("{}. Run 'snapshot -i' to see current screen state.", error)
}

/// Try to acquire a session lock with timeout and exponential backoff
/// Returns None if unable to acquire lock within timeout
///
/// Uses exponential backoff starting at 100µs, doubling up to 50ms max,
/// to reduce CPU usage during contention while staying responsive.
fn acquire_session_lock(
    session: &Arc<Mutex<Session>>,
    timeout: Duration,
) -> Option<MutexGuard<'_, Session>> {
    let start = Instant::now();
    let mut backoff = Duration::from_micros(100);
    const MAX_BACKOFF: Duration = Duration::from_millis(50);

    while start.elapsed() < timeout {
        if let Ok(guard) = session.try_lock() {
            return Some(guard);
        }
        thread::sleep(backoff);
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
    None
}

/// Default lock timeout for handlers
const LOCK_TIMEOUT: Duration = Duration::from_secs(5);

/// Create a standardized lock timeout error response
fn lock_timeout_response(request_id: u64, session_context: Option<&str>) -> Response {
    let msg = match session_context {
        Some(sid) => format!(
            "Session '{}': {}",
            sid,
            ai_friendly_error("lock timeout", None)
        ),
        None => ai_friendly_error("lock timeout", None),
    };
    Response::error(request_id, -32000, &msg)
}

// ============================================================================
// Select Widget Navigation Helpers
// ============================================================================

/// Arrow key escape sequences
const ARROW_UP: &[u8] = b"\x1b[A";
const ARROW_DOWN: &[u8] = b"\x1b[B";

/// Navigate to a select option using arrow keys
fn navigate_to_option(
    sess: &mut Session,
    target: &str,
    screen_text: &str,
) -> Result<(), crate::session::SessionError> {
    let (options, current_idx) = parse_select_options(screen_text);

    // Find target option (case-insensitive partial match)
    let target_lower = target.to_lowercase();
    let target_idx = options
        .iter()
        .position(|opt| opt.to_lowercase().contains(&target_lower))
        .unwrap_or(0);

    // Calculate steps and direction
    let steps = target_idx as i32 - current_idx as i32;
    let key = if steps > 0 { ARROW_DOWN } else { ARROW_UP };

    // Send arrow keys with small delay for TUI to update
    for _ in 0..steps.unsigned_abs() {
        sess.pty_write(key)?;
        thread::sleep(Duration::from_millis(30));
    }

    Ok(())
}

/// Parse select options from screen text
/// Returns (options, currently_selected_index)
fn parse_select_options(screen_text: &str) -> (Vec<String>, usize) {
    let mut options = Vec::new();
    let mut selected_idx = 0;

    for line in screen_text.lines() {
        let trimmed = line.trim();

        // Ink/Inquirer selection markers: ❯ or ›
        if trimmed.starts_with('❯') || trimmed.starts_with('›') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['❯', '›', ' ']).to_string());
        }
        // Inquirer radio buttons: ◉ (selected) or ◯ (unselected)
        else if trimmed.starts_with('◉') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['◉', ' ']).to_string());
        } else if trimmed.starts_with('◯') {
            options.push(trimmed.trim_start_matches(['◯', ' ']).to_string());
        }
        // BubbleTea/generic: > marker (but not >>)
        else if trimmed.starts_with('>') && !trimmed.starts_with(">>") {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['>', ' ']).to_string());
        }
    }

    (options, selected_idx)
}

/// Get the socket path
pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("agent-tui.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/agent-tui.sock"))
}

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC sequence - consume until we hit the terminator
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // CSI sequence: consume until we hit a letter (@ through ~)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == '~' || next == '@' {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                // OSC sequence: consume until BEL (\x07) or ST (\x1b\\)
                chars.next(); // consume ']'
                while let Some(&next) = chars.peek() {
                    if next == '\x07' {
                        chars.next();
                        break;
                    } else if next == '\x1b' {
                        chars.next();
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                    chars.next();
                }
            } else {
                // Other escape sequences - consume one more char
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// JSON-RPC request
#[derive(Debug, Deserialize)]
struct Request {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl Response {
    fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: u64, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

pub struct DaemonServer {
    session_manager: Arc<SessionManager>,
    start_time: Instant,
}

impl Default for DaemonServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonServer {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            start_time: Instant::now(),
        }
    }

    /// Handle a JSON-RPC request
    fn handle_request(&self, request: Request) -> Response {
        match request.method.as_str() {
            "ping" => Response::success(request.id, json!({ "pong": true })),

            "health" => {
                let uptime_ms = self.start_time.elapsed().as_millis() as u64;
                Response::success(
                    request.id,
                    json!({
                        "status": "healthy",
                        "pid": std::process::id(),
                        "uptime_ms": uptime_ms,
                        "session_count": self.session_manager.session_count(),
                        "version": env!("CARGO_PKG_VERSION")
                    }),
                )
            }

            "spawn" => self.handle_spawn(request),
            "snapshot" => self.handle_snapshot(request),
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
            "screen" => self.handle_screen(request),
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

            _ => Response::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_spawn(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
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

        match self
            .session_manager
            .spawn(command, &args, cwd, None, session_id, cols, rows)
        {
            Ok((session_id, pid)) => Response::success(
                request.id,
                json!({
                    "session_id": session_id,
                    "pid": pid
                }),
            ),
            Err(e) => {
                let err_str = e.to_string();
                let friendly_msg = if err_str.contains("No such file")
                    || err_str.contains("not found")
                {
                    format!(
                        "Failed to spawn '{}': Command not found. Check if the command exists and is in PATH.",
                        command
                    )
                } else if err_str.contains("Permission denied") {
                    format!(
                        "Failed to spawn '{}': Permission denied. Check file permissions.",
                        command
                    )
                } else {
                    format!(
                        "Failed to spawn '{}': {}. Run 'sessions' to see active sessions.",
                        command, err_str
                    )
                };
                Response::error(request.id, -32000, &friendly_msg)
            }
        }
    }

    fn handle_snapshot(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let include_elements = params
            .get("include_elements")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let interactive_only = params
            .get("interactive_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let compact = params
            .get("compact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                let _ = sess.update();

                let screen = sess.screen_text();
                let cursor = sess.cursor();
                let (cols, rows) = sess.size();

                // Get and process elements
                let (elements, stats) = if include_elements {
                    let all_elements = sess.detect_elements();
                    let elements_total = all_elements.len();
                    let elements_interactive =
                        all_elements.iter().filter(|e| e.is_interactive()).count();

                    // Apply filtering
                    let filtered_elements: Vec<_> = all_elements
                        .iter()
                        .filter(|el| {
                            // Apply interactive-only filter
                            if interactive_only && !el.is_interactive() {
                                return false;
                            }
                            // Apply compact filter (keep interactive OR has content)
                            if compact && !el.is_interactive() && !el.has_content() {
                                return false;
                            }
                            true
                        })
                        .map(element_to_json)
                        .collect();

                    let elements_shown = filtered_elements.len();

                    (
                        Some(filtered_elements),
                        json!({
                            "lines": screen.lines().count(),
                            "chars": screen.len(),
                            "elements_total": elements_total,
                            "elements_interactive": elements_interactive,
                            "elements_shown": elements_shown
                        }),
                    )
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

                Response::success(
                    request.id,
                    json!({
                        "session_id": sess.id,
                        "screen": screen,
                        "elements": elements,
                        "cursor": {
                            "row": cursor.row,
                            "col": cursor.col,
                            "visible": cursor.visible
                        },
                        "size": {
                            "cols": cols,
                            "rows": rows
                        },
                        "stats": stats
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_click(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                match sess.click(element_ref) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_dbl_click(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                // First click - acquire lock, click, then release
                {
                    let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error("lock timeout", None),
                        );
                    };

                    if let Err(e) = sess.click(element_ref) {
                        return Response::success(
                            request.id,
                            json!({
                                "success": false,
                                "message": ai_friendly_error(&e.to_string(), Some(element_ref))
                            }),
                        );
                    }
                    // Lock released here
                }

                // Small delay between clicks - no lock held during sleep
                thread::sleep(Duration::from_millis(50));

                // Second click - reacquire lock
                {
                    let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error("lock timeout (second click)", None),
                        );
                    };

                    match sess.click(element_ref) {
                        Ok(()) => Response::success(request.id, json!({ "success": true })),
                        Err(e) => Response::success(
                            request.id,
                            json!({
                                "success": false,
                                "message": ai_friendly_error(&e.to_string(), Some(element_ref))
                            }),
                        ),
                    }
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_fill(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let value = match params.get("value").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return Response::error(request.id, -32602, "Missing 'value' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                match sess.fill(element_ref, value) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_keystroke(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => return Response::error(request.id, -32602, "Missing 'key' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                match sess.keystroke(key) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), Some(key))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_keydown(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => return Response::error(request.id, -32602, "Missing 'key' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                match sess.keydown(key) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), Some(key))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_keyup(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => return Response::error(request.id, -32602, "Missing 'key' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                match sess.keyup(key) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), Some(key))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_type(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Response::error(request.id, -32602, "Missing 'text' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                match sess.type_text(text) {
                    Ok(()) => Response::success(request.id, json!({ "success": true })),
                    Err(e) => Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error(&e.to_string(), None)
                        }),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_wait(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let text = params.get("text").and_then(|v| v.as_str());
        let condition_str = params.get("condition").and_then(|v| v.as_str());
        let target = params.get("target").and_then(|v| v.as_str());
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        // Parse the wait condition
        let condition = match WaitCondition::parse(condition_str, target, text) {
            Some(c) => c,
            None => {
                // If no valid condition, require at least text
                if text.is_none() {
                    return Response::error(
                        request.id,
                        -32602,
                        "Missing condition: provide 'text' or 'condition' with 'target'",
                    );
                }
                WaitCondition::Text(text.unwrap().to_string())
            }
        };

        let session = match self.session_manager.resolve(session_id) {
            Ok(s) => s,
            Err(e) => {
                return Response::error(
                    request.id,
                    -32000,
                    &ai_friendly_error(&e.to_string(), session_id),
                )
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
                    // Capture match details based on condition type
                    match &condition {
                        WaitCondition::Text(t) => {
                            matched_text = Some(t.clone());
                        }
                        WaitCondition::Element(e) => {
                            element_ref = Some(e.clone());
                        }
                        WaitCondition::Focused(e) => {
                            element_ref = Some(e.clone());
                        }
                        WaitCondition::Value { element, expected } => {
                            element_ref = Some(element.clone());
                            matched_text = Some(expected.clone());
                        }
                        _ => {}
                    }
                    break;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        // Build response with additional context on timeout
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
        } else {
            // On timeout, provide helpful context
            if let Some(sess) = acquire_session_lock(&session, Duration::from_millis(100)) {
                let screen = sess.screen_text();
                // Truncate screen context to first 200 chars
                let screen_preview: String = screen.chars().take(200).collect();
                let screen_context = if screen.len() > 200 {
                    format!("{}...", screen_preview)
                } else {
                    screen_preview
                };
                response["screen_context"] = json!(screen_context);

                // Generate helpful suggestion based on condition
                let suggestion = match &condition {
                    WaitCondition::Text(t) => {
                        format!(
                            "Text '{}' not found. Check if the app finished loading or try 'snapshot -i' to see current screen.",
                            t
                        )
                    }
                    WaitCondition::Element(e) => {
                        format!(
                            "Element {} not found. Try 'snapshot -i' to see available elements.",
                            e
                        )
                    }
                    WaitCondition::Focused(e) => {
                        format!(
                            "Element {} exists but is not focused. Try 'click {}' to focus it.",
                            e, e
                        )
                    }
                    WaitCondition::NotVisible(e) => {
                        format!(
                            "Element {} is still visible. The app may still be processing.",
                            e
                        )
                    }
                    WaitCondition::Stable => {
                        "Screen is still changing. The app may have animations or be loading."
                            .to_string()
                    }
                    WaitCondition::TextGone(t) => {
                        format!(
                            "Text '{}' is still visible. The operation may not have completed.",
                            t
                        )
                    }
                    WaitCondition::Value { element, expected } => {
                        format!(
                            "Element {} does not have value '{}'. Check if input was accepted.",
                            element, expected
                        )
                    }
                };
                response["suggestion"] = json!(suggestion);
            }
        }

        Response::success(request.id, response)
    }

    fn handle_kill(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        let session_to_kill = match session_id {
            Some(id) => id.to_string(),
            None => match self.session_manager.active_session_id() {
                Some(id) => id,
                None => return Response::error(request.id, -32000, "No active session"),
            },
        };

        match self.session_manager.kill(&session_to_kill) {
            Ok(()) => Response::success(
                request.id,
                json!({
                    "success": true,
                    "session_id": session_to_kill
                }),
            ),
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(&session_to_kill)),
            ),
        }
    }

    fn handle_restart(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        // Get the session info before killing
        let (old_session_id, command, cols, rows) = match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let (cols, rows) = sess.size();
                (sess.id.clone(), sess.command.clone(), cols, rows)
            }
            Err(e) => {
                return Response::error(
                    request.id,
                    -32000,
                    &ai_friendly_error(&e.to_string(), session_id),
                )
            }
        };

        // Kill the old session
        if let Err(e) = self.session_manager.kill(&old_session_id) {
            return Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(&old_session_id)),
            );
        }

        // Spawn a new session with the same command
        match self
            .session_manager
            .spawn(&command, &[], None, None, None, cols, rows)
        {
            Ok((new_session_id, pid)) => Response::success(
                request.id,
                json!({
                    "success": true,
                    "old_session_id": old_session_id,
                    "new_session_id": new_session_id,
                    "command": command,
                    "pid": pid
                }),
            ),
            Err(e) => Response::error(
                request.id,
                -32000,
                &format!(
                    "Killed session {} but failed to respawn '{}': {}",
                    old_session_id, command, e
                ),
            ),
        }
    }

    fn handle_sessions(&self, request: Request) -> Response {
        let sessions = self.session_manager.list();
        let active_id = self.session_manager.active_session_id();

        Response::success(
            request.id,
            json!({
                "sessions": sessions.iter().map(|s| json!({
                    "id": s.id,
                    "command": s.command,
                    "pid": s.pid,
                    "running": s.running,
                    "created_at": s.created_at,
                    "size": { "cols": s.size.0, "rows": s.size.1 }
                })).collect::<Vec<_>>(),
                "active_session": active_id
            }),
        )
    }

    fn handle_resize(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;
        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                match sess.resize(cols, rows) {
                    Ok(()) => Response::success(
                        request.id,
                        json!({
                            "success": true,
                            "session_id": sess.id,
                            "size": { "cols": cols, "rows": rows }
                        }),
                    ),
                    Err(e) => Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), session_id),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_screen(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let strip_ansi = params
            .get("strip_ansi")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_cursor = params
            .get("include_cursor")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                let mut screen = sess.screen_text();
                let (cols, rows) = sess.size();
                let cursor = sess.cursor();

                if strip_ansi {
                    screen = strip_ansi_codes(&screen);
                }

                let mut result = json!({
                    "session_id": sess.id,
                    "screen": screen,
                    "size": { "cols": cols, "rows": rows }
                });

                if include_cursor {
                    result["cursor"] = json!({
                        "row": cursor.row,
                        "col": cursor.col,
                        "visible": cursor.visible
                    });
                }

                Response::success(request.id, result)
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_find(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let role = params.get("role").and_then(|v| v.as_str());
        let name = params.get("name").and_then(|v| v.as_str());
        let text = params.get("text").and_then(|v| v.as_str());
        let placeholder = params.get("placeholder").and_then(|v| v.as_str());
        let focused_only = params
            .get("focused")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let nth = params
            .get("nth")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        let exact = params
            .get("exact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                let elements = sess.detect_elements();

                let matches: Vec<_> = elements
                    .iter()
                    .filter(|el| {
                        // Filter by role if specified
                        if let Some(r) = role {
                            if el.element_type.as_str() != r {
                                return false;
                            }
                        }
                        // Filter by name/label if specified
                        if let Some(n) = name {
                            let matches = if exact {
                                el.label.as_ref().map(|l| l == n).unwrap_or(false)
                            } else {
                                // Case-insensitive substring match by default
                                let n_lower = n.to_lowercase();
                                el.label
                                    .as_ref()
                                    .map(|l| l.to_lowercase().contains(&n_lower))
                                    .unwrap_or(false)
                            };
                            if !matches {
                                return false;
                            }
                        }
                        // Filter by text content (label or value)
                        if let Some(t) = text {
                            let in_label = if exact {
                                el.label.as_ref().map(|l| l == t).unwrap_or(false)
                            } else {
                                // Case-insensitive substring match by default
                                let t_lower = t.to_lowercase();
                                el.label
                                    .as_ref()
                                    .map(|l| l.to_lowercase().contains(&t_lower))
                                    .unwrap_or(false)
                            };
                            let in_value = if exact {
                                el.value.as_ref().map(|v| v == t).unwrap_or(false)
                            } else {
                                // Case-insensitive substring match by default
                                let t_lower = t.to_lowercase();
                                el.value
                                    .as_ref()
                                    .map(|v| v.to_lowercase().contains(&t_lower))
                                    .unwrap_or(false)
                            };
                            if !in_label && !in_value {
                                return false;
                            }
                        }
                        // Filter by placeholder (maps to internal 'hint' field)
                        if let Some(p) = placeholder {
                            let matches = if exact {
                                el.hint.as_ref().map(|h| h == p).unwrap_or(false)
                            } else {
                                // Case-insensitive substring match by default
                                let p_lower = p.to_lowercase();
                                el.hint
                                    .as_ref()
                                    .map(|h| h.to_lowercase().contains(&p_lower))
                                    .unwrap_or(false)
                            };
                            if !matches {
                                return false;
                            }
                        }
                        // Filter by focused state
                        if focused_only && !el.focused {
                            return false;
                        }
                        true
                    })
                    .map(element_to_json)
                    .collect();

                // Apply nth filter if specified
                let final_matches = if let Some(n) = nth {
                    if n < matches.len() {
                        vec![matches[n].clone()]
                    } else {
                        vec![]
                    }
                } else {
                    matches
                };

                Response::success(
                    request.id,
                    json!({
                        "session_id": sess.id,
                        "elements": final_matches,
                        "count": final_matches.len()
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_get_text(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                let _ = sess.update();
                sess.detect_elements();

                match sess.find_element(element_ref) {
                    Some(el) => {
                        let text = el.label.clone().or_else(|| el.value.clone());
                        Response::success(
                            request.id,
                            json!({
                                "ref": element_ref,
                                "text": text,
                                "found": true
                            }),
                        )
                    }
                    None => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "text": null,
                            "found": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_get_value(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                match sess.find_element(element_ref) {
                    Some(el) => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "value": el.value,
                            "found": true
                        }),
                    ),
                    None => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "value": null,
                            "found": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_is_visible(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                let visible = sess.find_element(element_ref).is_some();
                Response::success(
                    request.id,
                    json!({
                        "ref": element_ref,
                        "visible": visible
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_is_focused(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                match sess.find_element(element_ref) {
                    Some(el) => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "focused": el.focused,
                            "found": true
                        }),
                    ),
                    None => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "focused": false,
                            "found": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_is_enabled(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                match sess.find_element(element_ref) {
                    Some(el) => {
                        let enabled = !el.disabled.unwrap_or(false);
                        Response::success(
                            request.id,
                            json!({
                                "ref": element_ref,
                                "enabled": enabled,
                                "found": true
                            }),
                        )
                    }
                    None => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "enabled": false,
                            "found": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_is_checked(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                match sess.find_element(element_ref) {
                    Some(el) => {
                        let el_type = el.element_type.as_str();
                        if el_type != "checkbox" && el_type != "radio" {
                            return Response::success(
                                request.id,
                                json!({
                                    "ref": element_ref,
                                    "checked": false,
                                    "found": true,
                                    "message": format!(
                                        "Element {} is a {} not a checkbox/radio. Run 'snapshot -i' to see element types.",
                                        element_ref, el_type
                                    )
                                }),
                            );
                        }
                        let checked = el.checked.unwrap_or(false);
                        Response::success(
                            request.id,
                            json!({
                                "ref": element_ref,
                                "checked": checked,
                                "found": true
                            }),
                        )
                    }
                    None => Response::success(
                        request.id,
                        json!({
                            "ref": element_ref,
                            "checked": false,
                            "found": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_count(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let role = params.get("role").and_then(|v| v.as_str());
        let name = params.get("name").and_then(|v| v.as_str());
        let text = params.get("text").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                let elements = sess.detect_elements();

                let count = elements
                    .iter()
                    .filter(|el| {
                        // Filter by role if specified
                        if let Some(r) = role {
                            if el.element_type.as_str() != r {
                                return false;
                            }
                        }
                        // Filter by name/label if specified
                        if let Some(n) = name {
                            if !el.label.as_ref().map(|l| l.contains(n)).unwrap_or(false) {
                                return false;
                            }
                        }
                        // Filter by text content (label or value)
                        if let Some(t) = text {
                            let in_label =
                                el.label.as_ref().map(|l| l.contains(t)).unwrap_or(false);
                            let in_value =
                                el.value.as_ref().map(|v| v.contains(t)).unwrap_or(false);
                            if !in_label && !in_value {
                                return false;
                            }
                        }
                        true
                    })
                    .count();

                Response::success(
                    request.id,
                    json!({
                        "count": count
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_scroll(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let direction = match params.get("direction").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return Response::error(request.id, -32602, "Missing 'direction' param"),
        };

        let amount = params.get("amount").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let session_id = params.get("session").and_then(|v| v.as_str());

        // Determine the escape sequence for the direction
        let key_seq: &[u8] = match direction {
            "up" => b"\x1b[A",    // Arrow up
            "down" => b"\x1b[B",  // Arrow down
            "left" => b"\x1b[D",  // Arrow left
            "right" => b"\x1b[C", // Arrow right
            _ => {
                return Response::error(
                    request.id,
                    -32602,
                    "Invalid direction. Use: up, down, left, right.",
                )
            }
        };

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

                // Send the key sequence 'amount' times
                for _ in 0..amount {
                    if let Err(e) = sess.pty_write(key_seq) {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "direction": direction,
                        "amount": amount
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    /// Scroll until an element is visible on screen (agent-browser parity)
    fn handle_scroll_into_view(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());
        let max_scrolls = 50; // Safety limit

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                for scroll_count in 0..max_scrolls {
                    let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error("lock timeout", None),
                        );
                    };
                    let _ = sess.update();
                    sess.detect_elements();

                    // Check if element is now visible
                    if sess.find_element(element_ref).is_some() {
                        return Response::success(
                            request.id,
                            json!({
                                "success": true,
                                "ref": element_ref,
                                "scrolls_needed": scroll_count
                            }),
                        );
                    }

                    // Try scrolling down (most common direction for finding elements)
                    if let Err(e) = sess.pty_write(b"\x1b[B") {
                        // Arrow down
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }

                    // Small delay to let the UI update
                    drop(sess);
                    thread::sleep(Duration::from_millis(50));
                }

                // Element not found after max scrolls
                Response::success(
                    request.id,
                    json!({
                        "success": false,
                        "message": ai_friendly_error("Element not found after scrolling", Some(element_ref))
                    }),
                )
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    /// Get the currently focused element (agent-browser parity)
    fn handle_get_focused(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                let _ = sess.update();
                let elements = sess.detect_elements();

                // Find the focused element
                if let Some(focused_el) = elements.iter().find(|e| e.focused) {
                    Response::success(
                        request.id,
                        json!({
                            "ref": focused_el.element_ref,
                            "type": focused_el.element_type.as_str(),
                            "label": focused_el.label,
                            "value": focused_el.value,
                            "found": true
                        }),
                    )
                } else {
                    Response::success(
                        request.id,
                        json!({
                            "found": false,
                            "message": "No focused element found. Run 'snapshot -i' to see all elements."
                        }),
                    )
                }
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    /// Get the session title/command (agent-browser parity)
    fn handle_get_title(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };

                Response::success(
                    request.id,
                    json!({
                        "session_id": sess.id,
                        "title": sess.command,
                        "command": sess.command
                    }),
                )
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_focus(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                let _ = sess.update();
                sess.detect_elements();

                // Check if element exists
                if sess.find_element(element_ref).is_none() {
                    return Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    );
                }

                // Send Tab to navigate (basic implementation)
                if let Err(e) = sess.pty_write(b"\t") {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref
                    }),
                )
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_clear(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                // Check if element exists
                if sess.find_element(element_ref).is_none() {
                    return Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    );
                }

                // Framework-aware clear: use appropriate sequence based on detected TUI framework
                let screen_text = sess.screen_text();
                let framework = detect_framework(&screen_text);

                let result = match framework {
                    Framework::Textual => {
                        // Textual (Python) needs Ctrl+A (select all) then Delete
                        sess.pty_write(b"\x01")
                            .and_then(|_| sess.pty_write(b"\x7f"))
                    }
                    _ => {
                        // Ctrl+U works for readline, Ink, Inquirer, BubbleTea, etc.
                        sess.pty_write(b"\x15")
                    }
                };

                if let Err(e) = result {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_select_all(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                // Check if element exists
                if sess.find_element(element_ref).is_none() {
                    return Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref))
                        }),
                    );
                }

                // Send Ctrl+A to select all
                if let Err(e) = sess.pty_write(b"\x01") {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_toggle(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());
        let force_state = params.get("state").and_then(|v| v.as_bool());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error("lock timeout", None),
                    );
                };
                let _ = sess.update();
                sess.detect_elements();

                // Check if element exists and is toggleable
                let current_checked = match sess.find_element(element_ref) {
                    Some(el) => {
                        let el_type = el.element_type.as_str();
                        if el_type != "checkbox" && el_type != "radio" {
                            return Response::success(
                                request.id,
                                json!({
                                    "success": false,
                                    "message": format!(
                                        "Element {} is a {} not a checkbox/radio. Run 'snapshot -i' to see element types.",
                                        element_ref, el_type
                                    )
                                }),
                            );
                        }
                        el.checked.unwrap_or(false)
                    }
                    None => {
                        return Response::success(
                            request.id,
                            json!({
                                "success": false,
                                "message": ai_friendly_error("Element not found", Some(element_ref))
                            }),
                        );
                    }
                };

                // Determine if we need to toggle based on force_state
                let should_toggle = match force_state {
                    Some(desired_state) => desired_state != current_checked,
                    None => true, // Always toggle if no state specified
                };

                let new_checked = if should_toggle {
                    // Send Space to toggle
                    if let Err(e) = sess.pty_write(b" ") {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }
                    !current_checked
                } else {
                    current_checked
                };

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref,
                        "checked": new_checked
                    }),
                )
            }
            Err(e) => Response::error(request.id, -32000, &ai_friendly_error(&e.to_string(), None)),
        }
    }

    fn handle_select(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let option = match params.get("option").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return Response::error(request.id, -32602, "Missing 'option' param"),
        };

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                // Check if element exists and is a select
                match sess.find_element(element_ref) {
                    Some(el) => {
                        if el.element_type.as_str() != "select" {
                            return Response::success(
                                request.id,
                                json!({
                                    "success": false,
                                    "message": ai_friendly_error("Element is not a select", Some(element_ref))
                                }),
                            );
                        }
                    }
                    None => {
                        return Response::success(
                            request.id,
                            json!({
                                "success": false,
                                "message": ai_friendly_error("Element not found", Some(element_ref))
                            }),
                        );
                    }
                }

                // Framework-aware select: use arrow navigation for known TUI frameworks
                let screen_text = sess.screen_text();
                let framework = detect_framework(&screen_text);

                let result = match framework {
                    Framework::Unknown => {
                        // Fallback: type to filter + Enter (current behavior)
                        sess.pty_write(b"\r")
                            .and_then(|_| sess.pty_write(option.as_bytes()))
                            .and_then(|_| sess.pty_write(b"\r"))
                    }
                    _ => {
                        // Arrow navigation for known TUI frameworks (Ink, Inquirer, BubbleTea, etc.)
                        navigate_to_option(&mut sess, option, &screen_text)
                            .and_then(|_| sess.pty_write(b"\r"))
                    }
                };

                if let Err(e) = result {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref,
                        "option": option
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_multiselect(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let element_ref = match params.get("ref").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Response::error(request.id, -32602, "Missing 'ref' param"),
        };

        let options: Vec<String> = match params.get("options").and_then(|v| v.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            None => return Response::error(request.id, -32602, "Missing 'options' param"),
        };

        if options.is_empty() {
            return Response::error(request.id, -32602, "Options array cannot be empty");
        }

        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                sess.detect_elements();

                // Verify element exists
                if sess.find_element(element_ref).is_none() {
                    return Response::success(
                        request.id,
                        json!({
                            "success": false,
                            "message": ai_friendly_error("Element not found", Some(element_ref)),
                            "selected_options": []
                        }),
                    );
                }

                // Multi-select in TUI typically involves:
                // 1. Focus the list element
                // 2. Navigate with arrow keys
                // 3. Press Space to toggle selection
                // For each option:
                // - Type to filter/search
                // - Press Space to select
                // - Continue for each option

                let mut selected = Vec::new();
                for option in &options {
                    // Type to filter to the option
                    if let Err(e) = sess.pty_write(option.as_bytes()) {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }

                    // Small delay for TUI to filter
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    // Press Space to toggle selection
                    if let Err(e) = sess.pty_write(b" ") {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }

                    // Clear the filter (Ctrl+U typically clears input)
                    if let Err(e) = sess.pty_write(&[0x15]) {
                        // Ctrl+U
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }

                    selected.push(option.clone());
                }

                // Confirm selection with Enter
                if let Err(e) = sess.pty_write(b"\r") {
                    return Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "ref": element_ref,
                        "selected_options": selected
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_attach(&self, request: Request) -> Response {
        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let session_id = match params.get("session").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Response::error(request.id, -32602, "Missing 'session' param"),
        };

        match self.session_manager.set_active(session_id) {
            Ok(()) => Response::success(
                request.id,
                json!({
                    "success": true,
                    "session_id": session_id,
                    "message": format!("Now attached to session {}", session_id)
                }),
            ),
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(session_id)),
            ),
        }
    }

    fn handle_record_start(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

                sess.start_recording();

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "recording": true
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_record_stop(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

                let frames = sess.stop_recording();

                let data = if format == "asciicast" {
                    // Convert to asciicast v2 format
                    // Spec: https://github.com/asciinema/asciinema/blob/develop/doc/asciicast-v2.md
                    let (cols, rows) = sess.size();
                    let mut output = Vec::new();

                    // Calculate total duration
                    let duration = if !frames.is_empty() {
                        frames
                            .last()
                            .map(|f| f.timestamp_ms as f64 / 1000.0)
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };

                    // Header (first line, required fields: version, width, height)
                    let header = json!({
                        "version": 2,
                        "width": cols,
                        "height": rows,
                        "timestamp": chrono::Utc::now().timestamp(),
                        "duration": duration,
                        "title": format!("agent-tui recording - {}", sess.id),
                        "env": {
                            "TERM": "xterm-256color",
                            "SHELL": std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
                        }
                    });
                    output.push(serde_json::to_string(&header).unwrap());

                    // Event stream (each line is a JSON array: [time, event_type, data])
                    // time: seconds since recording start (float)
                    // event_type: "o" for output (terminal writes), "i" for input (user types)
                    // data: string data
                    let mut prev_screen = String::new();
                    for frame in &frames {
                        // For asciicast, we should output the diff or full screen
                        // Using full screen for simplicity, but could optimize with diffs
                        let time_secs = frame.timestamp_ms as f64 / 1000.0;

                        // Only output if screen changed (avoid duplicate frames)
                        if frame.screen != prev_screen {
                            // Calculate the diff - for now, output full screen with clear
                            // In a real implementation, we'd compute the actual terminal output
                            let screen_data = if prev_screen.is_empty() {
                                frame.screen.clone()
                            } else {
                                // Clear screen and redraw
                                format!("\x1b[2J\x1b[H{}", frame.screen)
                            };

                            let event = json!([time_secs, "o", screen_data]);
                            output.push(serde_json::to_string(&event).unwrap());
                            prev_screen = frame.screen.clone();
                        }
                    }

                    json!({
                        "format": "asciicast",
                        "version": 2,
                        "data": output.join("\n")
                    })
                } else {
                    json!({
                        "format": "json",
                        "frames": frames.iter().map(|f| json!({
                            "timestamp_ms": f.timestamp_ms,
                            "screen": f.screen
                        })).collect::<Vec<_>>()
                    })
                };

                Response::success(
                    request.id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "frame_count": frames.len(),
                        "data": data
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_record_status(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

                let status = sess.recording_status();

                Response::success(
                    request.id,
                    json!({
                        "session_id": sess.id,
                        "recording": status.is_recording,
                        "frame_count": status.frame_count,
                        "duration_ms": status.duration_ms
                    }),
                )
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_trace(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let start = params
            .get("start")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let stop = params
            .get("stop")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

                if start {
                    sess.start_trace();
                    return Response::success(
                        request.id,
                        json!({
                            "success": true,
                            "session_id": sess.id,
                            "tracing": true
                        }),
                    );
                }

                if stop {
                    sess.stop_trace();
                    return Response::success(
                        request.id,
                        json!({
                            "success": true,
                            "session_id": sess.id,
                            "tracing": false
                        }),
                    );
                }

                // Return recent trace entries
                let entries = sess.get_trace_entries(count);
                Response::success(
                    request.id,
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
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_console(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let lines = params
            .get("count")
            .or_else(|| params.get("lines"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;
        let clear = params
            .get("clear")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                let _ = sess.update();
                let screen = sess.screen_text();

                // Get the last N lines
                let all_lines: Vec<&str> = screen.lines().collect();
                let start = if all_lines.len() > lines {
                    all_lines.len() - lines
                } else {
                    0
                };
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

                Response::success(request.id, result)
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_errors(&self, request: Request) -> Response {
        let params = request.params.unwrap_or(json!({}));
        let session_id = params.get("session").and_then(|v| v.as_str());
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let clear = params
            .get("clear")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };

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

                Response::success(request.id, result)
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn handle_pty_read(&self, request: Request) -> Response {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let session_id = match params.get("session").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Response::error(request.id, -32602, "Missing 'session' param"),
        };

        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as i32;

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let sess = session.lock().unwrap();
                let mut buf = [0u8; 4096];
                match sess.pty_try_read(&mut buf, timeout_ms) {
                    Ok(n) => {
                        let data = STANDARD.encode(&buf[..n]);
                        Response::success(
                            request.id,
                            json!({
                                "session_id": session_id,
                                "data": data,
                                "bytes_read": n
                            }),
                        )
                    }
                    Err(e) => Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), Some(session_id)),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(session_id)),
            ),
        }
    }

    fn handle_pty_write(&self, request: Request) -> Response {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let params = match request.params {
            Some(p) => p,
            None => return Response::error(request.id, -32602, "Missing params"),
        };

        let session_id = match params.get("session").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Response::error(request.id, -32602, "Missing 'session' param"),
        };

        let data_b64 = match params.get("data").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return Response::error(request.id, -32602, "Missing 'data' param"),
        };

        let data = match STANDARD.decode(data_b64) {
            Ok(d) => d,
            Err(_) => return Response::error(request.id, -32602, "Invalid base64 data"),
        };

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let sess = session.lock().unwrap();
                match sess.pty_write(&data) {
                    Ok(()) => Response::success(
                        request.id,
                        json!({
                            "success": true,
                            "session_id": session_id
                        }),
                    ),
                    Err(e) => Response::error(
                        request.id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), Some(session_id)),
                    ),
                }
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(session_id)),
            ),
        }
    }

    /// Handle a client connection
    fn handle_client(&self, stream: UnixStream) {
        let reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if line.trim().is_empty() {
                continue;
            }

            let request: Request = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
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

            let response = self.handle_request(request);
            let response_json = serde_json::to_string(&response).unwrap();

            if writeln!(writer, "{}", response_json).is_err() {
                break;
            }
        }
    }
}

/// Start the daemon server
pub fn start_daemon() -> std::io::Result<()> {
    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    // Acquire exclusive lock to ensure singleton daemon
    // Note: Don't truncate on open - we only truncate after acquiring the lock
    // to avoid clearing another daemon's PID if the lock fails
    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    let fd = lock_file.as_raw_fd();
    // SAFETY: flock is a standard POSIX call, fd is valid from open()
    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "Another daemon instance is running",
        ));
    }

    // Write PID to lock file for debugging (truncate first to clear old content)
    use std::io::Write as _;
    lock_file.set_len(0)?;
    let mut lock_file = lock_file;
    writeln!(lock_file, "{}", std::process::id())?;

    // Now safe to remove stale socket - we hold the exclusive lock
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    eprintln!("agent-tui daemon started on {}", socket_path.display());
    eprintln!("PID: {}", std::process::id());

    let server = Arc::new(DaemonServer::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                thread::spawn(move || {
                    server.handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use tempfile::tempdir;

    /// Test that file lock prevents second daemon from starting
    #[test]
    fn test_daemon_singleton_lock() {
        let tmp_dir = tempdir().expect("Failed to create temp dir");
        let lock_path = tmp_dir.path().join("agent-tui.lock");

        // First "daemon" acquires the lock
        let lock_file1 = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .expect("Failed to create lock file");

        let fd1 = lock_file1.as_raw_fd();
        let result1 = unsafe { libc::flock(fd1, libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(result1, 0, "First lock acquisition should succeed");

        // Write PID to lock file
        let mut lock_file1 = lock_file1;
        writeln!(lock_file1, "{}", std::process::id()).expect("Failed to write PID");

        // Second "daemon" tries to acquire the same lock - should fail
        let lock_file2 = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .expect("Failed to open lock file");

        let fd2 = lock_file2.as_raw_fd();
        let result2 = unsafe { libc::flock(fd2, libc::LOCK_EX | libc::LOCK_NB) };
        assert_ne!(result2, 0, "Second lock acquisition should fail");

        // Verify the error is EWOULDBLOCK
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        assert!(
            errno == libc::EWOULDBLOCK || errno == libc::EAGAIN,
            "Expected EWOULDBLOCK or EAGAIN, got errno {}",
            errno
        );
    }

    /// Test that lock is released when file handle is dropped
    #[test]
    fn test_daemon_lock_released_on_drop() {
        let tmp_dir = tempdir().expect("Failed to create temp dir");
        let lock_path = tmp_dir.path().join("agent-tui.lock");

        // Acquire lock in a scope so it gets dropped
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
            // lock_file dropped here, releasing the lock
        }

        // Now another process should be able to acquire the lock
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
}
