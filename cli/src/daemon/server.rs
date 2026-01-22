use super::error_messages::{ai_friendly_error, lock_timeout_response};
use super::lock_helpers::{acquire_session_lock, LOCK_TIMEOUT};
use super::rpc_types::{Request, Response};
use super::select_helpers::{navigate_to_option, strip_ansi_codes};
use crate::json_ext::ValueExt;
use crate::session::{Element, SessionManager};
use crate::sync_utils::mutex_lock_or_recover;
use crate::wait::{check_condition, StableTracker, WaitCondition};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("agent-tui.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/agent-tui.sock"))
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
            let matches = if self.exact {
                el.label.as_ref().map(|l| l == n).unwrap_or(false)
            } else {
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
        if let Some(t) = self.text {
            let in_label = if self.exact {
                el.label.as_ref().map(|l| l == t).unwrap_or(false)
            } else {
                let t_lower = t.to_lowercase();
                el.label
                    .as_ref()
                    .map(|l| l.to_lowercase().contains(&t_lower))
                    .unwrap_or(false)
            };
            let in_value = if self.exact {
                el.value.as_ref().map(|v| v == t).unwrap_or(false)
            } else {
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
        if let Some(p) = self.placeholder {
            let matches = if self.exact {
                el.hint.as_ref().map(|h| h == p).unwrap_or(false)
            } else {
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
        if self.focused_only && !el.focused {
            return false;
        }
        true
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

    fn with_session<F>(&self, request: &Request, session_id: Option<&str>, f: F) -> Response
    where
        F: FnOnce(&mut crate::session::Session) -> Response,
    {
        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                f(&mut sess)
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn with_session_and_ref<F>(&self, request: &Request, f: F) -> Response
    where
        F: FnOnce(&mut crate::session::Session, &str) -> Response,
    {
        let element_ref = match request.require_str("ref") {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        let session_id = request.param_str("session");

        match self.session_manager.resolve(session_id) {
            Ok(session) => {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(request.id, session_id);
                };
                f(&mut sess, element_ref)
            }
            Err(e) => Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), session_id),
            ),
        }
    }

    fn with_detected_session_and_ref<F>(&self, request: &Request, f: F) -> Response
    where
        F: FnOnce(&mut crate::session::Session, &str, Option<String>) -> Response,
    {
        self.with_session_and_ref(request, |sess, element_ref| {
            let update_warning = match sess.update() {
                Ok(()) => None,
                Err(e) => {
                    eprintln!("Warning: Session update failed: {}", e);
                    Some(format!("Element data may be stale: {}", e))
                }
            };
            sess.detect_elements();
            f(sess, element_ref, update_warning)
        })
    }

    fn with_detected_session<F>(&self, request: &Request, f: F) -> Response
    where
        F: FnOnce(&mut crate::session::Session, Option<String>) -> Response,
    {
        let session_id = request.param_str("session");
        self.with_session(request, session_id, |sess| {
            let update_warning = match sess.update() {
                Ok(()) => None,
                Err(e) => {
                    eprintln!("Warning: Session update failed: {}", e);
                    Some(format!("Element data may be stale: {}", e))
                }
            };
            sess.detect_elements();
            f(sess, update_warning)
        })
    }

    fn with_session_action<F>(&self, request: &Request, param: &str, f: F) -> Response
    where
        F: FnOnce(&mut crate::session::Session, &str) -> Result<(), Box<dyn std::error::Error>>,
    {
        let req_id = request.id;
        let value = match request.require_str(param) {
            Ok(v) => v.to_string(),
            Err(resp) => return resp,
        };
        let session_id = request.param_str("session");
        self.with_session(request, session_id, |sess| match f(sess, &value) {
            Ok(()) => Response::action_success(req_id),
            Err(e) => Response::action_failed(req_id, Some(&value), &e.to_string()),
        })
    }

    #[allow(clippy::result_large_err)]
    fn with_resolved_session(
        &self,
        request: &Request,
    ) -> Result<(Arc<std::sync::Mutex<crate::session::Session>>, String), Response> {
        let element_ref = match request.require_str("ref") {
            Ok(r) => r.to_owned(),
            Err(resp) => return Err(resp),
        };
        let session_id = request.param_str("session");
        match self.session_manager.resolve(session_id) {
            Ok(s) => Ok((s, element_ref)),
            Err(e) => Err(Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), None),
            )),
        }
    }

    fn element_property<F, T>(&self, request: &Request, field_name: &str, extract: F) -> Response
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
                None => json!({
                    "ref": element_ref,
                    field_name: serde_json::Value::Null,
                    "found": false,
                    "message": ai_friendly_error("Element not found", Some(element_ref))
                }),
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn element_action(&self, request: &Request, pty_bytes: &[u8]) -> Response {
        let req_id = request.id;
        self.with_detected_session_and_ref(request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                return Response::element_not_found(req_id, element_ref);
            }
            if let Err(e) = sess.pty_write(pty_bytes) {
                return Response::error(req_id, -32000, &ai_friendly_error(&e.to_string(), None));
            }
            let mut response = json!({ "success": true, "ref": element_ref });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

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

            "screen" => Response::error(
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

            Response::success(req_id, response)
        })
    }

    fn handle_click(&self, request: Request) -> Response {
        let req_id = request.id;
        self.with_session_and_ref(&request, |sess, element_ref| {
            match sess.click(element_ref) {
                Ok(()) => Response::action_success(req_id),
                Err(e) => Response::action_failed(req_id, Some(element_ref), &e.to_string()),
            }
        })
    }

    fn handle_dbl_click(&self, request: Request) -> Response {
        let req_id = request.id;
        let (session, element_ref) = match self.with_resolved_session(&request) {
            Ok(result) => result,
            Err(resp) => return resp,
        };

        {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, request.param_str("session"));
            };
            if let Err(e) = sess.click(&element_ref) {
                return Response::action_failed(req_id, Some(&element_ref), &e.to_string());
            }
        }

        thread::sleep(Duration::from_millis(50));

        {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, request.param_str("session"));
            };
            match sess.click(&element_ref) {
                Ok(()) => Response::action_success(req_id),
                Err(e) => Response::action_failed(req_id, Some(&element_ref), &e.to_string()),
            }
        }
    }

    fn handle_fill(&self, request: Request) -> Response {
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
                    return Response::element_not_found(req_id, element_ref);
                }
            };


            if let Err(e) = sess.pty_write(value.as_bytes()) {
                return Response::action_failed(req_id, Some(element_ref), &e.to_string());
            }

            let mut response = json!({
                "success": true,
                "ref": element_ref,
                "value": value
            });


            let combined_warning = match (update_warning, type_warning) {
                (Some(uw), Some(tw)) => Some(format!("{}. {}", uw, tw)),
                (Some(w), None) | (None, Some(w)) => Some(w),
                (None, None) => None,
            };
            if let Some(warn_msg) = combined_warning {
                response["warning"] = json!(warn_msg);
            }

            Response::success(req_id, response)
        })
    }

    fn handle_keystroke(&self, request: Request) -> Response {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keystroke(key).map_err(|e| e.into())
        })
    }

    fn handle_keydown(&self, request: Request) -> Response {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keydown(key).map_err(|e| e.into())
        })
    }

    fn handle_keyup(&self, request: Request) -> Response {
        self.with_session_action(&request, "key", |sess, key| {
            sess.keyup(key).map_err(|e| e.into())
        })
    }

    fn handle_type(&self, request: Request) -> Response {
        self.with_session_action(&request, "text", |sess, text| {
            sess.type_text(text).map_err(|e| e.into())
        })
    }

    fn handle_wait(&self, request: Request) -> Response {
        let session_id = request.param_str("session");
        let text = request.param_str("text");
        let condition_str = request.param_str("condition");
        let target = request.param_str("target");
        let timeout_ms = request.param_u64("timeout_ms", 30000);

        let condition = match WaitCondition::parse(condition_str, target, text) {
            Some(c) => c,
            None => {
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

        Response::success(request.id, response)
    }

    fn handle_kill(&self, request: Request) -> Response {
        let session_id = request.param_str("session");

        let session_to_kill = match session_id {
            Some(id) => id.to_string(),
            None => match self.session_manager.active_session_id() {
                Some(id) => id.to_string(),
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
                return Response::error(
                    request.id,
                    -32000,
                    &ai_friendly_error(&e.to_string(), session_id),
                )
            }
        };

        if let Err(e) = self.session_manager.kill(&old_session_id) {
            return Response::error(
                request.id,
                -32000,
                &ai_friendly_error(&e.to_string(), Some(&old_session_id)),
            );
        }

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
        let cols = request.param_u16("cols", 80);
        let rows = request.param_u16("rows", 24);
        let session_id = request.param_str("session");

        let req_id = request.id;
        self.with_session(&request, session_id, |sess| match sess.resize(cols, rows) {
            Ok(()) => Response::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": sess.id,
                    "size": { "cols": cols, "rows": rows }
                }),
            ),
            Err(e) => Response::action_failed(req_id, None, &e.to_string()),
        })
    }

    fn handle_find(&self, request: Request) -> Response {
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
            let elements = sess.cached_elements();

            let matches: Vec<_> = elements
                .iter()
                .filter(|el| filter.matches(el))
                .map(element_to_json)
                .collect();

            let final_matches = if let Some(n) = nth {
                if n < matches.len() {
                    vec![matches[n].clone()]
                } else {
                    vec![]
                }
            } else {
                matches
            };

            let mut response = json!({
                "session_id": sess.id,
                "elements": final_matches,
                "count": final_matches.len()
            });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_get_text(&self, request: Request) -> Response {
        self.element_property(&request, "text", |el| {
            el.label.clone().or_else(|| el.value.clone())
        })
    }

    fn handle_get_value(&self, request: Request) -> Response {
        self.element_property(&request, "value", |el| el.value.clone())
    }

    fn handle_is_visible(&self, request: Request) -> Response {
        let req_id = request.id;
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let visible = sess.find_element(element_ref).is_some();
            let mut response = json!({ "ref": element_ref, "visible": visible });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_is_focused(&self, request: Request) -> Response {
        self.element_property(&request, "focused", |el| el.focused)
    }

    fn handle_is_enabled(&self, request: Request) -> Response {
        self.element_property(&request, "enabled", |el| !el.disabled.unwrap_or(false))
    }

    fn handle_is_checked(&self, request: Request) -> Response {
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
                None => json!({
                    "ref": element_ref, "checked": false, "found": false,
                    "message": ai_friendly_error("Element not found", Some(element_ref))
                }),
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_count(&self, request: Request) -> Response {
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
            let elements = sess.cached_elements();
            let count = elements.iter().filter(|el| filter.matches(el)).count();
            let mut response = json!({ "count": count });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_scroll(&self, request: Request) -> Response {
        let direction = match request.require_str("direction") {
            Ok(d) => d,
            Err(resp) => return resp,
        };
        let amount = request.param_u64("amount", 5) as usize;
        let session_id = request.param_str("session");

        let key_seq: &[u8] = match direction {
            "up" => b"\x1b[A",
            "down" => b"\x1b[B",
            "left" => b"\x1b[D",
            "right" => b"\x1b[C",
            _ => {
                return Response::error(
                    request.id,
                    -32602,
                    "Invalid direction. Use: up, down, left, right.",
                )
            }
        };

        let req_id = request.id;
        self.with_session(&request, session_id, |sess| {
            for _ in 0..amount {
                if let Err(e) = sess.pty_write(key_seq) {
                    return Response::action_failed(req_id, None, &e.to_string());
                }
            }
            Response::success(
                req_id,
                json!({ "success": true, "direction": direction, "amount": amount }),
            )
        })
    }

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
                        return Response::success(
                            request.id,
                            json!({
                                "success": true,
                                "ref": element_ref,
                                "scrolls_needed": scroll_count
                            }),
                        );
                    }

                    if let Err(e) = sess.pty_write(b"\x1b[B") {
                        return Response::error(
                            request.id,
                            -32000,
                            &ai_friendly_error(&e.to_string(), None),
                        );
                    }

                    drop(sess);
                    thread::sleep(Duration::from_millis(50));
                }

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

    fn handle_get_focused(&self, request: Request) -> Response {
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
            Response::success(req_id, response)
        })
    }

    fn handle_get_title(&self, request: Request) -> Response {
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            Response::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "title": sess.command,
                    "command": sess.command
                }),
            )
        })
    }

    fn handle_focus(&self, request: Request) -> Response {
        self.element_action(&request, b"\t")
    }

    fn handle_clear(&self, request: Request) -> Response {
        let req_id = request.id;
        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                return Response::element_not_found(req_id, element_ref);
            }

            if let Err(e) = sess.pty_write(b"\x15") {
                return Response::error(req_id, -32000, &ai_friendly_error(&e.to_string(), None));
            }
            let mut response = json!({ "success": true, "ref": element_ref });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_select_all(&self, request: Request) -> Response {
        self.element_action(&request, b"\x01")
    }

    fn handle_toggle(&self, request: Request) -> Response {
        let force_state = request.param_bool("state");
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            let current_checked = match sess.find_element(element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "checkbox" && el_type != "radio" {
                        return Response::wrong_element_type(
                            req_id,
                            element_ref,
                            el_type,
                            "checkbox/radio",
                        );
                    }
                    el.checked.unwrap_or(false)
                }
                None => {
                    return Response::element_not_found(req_id, element_ref);
                }
            };

            let should_toggle = force_state != Some(current_checked);
            let new_checked = if should_toggle {
                if let Err(e) = sess.pty_write(b" ") {
                    return Response::error(
                        req_id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
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
            Response::success(req_id, response)
        })
    }

    fn handle_select(&self, request: Request) -> Response {
        let option = match request.require_str("option") {
            Ok(o) => o.to_owned(),
            Err(resp) => return resp,
        };
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            match sess.find_element(element_ref) {
                Some(el) if el.element_type.as_str() != "select" => {
                    return Response::wrong_element_type(
                        req_id,
                        element_ref,
                        el.element_type.as_str(),
                        "select",
                    );
                }
                None => {
                    return Response::element_not_found(req_id, element_ref);
                }
                _ => {}
            }

            let screen_text = sess.screen_text();

            let result =
                navigate_to_option(sess, &option, &screen_text).and_then(|_| sess.pty_write(b"\r"));

            if let Err(e) = result {
                return Response::error(req_id, -32000, &ai_friendly_error(&e.to_string(), None));
            }

            let mut response = json!({ "success": true, "ref": element_ref, "option": option });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
    }

    fn handle_multiselect(&self, request: Request) -> Response {
        let options: Vec<String> = match request.require_array("options") {
            Ok(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            Err(resp) => return resp,
        };
        if options.is_empty() {
            return Response::error(request.id, -32602, "Options array cannot be empty");
        }
        let req_id = request.id;

        self.with_detected_session_and_ref(&request, |sess, element_ref, update_warning| {
            if sess.find_element(element_ref).is_none() {
                return Response::success(
                    req_id,
                    json!({
                        "success": false,
                        "message": ai_friendly_error("Element not found", Some(element_ref)),
                        "selected_options": []
                    }),
                );
            }

            let mut selected = Vec::new();
            for option in &options {
                if let Err(e) = sess.pty_write(option.as_bytes()) {
                    return Response::error(
                        req_id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Err(e) = sess.pty_write(b" ") {
                    return Response::error(
                        req_id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }
                if let Err(e) = sess.pty_write(&[0x15]) {
                    return Response::error(
                        req_id,
                        -32000,
                        &ai_friendly_error(&e.to_string(), None),
                    );
                }
                selected.push(option.clone());
            }

            if let Err(e) = sess.pty_write(b"\r") {
                return Response::error(req_id, -32000, &ai_friendly_error(&e.to_string(), None));
            }

            let mut response =
                json!({ "success": true, "ref": element_ref, "selected_options": selected });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            Response::success(req_id, response)
        })
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
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            sess.start_recording();
            Response::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": sess.id,
                    "recording": true
                }),
            )
        })
    }

    fn handle_record_stop(&self, request: Request) -> Response {
        let req_id = request.id;
        let session_id = request.param_str("session");
        let format = request.param_str("format").unwrap_or("json");

        self.with_session(&request, session_id, |sess| {
            let frames = sess.stop_recording();

            let data = if format == "asciicast" {
                let (cols, rows) = sess.size();
                let mut output = Vec::new();

                let duration = if !frames.is_empty() {
                    frames
                        .last()
                        .map(|f| f.timestamp_ms as f64 / 1000.0)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

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

                let mut prev_screen = String::new();
                for frame in &frames {
                    let time_secs = frame.timestamp_ms as f64 / 1000.0;

                    if frame.screen != prev_screen {
                        let screen_data = if prev_screen.is_empty() {
                            frame.screen.clone()
                        } else {
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

    fn handle_record_status(&self, request: Request) -> Response {
        let req_id = request.id;
        let session_id = request.param_str("session");
        self.with_session(&request, session_id, |sess| {
            let status = sess.recording_status();
            Response::success(
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

    fn handle_trace(&self, request: Request) -> Response {
        let req_id = request.id;
        let session_id = request.param_str("session");
        let start = request.param_bool("start").unwrap_or(false);
        let stop = request.param_bool("stop").unwrap_or(false);
        let count = request.param_u64("count", 10) as usize;

        self.with_session(&request, session_id, |sess| {
            if start {
                sess.start_trace();
                return Response::success(
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
                return Response::success(
                    req_id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "tracing": false
                    }),
                );
            }

            let entries = sess.get_trace_entries(count);
            Response::success(
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

    fn handle_console(&self, request: Request) -> Response {
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

            Response::success(req_id, result)
        })
    }

    fn handle_errors(&self, request: Request) -> Response {
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

            Response::success(req_id, result)
        })
    }

    fn handle_pty_read(&self, request: Request) -> Response {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let session_id = match request.require_str("session") {
            Ok(id) => id,
            Err(resp) => return resp,
        };
        let timeout_ms = request.param_i32("timeout_ms", 50);

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let sess = mutex_lock_or_recover(&session);
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
            Err(_) => return Response::error(request.id, -32602, "Invalid base64 data"),
        };

        match self.session_manager.get(session_id) {
            Ok(session) => {
                let sess = mutex_lock_or_recover(&session);
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

    fn handle_client(&self, stream: UnixStream) {
        let reader_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to clone stream for reading: {}", e);
                return;
            }
        };
        let reader = BufReader::new(reader_stream);
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
            let response_json = match serde_json::to_string(&response) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to serialize response: {}", e);

                    r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Internal error: failed to serialize response"}}"#.to_string()
                }
            };

            if writeln!(writer, "{}", response_json).is_err() {
                break;
            }
        }
    }
}

pub fn start_daemon() -> std::io::Result<()> {
    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;

    let fd = lock_file.as_raw_fd();

    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "Another daemon instance is running",
        ));
    }

    use std::io::Write as _;
    lock_file.set_len(0)?;
    let mut lock_file = lock_file;
    writeln!(lock_file, "{}", std::process::id())?;

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

fn vom_component_to_json(comp: &crate::vom::Component, index: usize) -> Value {
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

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use tempfile::tempdir;

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
}
