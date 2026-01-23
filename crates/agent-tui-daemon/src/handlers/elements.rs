use agent_tui_common::ValueExt;
use agent_tui_core::Element;
use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::{Value, json};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::common::{domain_error_response, lock_timeout_response, session_error_response};
use crate::adapters::{
    count_output_to_response, element_to_json as adapter_element_to_json, fill_success_response,
    parse_count_input, parse_fill_input, parse_find_input, parse_scroll_input,
    parse_snapshot_input, scroll_output_to_response,
};
use crate::ansi_keys;
use crate::domain::{
    ClearInput, ClickInput, DoubleClickInput, ElementStateInput, FocusInput, MultiselectInput,
    ScrollIntoViewInput, SelectAllInput, SelectInput, ToggleInput,
};
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::select_helpers::{navigate_to_option, strip_ansi_codes};
use crate::session::{Session, SessionManager};
use crate::usecases::{
    ClearUseCase, ClickUseCase, CountUseCase, DoubleClickUseCase, FillUseCase, FindUseCase,
    FocusUseCase, GetFocusedUseCase, GetTextUseCase, GetTitleUseCase, GetValueUseCase,
    IsCheckedUseCase, IsEnabledUseCase, IsFocusedUseCase, IsVisibleUseCase, MultiselectUseCase,
    ScrollIntoViewUseCase, ScrollUseCase, SelectAllUseCase, SelectUseCase, SnapshotUseCase,
    ToggleUseCase,
};

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

/// Handle snapshot requests using the use case pattern.
pub fn handle_snapshot_uc<U: SnapshotUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_snapshot_input(&request);
    let should_strip_ansi = request
        .params
        .as_ref()
        .map(|p| p.bool_or("strip_ansi", false))
        .unwrap_or(false);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => {
            let mut screen = output.screen;
            if should_strip_ansi {
                screen = strip_ansi_codes(&screen);
            }

            let mut response = json!({
                "session_id": output.session_id,
                "screen": screen
            });

            if let Some(elements) = output.elements {
                let elements_json: Vec<Value> = elements.iter().map(element_to_json).collect();
                response["elements"] = json!(elements_json);
            }

            if let Some(cursor) = output.cursor {
                response["cursor"] = json!({
                    "row": cursor.row,
                    "col": cursor.col,
                    "visible": cursor.visible
                });
            }

            RpcResponse::success(req_id, response)
        }
        Err(e) => session_error_response(req_id, e),
    }
}

/// Legacy handle_snapshot using SessionManager directly.
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

/// Handle click requests using the use case pattern.
pub fn handle_click_uc<U: ClickUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ClickInput {
        session_id,
        element_ref,
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Legacy handle_click using SessionManager directly.
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

/// Handle double-click requests using the use case pattern.
pub fn handle_dbl_click_uc<U: DoubleClickUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = DoubleClickInput {
        session_id,
        element_ref,
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
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

/// Handle fill requests using the use case pattern.
pub fn handle_fill_uc<U: FillUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = match parse_fill_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };
    let element_ref = input.element_ref.clone();
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(_output) => fill_success_response(req_id, &element_ref),
        Err(e) => session_error_response(req_id, e),
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

/// Handle find requests using the use case pattern.
pub fn handle_find_uc<U: FindUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_find_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => {
            let elements_json: Vec<Value> = output
                .elements
                .iter()
                .map(adapter_element_to_json)
                .collect();
            RpcResponse::success(
                req_id,
                json!({
                    "elements": elements_json,
                    "count": output.count
                }),
            )
        }
        Err(e) => session_error_response(req_id, e),
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

/// Handle count requests using the use case pattern.
pub fn handle_count_uc<U: CountUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_count_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => count_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
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

/// Handle scroll requests using the use case pattern.
pub fn handle_scroll_uc<U: ScrollUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = match parse_scroll_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };
    let direction = input.direction.clone();
    let amount = input.amount;
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => scroll_output_to_response(req_id, output, &direction, amount),
        Err(e) => session_error_response(req_id, e),
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

/// Handle get_text requests using the use case pattern.
pub fn handle_get_text_uc<U: GetTextUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "ref": element_ref, "text": output.text, "found": output.found }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_get_text(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_property(session_manager, &request, "text", |el| {
        el.label.clone().or_else(|| el.value.clone())
    })
}

/// Handle get_value requests using the use case pattern.
pub fn handle_get_value_uc<U: GetValueUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "ref": element_ref, "value": output.value, "found": output.found }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_get_value(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_property(session_manager, &request, "value", |el| el.value.clone())
}

/// Handle is_visible requests using the use case pattern.
pub fn handle_is_visible_uc<U: IsVisibleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "ref": element_ref, "visible": output.visible }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
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

