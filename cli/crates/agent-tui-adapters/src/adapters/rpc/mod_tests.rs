use super::*;
use crate::domain::TerminalSize;

fn make_request(id: u64, method: &str, params: Option<serde_json::Value>) -> RpcRequest {
    RpcRequest::new(id, method.to_string(), params)
}

#[test]
fn test_parse_spawn_input_defaults() {
    let request = make_request(1, "spawn", Some(json!({})));
    let input = parse_spawn_input(&request).expect("spawn input should parse");
    assert_eq!(input.command, "bash");
    assert!(input.env.is_none());
    assert_eq!(input.size, TerminalSize::default());
}

#[test]
fn test_parse_spawn_input_preserves_env_map() {
    let request = make_request(
        1,
        "spawn",
        Some(json!({
            "command": "bash",
            "env": {
                "FOO": "bar",
                "EMPTY": "",
            }
        })),
    );

    let input = parse_spawn_input(&request).expect("spawn input should parse");
    assert_eq!(
        input.env,
        Some(std::collections::HashMap::from([
            ("FOO".to_string(), "bar".to_string()),
            ("EMPTY".to_string(), String::new()),
        ]))
    );
}

#[test]
fn test_parse_snapshot_input() {
    let request = make_request(
        1,
        "snapshot",
        Some(json!({"retain_ansi": true, "include_cursor": true})),
    );
    let input = parse_snapshot_input(&request).expect("snapshot params should parse");
    assert!(input.retain_ansi);
    assert!(input.include_cursor);
    assert!(input.include_render);
}

