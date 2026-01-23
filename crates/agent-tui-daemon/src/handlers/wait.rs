use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::DomainError;
use crate::lock_helpers::acquire_session_lock;
use crate::session::SessionManager;
use crate::wait::{StableTracker, WaitCondition, check_condition};

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

pub fn handle_wait(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
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

    let session = match session_manager.resolve(session_id) {
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
