use crate::adapters::ipc::{RpcRequest, RpcResponse};
use serde_json::{Value, json};

use super::common;
use super::common::session_error_response;
use crate::adapters::{
    count_output_to_response, element_to_json as adapter_element_to_json, fill_success_response,
    parse_count_input, parse_fill_input, parse_find_input, parse_scroll_input, parse_session_id,
    parse_session_input, parse_snapshot_input, scroll_output_to_response,
    snapshot_output_to_response, snapshot_to_dto,
};
use crate::domain::{
    AccessibilitySnapshotInput, ClearInput, ClickInput, DoubleClickInput, ElementStateInput,
    FocusInput, MultiselectInput, ScrollIntoViewInput, SelectAllInput, SelectInput, ToggleInput,
};
use crate::usecases::{
    AccessibilitySnapshotUseCase, ClearUseCase, ClickUseCase, CountUseCase, DoubleClickUseCase,
    FillUseCase, FindUseCase, FocusUseCase, GetFocusedUseCase, GetTextUseCase, GetTitleUseCase,
    GetValueUseCase, IsCheckedUseCase, IsEnabledUseCase, IsFocusedUseCase, IsVisibleUseCase,
    MultiselectUseCase, ScrollIntoViewUseCase, ScrollUseCase, SelectAllUseCase, SelectUseCase,
    SnapshotUseCase, ToggleUseCase,
};

pub fn handle_snapshot_uc<U: SnapshotUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "snapshot").entered();
    let input = parse_snapshot_input(&request);
    let strip_ansi = request
        .params
        .as_ref()
        .and_then(|p| p.get("strip_ansi"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => snapshot_output_to_response(req_id, output, strip_ansi),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_accessibility_snapshot_uc<U: AccessibilitySnapshotUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "accessibility_snapshot").entered();
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let interactive_only = request.param_bool("interactive", false);
    let req_id = request.id;

    let input = AccessibilitySnapshotInput {
        session_id,
        interactive_only,
    };

    match usecase.execute(input) {
        Ok(output) => {
            let dto = snapshot_to_dto(&output.snapshot);
            RpcResponse::success(
                req_id,
                json!({
                    "session_id": output.session_id.as_str(),
                    "tree": dto.tree,
                    "refs": dto.refs,
                    "stats": dto.stats
                }),
            )
        }
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_click_uc<U: ClickUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "click").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_dbl_click_uc<U: DoubleClickUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "dbl_click").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_fill_uc<U: FillUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "fill").entered();
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

pub fn handle_find_uc<U: FindUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "find").entered();
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

pub fn handle_count_uc<U: CountUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "count").entered();
    let input = parse_count_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => count_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_scroll_uc<U: ScrollUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "scroll").entered();
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

pub fn handle_get_text_uc<U: GetTextUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "get_text").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_get_value_uc<U: GetValueUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "get_value").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_is_visible_uc<U: IsVisibleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "is_visible").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_is_focused_uc<U: IsFocusedUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "is_focused").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_is_enabled_uc<U: IsEnabledUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "is_enabled").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_is_checked_uc<U: IsCheckedUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "is_checked").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_get_focused_uc<U: GetFocusedUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "get_focused").entered();
    let input = parse_session_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
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