/// Handle is_focused requests using the use case pattern.
pub fn handle_is_focused_uc<U: IsFocusedUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "ref": element_ref, "focused": output.focused, "found": output.found }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_is_focused(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_property(session_manager, &request, "focused", |el| el.focused)
}

/// Handle is_enabled requests using the use case pattern.
pub fn handle_is_enabled_uc<U: IsEnabledUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "ref": element_ref, "enabled": output.enabled, "found": output.found }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_is_enabled(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_property(session_manager, &request, "enabled", |el| {
        !el.disabled.unwrap_or(false)
    })
}

/// Handle is_checked requests using the use case pattern.
pub fn handle_is_checked_uc<U: IsCheckedUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ElementStateInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => {
            let mut response =
                json!({ "ref": element_ref, "checked": output.checked, "found": output.found });
            if let Some(msg) = output.message {
                response["message"] = json!(msg);
            }
            RpcResponse::success(req_id, response)
        }
        Err(e) => session_error_response(req_id, e),
    }
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

/// Handle get_focused requests using the use case pattern.
pub fn handle_get_focused_uc<U: GetFocusedUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match usecase.execute(session_id) {
        Ok(output) => {
            if let Some(el) = output.element {
                RpcResponse::success(
                    req_id,
                    json!({
                        "ref": el.element_ref,
                        "type": el.element_type.as_str(),
                        "label": el.label,
                        "value": el.value,
                        "found": true
                    }),
                )
            } else {
                RpcResponse::success(
                    req_id,
                    json!({
                        "found": false,
                        "message": "No focused element found."
                    }),
                )
            }
        }
        Err(e) => session_error_response(req_id, e),
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

/// Handle get_title requests using the use case pattern.
pub fn handle_get_title_uc<U: GetTitleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match usecase.execute(session_id) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "session_id": output.session_id,
                "title": output.title,
                "command": output.title
            }),
        ),
        Err(e) => session_error_response(req_id, e),
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

/// Handle focus requests using the use case pattern.
pub fn handle_focus_uc<U: FocusUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = FocusInput {
        session_id,
        element_ref,
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_focus(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_action(session_manager, &request, b"\t")
}

/// Handle clear requests using the use case pattern.
pub fn handle_clear_uc<U: ClearUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ClearInput {
        session_id,
        element_ref,
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_clear(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    element_action(session_manager, &request, b"\x15")
}

/// Handle select_all requests using the use case pattern.
pub fn handle_select_all_uc<U: SelectAllUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = SelectAllInput {
        session_id,
        element_ref,
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_select_all(
    session_manager: &Arc<SessionManager>,
    request: RpcRequest,
) -> RpcResponse {
    element_action(session_manager, &request, b"\x01")
}

/// Handle toggle requests using the use case pattern.
pub fn handle_toggle_uc<U: ToggleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let force_state = request.param_bool_opt("state");
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ToggleInput {
        session_id,
        element_ref: element_ref.clone(),
        state: force_state,
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "success": true, "ref": element_ref, "checked": output.checked }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_toggle(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let force_state = request.param_bool_opt("state");
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

/// Handle select requests using the use case pattern.
pub fn handle_select_uc<U: SelectUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let option = match request.require_str("option") {
        Ok(o) => o.to_owned(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = SelectInput {
        session_id,
        element_ref: element_ref.clone(),
        option: option.clone(),
    };

    match usecase.execute(input) {
        Ok(_output) => RpcResponse::success(
            req_id,
            json!({ "success": true, "ref": element_ref, "option": option }),
        ),
        Err(e) => session_error_response(req_id, e),
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

/// Handle scroll_into_view requests using the use case pattern.
pub fn handle_scroll_into_view_uc<U: ScrollIntoViewUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = ScrollIntoViewInput {
        session_id,
        element_ref: element_ref.clone(),
    };

    match usecase.execute(input) {
        Ok(output) => {
            if output.success {
                RpcResponse::success(
                    req_id,
                    json!({
                        "success": true,
                        "ref": element_ref,
                        "scrolls_needed": output.scrolls_needed
                    }),
                )
            } else {
                RpcResponse::success(
                    req_id,
                    json!({
                        "success": false,
                        "message": output.message.unwrap_or_else(|| "Element not found after scrolling".to_string())
                    }),
                )
            }
        }
        Err(e) => session_error_response(req_id, e),
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

/// Handle multiselect requests using the use case pattern.
pub fn handle_multiselect_uc<U: MultiselectUseCase>(
    usecase: &U,
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
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = MultiselectInput {
        session_id,
        element_ref: element_ref.clone(),
        options,
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({ "success": true, "ref": element_ref, "selected_options": output.selected_options }),
        ),
        Err(e) => session_error_response(req_id, e),
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