#[test]
fn test_parse_snapshot_input_rejects_invalid_params() {
    let request = make_request(1, "snapshot", Some(json!({"include_cursor": "yes"})));
    let response = parse_snapshot_input(&request).expect_err("invalid params should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_snapshot_output_to_response_prefers_rendered_when_retaining_ansi() {
    let output = SnapshotOutput {
        session_id: crate::domain::SessionId::try_new("session-1").expect("valid session id"),
        screenshot: "plain text".to_string(),
        cursor: None,
        rendered: Some("\u{1b}[31mplain text\u{1b}[0m".to_string()),
        compact_rendered: Some("\u{1b}[31mplain text\u{1b}[0m".to_string()),
    };

    let response = snapshot_output_to_response(1, output, false, true);
    let value = serde_json::to_value(response).expect("response should serialize");

    assert_eq!(
        value["result"]["screenshot"],
        json!("\u{1b}[31mplain text\u{1b}[0m")
    );
    assert_eq!(
        value["result"]["rendered"],
        json!("\u{1b}[31mplain text\u{1b}[0m")
    );
    assert_eq!(
        value["result"]["compact_rendered"],
        json!("\u{1b}[31mplain text\u{1b}[0m")
    );
}

#[test]
fn test_parse_session_selector_defaults_to_active() {
    assert_eq!(
        parse_session_selector(1, None).expect("missing selector"),
        None
    );
    assert_eq!(
        parse_session_selector(1, Some("active".to_string())).expect("active selector"),
        None
    );
    assert_eq!(
        parse_session_selector(1, Some("  active  ".to_string())).expect("trimmed active selector"),
        None
    );
}

#[test]
fn test_parse_session_selector_keeps_explicit_id() {
    let parsed = parse_session_selector(1, Some("sess-1".to_string()))
        .expect("session id")
        .expect("explicit session id");
    assert_eq!(parsed.as_str(), "sess-1");
}

#[test]
fn test_parse_session_selector_rejects_blank_explicit_id() {
    let response = parse_session_selector(7, Some("   ".to_string()))
        .expect_err("blank explicit selector should be rejected");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
    assert_eq!(
        value["error"]["message"],
        "Invalid session: Session ID cannot be empty or whitespace-only"
    );
}

#[test]
fn test_parse_spawn_input_keeps_active_as_explicit_id() {
    let request = make_request(
        1,
        "spawn",
        Some(json!({
            "session": "active",
            "command": "bash",
        })),
    );
    let input = parse_spawn_input(&request).expect("spawn input");
    let session_id = input.session_id.expect("spawn session id");
    assert_eq!(session_id.as_str(), "active");
}

#[test]
fn test_parse_spawn_input_rejects_blank_explicit_session_id() {
    let request = make_request(
        1,
        "spawn",
        Some(json!({
            "session": "   ",
            "command": "bash",
        })),
    );
    let response = parse_spawn_input(&request).expect_err("blank custom session id");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_wait_input() {
    let request = make_request(
        1,
        "wait",
        Some(json!({"text": "ready", "timeout_ms": 5000})),
    );
    let input = parse_wait_input(&request).expect("wait input should parse");
    assert_eq!(input.text.as_deref(), Some("ready"));
    assert_eq!(input.timeout_ms, 5000);
}

#[test]
fn test_parse_wait_input_requires_text() {
    let request = make_request(1, "wait", Some(json!({"condition": "text"})));
    let response = parse_wait_input(&request).expect_err("wait input should fail without text");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_wait_input_rejects_invalid_params() {
    let request = make_request(1, "wait", Some(json!({"timeout_ms": "fast"})));
    let response = parse_wait_input(&request).expect_err("invalid params should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_wait_input_rejects_blank_explicit_session() {
    let request = make_request(1, "wait", Some(json!({"session": "   ", "text": "ready"})));
    let response = parse_wait_input(&request).expect_err("blank explicit session should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_keydown_input() {
    let request = make_request(1, "keydown", Some(json!({"key": "Ctrl"})));
    let input = parse_keydown_input(&request).expect("keydown input should parse");
    assert_eq!(input.key, "Ctrl");
}

#[test]
fn test_parse_keydown_input_rejects_blank_explicit_session() {
    let request = make_request(1, "keydown", Some(json!({"key": "Ctrl", "session": " "})));
    let response = parse_keydown_input(&request).expect_err("blank explicit session should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_keyup_input() {
    let request = make_request(1, "keyup", Some(json!({"key": "Ctrl"})));
    let input = parse_keyup_input(&request).expect("keyup input should parse");
    assert_eq!(input.key, "Ctrl");
}

#[test]
fn test_parse_terminal_write_input() {
    let data = STANDARD.encode(b"hello");
    let request = make_request(1, "pty_write", Some(json!({"data": data})));
    let input = parse_terminal_write_input(&request).expect("pty_write input should parse");
    assert_eq!(input.data, b"hello");
}

#[test]
fn test_parse_resize_input_rejects_invalid_params() {
    let request = make_request(1, "resize", Some(json!({"cols": "wide", "rows": 24})));
    let response = parse_resize_input(&request).expect_err("invalid params should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_resize_input_rejects_blank_explicit_session() {
    let request = make_request(
        1,
        "resize",
        Some(json!({"cols": 80, "rows": 24, "session": " "})),
    );
    let response = parse_resize_input(&request).expect_err("blank explicit session should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_mouse_click_input() {
    let request = make_request(
        1,
        "mouse_click",
        Some(json!({"col": 5, "row": 10, "button": "left"})),
    );
    let input = parse_mouse_click_input(&request).expect("mouse_click input should parse");
    assert_eq!(input.col, 5);
    assert_eq!(input.row, 10);
    assert_eq!(input.button, crate::domain::MouseButton::Left);
}

#[test]
fn test_parse_mouse_click_input_defaults_button() {
    let request = make_request(1, "mouse_click", Some(json!({"col": 5, "row": 10})));
    let input = parse_mouse_click_input(&request).expect("mouse_click should default to left button");
    assert_eq!(input.button, crate::domain::MouseButton::Left);
}

#[test]
fn test_parse_mouse_click_input_right_button() {
    let request = make_request(
        1,
        "mouse_click",
        Some(json!({"col": 5, "row": 10, "button": "right"})),
    );
    let input = parse_mouse_click_input(&request).expect("mouse_click should parse right button");
    assert_eq!(input.button, crate::domain::MouseButton::Right);
}

#[test]
fn test_parse_mouse_click_input_invalid_button() {
    let request = make_request(
        1,
        "mouse_click",
        Some(json!({"col": 5, "row": 10, "button": "invalid"})),
    );
    let response = parse_mouse_click_input(&request).expect_err("invalid button should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_mouse_click_input_missing_col() {
    let request = make_request(1, "mouse_click", Some(json!({"row": 10})));
    let response = parse_mouse_click_input(&request).expect_err("missing col should error");
    let value = serde_json::to_value(response).expect("response should serialize");
    assert_eq!(value["error"]["code"], -32602);
}

#[test]
fn test_parse_mouse_move_input() {
    let request = make_request(
        1,
        "mouse_move",
        Some(json!({"col": 5, "row": 10})),
    );
    let input = parse_mouse_move_input(&request).expect("mouse_move input should parse");
    assert_eq!(input.col, 5);
    assert_eq!(input.row, 10);
}

#[test]
fn test_parse_mouse_down_input() {
    let request = make_request(
        1,
        "mouse_down",
        Some(json!({"col": 5, "row": 10, "button": "middle"})),
    );
    let input = parse_mouse_down_input(&request).expect("mouse_down input should parse");
    assert_eq!(input.col, 5);
    assert_eq!(input.row, 10);
    assert_eq!(input.button, crate::domain::MouseButton::Middle);
}

#[test]
fn test_parse_mouse_up_input() {
    let request = make_request(
        1,
        "mouse_up",
        Some(json!({"col": 5, "row": 10, "button": "right"})),
    );
    let input = parse_mouse_up_input(&request).expect("mouse_up input should parse");
    assert_eq!(input.button, crate::domain::MouseButton::Right);
}