pub fn handle_get_title_uc<U: GetTitleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "get_title").entered();
    let input = parse_session_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "session_id": output.session_id.as_str(),
                "title": output.title
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_focus_uc<U: FocusUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "focus").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_clear_uc<U: ClearUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "clear").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_select_all_uc<U: SelectAllUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "select_all").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_toggle_uc<U: ToggleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "toggle").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let force_state = request.param_bool_opt("state");
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_select_uc<U: SelectUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "select").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let option = match request.require_str("option") {
        Ok(o) => o.to_owned(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_scroll_into_view_uc<U: ScrollIntoViewUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "scroll_into_view").entered();
    let element_ref = match request.require_str("ref") {
        Ok(r) => r.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

pub fn handle_multiselect_uc<U: MultiselectUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "multiselect").entered();
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
    let session_id = parse_session_id(request.param_str("session").map(String::from));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ClickInput, ClickOutput, DomainElement, DomainElementType, DomainPosition,
        ElementStateInput, FillInput, FillOutput, FindInput, FindOutput, GetTextOutput,
        GetTitleOutput, SessionId, SessionInput, VisibilityOutput,
    };
    use crate::usecases::ports::SessionError;
    use crate::usecases::{
        ClickUseCase, FillUseCase, FindUseCase, GetTextUseCase, GetTitleUseCase, IsVisibleUseCase,
    };

    fn extract_json(response: RpcResponse) -> serde_json::Value {
        let json_str = serde_json::to_string(&response).expect("serialize");
        serde_json::from_str(&json_str).expect("parse")
    }

    struct MockGetTitleUseCase {
        session_id: String,
        title: String,
    }

    impl MockGetTitleUseCase {
        fn with_success(session_id: &str, title: &str) -> Self {
            Self {
                session_id: session_id.to_string(),
                title: title.to_string(),
            }
        }
    }

    impl GetTitleUseCase for MockGetTitleUseCase {
        fn execute(&self, _input: SessionInput) -> Result<GetTitleOutput, SessionError> {
            Ok(GetTitleOutput {
                session_id: SessionId::new(&self.session_id),
                title: self.title.clone(),
            })
        }
    }

    enum MockClickResult {
        Success { message: Option<String> },
        Error(SessionError),
    }

    struct MockClickUseCase {
        result: MockClickResult,
    }

    impl MockClickUseCase {
        fn with_success() -> Self {
            Self {
                result: MockClickResult::Success { message: None },
            }
        }

        fn with_error(error: SessionError) -> Self {
            Self {
                result: MockClickResult::Error(error),
            }
        }
    }

    impl ClickUseCase for MockClickUseCase {
        fn execute(&self, _input: ClickInput) -> Result<ClickOutput, SessionError> {
            match &self.result {
                MockClickResult::Success { message } => Ok(ClickOutput {
                    success: true,
                    message: message.clone(),
                    warning: None,
                }),
                MockClickResult::Error(e) => Err(SessionError::ElementNotFound(e.to_string())),
            }
        }
    }

    enum MockFillResult {
        Success,
        Error(SessionError),
    }

    struct MockFillUseCase {
        result: MockFillResult,
    }

    impl MockFillUseCase {
        fn with_success() -> Self {
            Self {
                result: MockFillResult::Success,
            }
        }

        fn with_error(error: SessionError) -> Self {
            Self {
                result: MockFillResult::Error(error),
            }
        }
    }

    impl FillUseCase for MockFillUseCase {
        fn execute(&self, _input: FillInput) -> Result<FillOutput, SessionError> {
            match &self.result {
                MockFillResult::Success => Ok(FillOutput {
                    success: true,
                    message: None,
                }),
                MockFillResult::Error(e) => Err(SessionError::ElementNotFound(e.to_string())),
            }
        }
    }

    struct MockFindUseCase {
        elements: Vec<DomainElement>,
    }

    impl MockFindUseCase {
        fn with_elements(elements: Vec<DomainElement>) -> Self {
            Self { elements }
        }

        fn empty() -> Self {
            Self { elements: vec![] }
        }
    }

    impl FindUseCase for MockFindUseCase {
        fn execute(&self, _input: FindInput) -> Result<FindOutput, SessionError> {
            Ok(FindOutput {
                elements: self.elements.clone(),
                count: self.elements.len(),
            })
        }
    }

    enum MockGetTextResult {
        Success { text: String },
        NotFound,
    }

    struct MockGetTextUseCase {
        result: MockGetTextResult,
    }

    impl MockGetTextUseCase {
        fn with_text(text: &str) -> Self {
            Self {
                result: MockGetTextResult::Success {
                    text: text.to_string(),
                },
            }
        }

        fn not_found() -> Self {
            Self {
                result: MockGetTextResult::NotFound,
            }
        }
    }

    impl GetTextUseCase for MockGetTextUseCase {
        fn execute(&self, _input: ElementStateInput) -> Result<GetTextOutput, SessionError> {
            match &self.result {
                MockGetTextResult::Success { text } => Ok(GetTextOutput {
                    text: text.clone(),
                    found: true,
                }),
                MockGetTextResult::NotFound => Ok(GetTextOutput {
                    text: String::new(),
                    found: false,
                }),
            }
        }
    }

    struct MockIsVisibleUseCase {
        visible: bool,
        found: bool,
    }

    impl MockIsVisibleUseCase {
        fn visible() -> Self {
            Self {
                visible: true,
                found: true,
            }
        }

        fn not_visible() -> Self {
            Self {
                visible: false,
                found: true,
            }
        }
    }

    impl IsVisibleUseCase for MockIsVisibleUseCase {
        fn execute(&self, _input: ElementStateInput) -> Result<VisibilityOutput, SessionError> {
            Ok(VisibilityOutput {
                visible: self.visible,
                found: self.found,
            })
        }
    }

    #[test]
    fn test_handle_get_title_response_has_no_command_field() {
        let usecase = MockGetTitleUseCase::with_success("test-session", "My App Title");
        let request = RpcRequest::new(1, "get_title".to_string(), None);

        let response = handle_get_title_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert!(parsed.get("result").is_some());

        let result = &parsed["result"];
        assert_eq!(result["session_id"], "test-session");
        assert_eq!(result["title"], "My App Title");
        assert!(
            result.get("command").is_none(),
            "Response should NOT contain 'command' field"
        );
    }

    #[test]
    fn test_handle_click_uc_success() {
        let usecase = MockClickUseCase::with_success();
        let request = RpcRequest::new(1, "click".to_string(), Some(json!({ "ref": "@btn1" })));

        let response = handle_click_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        let result = &parsed["result"];
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_handle_click_uc_with_session() {
        let usecase = MockClickUseCase::with_success();
        let request = RpcRequest::new(
            1,
            "click".to_string(),
            Some(json!({ "ref": "@submit", "session": "sess1" })),
        );

        let response = handle_click_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        let result = &parsed["result"];
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_handle_click_uc_element_not_found() {
        let usecase =
            MockClickUseCase::with_error(SessionError::ElementNotFound("@missing".into()));
        let request = RpcRequest::new(1, "click".to_string(), Some(json!({ "ref": "@missing" })));

        let response = handle_click_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_some());
    }

    #[test]
    fn test_handle_click_uc_missing_ref_returns_error() {
        let usecase = MockClickUseCase::with_success();
        let request = RpcRequest::new(1, "click".to_string(), Some(json!({})));

        let response = handle_click_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_some());
    }

    #[test]
    fn test_handle_fill_uc_success() {
        let usecase = MockFillUseCase::with_success();
        let request = RpcRequest::new(
            1,
            "fill".to_string(),
            Some(json!({ "ref": "@input", "value": "hello world" })),
        );

        let response = handle_fill_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        let result = &parsed["result"];
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_handle_fill_uc_element_not_found() {
        let usecase = MockFillUseCase::with_error(SessionError::ElementNotFound("@input".into()));
        let request = RpcRequest::new(
            1,
            "fill".to_string(),
            Some(json!({ "ref": "@input", "value": "test" })),
        );

        let response = handle_fill_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_some());
    }

    #[test]
    fn test_handle_fill_uc_missing_value_returns_error() {
        let usecase = MockFillUseCase::with_success();
        let request = RpcRequest::new(1, "fill".to_string(), Some(json!({ "ref": "@input" })));

        let response = handle_fill_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_some());
    }

    #[test]
    fn test_handle_find_uc_returns_elements() {
        let elements = vec![DomainElement {
            element_ref: "@btn1".to_string(),
            element_type: DomainElementType::Button,
            label: Some("Submit".to_string()),
            value: None,
            position: DomainPosition {
                row: 5,
                col: 10,
                width: Some(8),
                height: Some(1),
            },
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }];
        let usecase = MockFindUseCase::with_elements(elements);
        let request = RpcRequest::new(1, "find".to_string(), Some(json!({ "role": "button" })));

        let response = handle_find_uc(&usecase, request);
        let parsed = extract_json(response);

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        let result = &parsed["result"];
        assert_eq!(result["count"], 1);
        assert!(result["elements"].is_array());
        assert_eq!(result["elements"][0]["ref"], "@btn1");
    }

    #[test]
    fn test_handle_find_uc_empty_result() {
        let usecase = MockFindUseCase::empty();
        let request = RpcRequest::new(
            1,
            "find".to_string(),
            Some(json!({ "role": "nonexistent" })),
        );

        let response = handle_find_uc(&usecase, request);
        let parsed = extract_json(response);

        let result = &parsed["result"];
        assert_eq!(result["count"], 0);
        assert_eq!(result["elements"], json!([]));
    }

    #[test]
    fn test_handle_get_text_uc_found() {
        let usecase = MockGetTextUseCase::with_text("Hello World");
        let request = RpcRequest::new(1, "get_text".to_string(), Some(json!({ "ref": "@label" })));

        let response = handle_get_text_uc(&usecase, request);
        let parsed = extract_json(response);

        let result = &parsed["result"];
        assert_eq!(result["text"], "Hello World");
        assert_eq!(result["found"], true);
        assert_eq!(result["ref"], "@label");
    }

    #[test]
    fn test_handle_get_text_uc_not_found() {
        let usecase = MockGetTextUseCase::not_found();
        let request = RpcRequest::new(
            1,
            "get_text".to_string(),
            Some(json!({ "ref": "@missing" })),
        );

        let response = handle_get_text_uc(&usecase, request);
        let parsed = extract_json(response);

        let result = &parsed["result"];
        assert_eq!(result["found"], false);
    }

    #[test]
    fn test_handle_is_visible_uc_visible() {
        let usecase = MockIsVisibleUseCase::visible();
        let request = RpcRequest::new(1, "is_visible".to_string(), Some(json!({ "ref": "@btn" })));

        let response = handle_is_visible_uc(&usecase, request);
        let parsed = extract_json(response);

        let result = &parsed["result"];
        assert_eq!(result["visible"], true);
        assert_eq!(result["ref"], "@btn");
    }

    #[test]
    fn test_handle_is_visible_uc_not_visible() {
        let usecase = MockIsVisibleUseCase::not_visible();
        let request = RpcRequest::new(
            1,
            "is_visible".to_string(),
            Some(json!({ "ref": "@hidden" })),
        );

        let response = handle_is_visible_uc(&usecase, request);
        let parsed = extract_json(response);

        let result = &parsed["result"];
        assert_eq!(result["visible"], false);
    }
}
