use agent_tui_common::ValueExt;
use agent_tui_core::Element;
use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::{Value, json};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::common::{domain_error_response, lock_timeout_response};
use crate::ansi_keys;
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::select_helpers::{navigate_to_option, strip_ansi_codes};
use crate::session::{Session, SessionManager};

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

fn combine_warnings(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(format!("{}. {}", x, y)),
        (w @ Some(_), None) | (None, w @ Some(_)) => w,
        (None, None) => None,
    }
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

fn matches_text(haystack: Option<&String>, needle: &str, exact: bool) -> bool {
    match haystack {
        Some(h) if exact => h == needle,
        Some(h) => h.to_lowercase().contains(&needle.to_lowercase()),
        None => false,
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

pub fn handle_snapshot(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let params = request.params.as_ref().cloned().unwrap_or(json!({}));
    let session_id = request.param_str("session");
    let include_elements = params.bool_or("include_elements", false);
    let should_strip_ansi = params.bool_or("strip_ansi", false);
    let include_cursor = params.bool_or("include_cursor", false);
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

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
            if should_strip_ansi {
                screen = strip_ansi_codes(&screen);
            }

            let mut response = json!({
                "session_id": sess.id,
                "screen": screen
            });

            if include_elements {
                sess.detect_elements();
                let elements: Vec<Value> =
                    sess.cached_elements().iter().map(element_to_json).collect();
                response["elements"] = json!(elements);
            }

            if include_cursor {
                let cursor = sess.cursor();
                response["cursor"] = json!({
                    "row": cursor.row,
                    "col": cursor.col,
                    "visible": cursor.visible
                });
            }

            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }

            RpcResponse::success(req_id, response)
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_click(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
            match sess.click(&element_ref) {
                Ok(()) => RpcResponse::action_success(req_id),
                Err(e) => domain_error_response(req_id, &DomainError::from(e)),
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_dbl_click(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
            if let Err(e) = sess.click(&element_ref) {
                return domain_error_response(req_id, &DomainError::from(e));
            }
            thread::sleep(Duration::from_millis(50));
            match sess.click(&element_ref) {
                Ok(()) => RpcResponse::action_success(req_id),
                Err(e) => domain_error_response(req_id, &DomainError::from(e)),
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_fill(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let value = match request.require_str("value") {
        Ok(v) => v.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            let type_warning = match sess.find_element(&element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "input" {
                        Some(format!(
                            "Warning: '{}' is a {} not an input field. Fill may not work as expected.",
                            element_ref, el_type
                        ))
                    } else {
                        None
                    }
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.clone(),
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_find(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
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
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_count(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let filter = ElementFilter {
        role: request.param_str("role"),
        name: request.param_str("name"),
        text: request.param_str("text"),
        placeholder: None,
        focused_only: false,
        exact: false,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);
            let count = filter.count(sess.cached_elements());
            let mut response = json!({ "count": count });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_scroll(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let direction = match request.require_str("direction") {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    let amount = request.param_u64("amount", 5) as usize;
    let session_id = request.param_str("session");
    let req_id = request.id;

    let key_seq: &[u8] = match direction {
        "up" => ansi_keys::UP,
        "down" => ansi_keys::DOWN,
        "left" => ansi_keys::LEFT,
        "right" => ansi_keys::RIGHT,
        _ => {
            return RpcResponse::error(
                req_id,
                -32602,
                "Invalid direction. Use: up, down, left, right.",
            );
        }
    };

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_get_text(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_property(session_manager, &request, "text", |el| {
        el.label.clone().or_else(|| el.value.clone())
    })
}

pub fn handle_get_value(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_property(session_manager, &request, "value", |el| el.value.clone())
}

pub fn handle_is_visible(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);
            let visible = sess.find_element(&element_ref).is_some();
            let mut response = json!({ "ref": element_ref, "visible": visible });
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_is_focused(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_property(session_manager, &request, "focused", |el| el.focused)
}

pub fn handle_is_enabled(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_property(session_manager, &request, "enabled", |el| {
        !el.disabled.unwrap_or(false)
    })
}

pub fn handle_is_checked(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            let mut response = match sess.find_element(&element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "checkbox" && el_type != "radio" {
                        json!({
                            "ref": element_ref, "checked": false, "found": true,
                            "message": format!(
                                "Element {} is a {} not a checkbox/radio.",
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
                        element_ref: element_ref.clone(),
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_get_focused(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);
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
                    "message": "No focused element found."
                })
            };
            if let Some(warning) = update_warning {
                response["warning"] = json!(warning);
            }
            RpcResponse::success(req_id, response)
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_get_title(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
            RpcResponse::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "title": sess.command,
                    "command": sess.command
                }),
            )
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_focus(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_action(session_manager, &request, b"\t")
}

pub fn handle_clear(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_action(session_manager, &request, b"\x15")
}

pub fn handle_select_all(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_action(session_manager, &request, b"\x01")
}

pub fn handle_toggle(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let force_state = request.param_bool("state");
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            let current_checked = match sess.find_element(&element_ref) {
                Some(el) => {
                    let el_type = el.element_type.as_str();
                    if el_type != "checkbox" && el_type != "radio" {
                        let err = DomainError::WrongElementType {
                            element_ref: element_ref.clone(),
                            actual: el_type.to_string(),
                            expected: "checkbox/radio".to_string(),
                        };
                        return domain_error_response(req_id, &err);
                    }
                    el.checked.unwrap_or(false)
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.clone(),
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_select(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let option = match request.require_str("option") {
        Ok(o) => o.to_owned(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            match sess.find_element(&element_ref) {
                Some(el) if el.element_type.as_str() != "select" => {
                    let err = DomainError::WrongElementType {
                        element_ref: element_ref.clone(),
                        actual: el.element_type.as_str().to_string(),
                        expected: "select".to_string(),
                    };
                    return domain_error_response(req_id, &err);
                }
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.clone(),
                        session_id: Some(sess.id.to_string()),
                    };
                    return domain_error_response(req_id, &err);
                }
                _ => {}
            }

            let screen_text = sess.screen_text();
            let result = navigate_to_option(&mut sess, &option, &screen_text)
                .and_then(|_| sess.pty_write(b"\r"));

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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

fn element_property<F, T>(
    session_manager: &Arc<SessionManager>,
    request: &RpcRequest,
    field_name: &str,
    extract: F,
) -> RpcResponse
where
    F: FnOnce(&Element) -> T,
    T: serde::Serialize,
{
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            let mut response = match sess.find_element(&element_ref) {
                Some(el) => json!({
                    "ref": element_ref,
                    field_name: extract(el),
                    "found": true
                }),
                None => {
                    let err = DomainError::ElementNotFound {
                        element_ref: element_ref.clone(),
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

fn element_action(
    session_manager: &Arc<SessionManager>,
    request: &RpcRequest,
    pty_bytes: &[u8],
) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            if sess.find_element(&element_ref).is_none() {
                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.clone(),
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_scroll_into_view(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let max_scrolls = 50;
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            for scroll_count in 0..max_scrolls {
                let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                    return lock_timeout_response(req_id, session_id);
                };
                if let Err(e) = sess.update() {
                    eprintln!(
                        "Warning: Session update failed during scroll_into_view: {}",
                        e
                    );
                }
                sess.detect_elements();

                if sess.find_element(&element_ref).is_some() {
                    return RpcResponse::success(
                        req_id,
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
                    return domain_error_response(req_id, &err);
                }

                drop(sess);
                thread::sleep(Duration::from_millis(50));
            }

            let err = DomainError::ElementNotFound {
                element_ref: element_ref.clone(),
                session_id: session_id.map(String::from),
            };
            RpcResponse::success(
                req_id,
                json!({
                    "success": false,
                    "message": err.suggestion()
                }),
            )
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_multiselect(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
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
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let update_warning = update_with_warning(&mut sess);

            if sess.find_element(&element_ref).is_none() {
                let err = DomainError::ElementNotFound {
                    element_ref: element_ref.clone(),
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
                thread::sleep(Duration::from_millis(50));
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
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}
