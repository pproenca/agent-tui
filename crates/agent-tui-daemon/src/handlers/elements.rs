use agent_tui_core::Element;
use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::{Value, json};

use super::common::session_error_response;
use crate::adapters::{
    count_output_to_response, element_to_json as adapter_element_to_json, fill_success_response,
    parse_count_input, parse_fill_input, parse_find_input, parse_scroll_input,
    parse_snapshot_input, scroll_output_to_response,
};
use crate::domain::{
    ClearInput, ClickInput, DoubleClickInput, ElementStateInput, FocusInput, MultiselectInput,
    ScrollIntoViewInput, SelectAllInput, SelectInput, ToggleInput,
};
use crate::select_helpers::strip_ansi_codes;
use crate::usecases::{
    ClearUseCase, ClickUseCase, CountUseCase, DoubleClickUseCase, FillUseCase, FindUseCase,
    FocusUseCase, GetFocusedUseCase, GetTextUseCase, GetTitleUseCase, GetValueUseCase,
    IsCheckedUseCase, IsEnabledUseCase, IsFocusedUseCase, IsVisibleUseCase, MultiselectUseCase,
    ScrollIntoViewUseCase, ScrollUseCase, SelectAllUseCase, SelectUseCase, SnapshotUseCase,
    ToggleUseCase,
};

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

/// Handle snapshot requests using the use case pattern.
pub fn handle_snapshot_uc<U: SnapshotUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_snapshot_input(&request);
    let should_strip_ansi = request
        .params
        .as_ref()
        .and_then(|p| p.get("strip_ansi"))
        .and_then(|v| v.as_bool())
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

/// Handle count requests using the use case pattern.
pub fn handle_count_uc<U: CountUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_count_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => count_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
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
