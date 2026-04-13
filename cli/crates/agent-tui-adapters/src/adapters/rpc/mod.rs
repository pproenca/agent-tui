//! RPC boundary types and conversions between transport and use cases.

pub mod params;
pub mod types;

pub use types::RpcRequest;
pub use types::RpcResponse;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use super::snapshot_adapters::session_info_to_json;
use crate::adapters::daemon::DomainError;
use crate::domain::AssertInput;
use crate::domain::AssertOutput;
use crate::domain::AttachInput;
use crate::domain::AttachOutput;
use crate::domain::CleanupInput;
use crate::domain::CleanupOutput;
use crate::domain::KeydownInput;
use crate::domain::KeystrokeInput;
use crate::domain::KeyupInput;
use crate::domain::KillOutput;
use crate::domain::ResizeInput;
use crate::domain::ResizeOutput;
use crate::domain::RestartOutput;
use crate::domain::SessionId;
use crate::domain::SessionInput;
use crate::domain::SessionsOutput;
use crate::domain::ShutdownOutput;
use crate::domain::SnapshotInput;
use crate::domain::SnapshotOutput;
use crate::domain::SpawnInput;
use crate::domain::SpawnOutput;
use crate::domain::TerminalWriteInput;
use crate::domain::TerminalWriteOutput;
use crate::domain::TypeInput;
use crate::domain::WaitInput;
use crate::domain::WaitOutput;
use crate::usecases::ports::SessionError;

pub fn to_value<T: Serialize>(value: T) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(value)
}

pub fn to_value_opt<T: Serialize>(
    value: Option<T>,
) -> Result<Option<serde_json::Value>, serde_json::Error> {
    value.map(serde_json::to_value).transpose()
}

fn invalid_session_response(id: u64, message: &str) -> RpcResponse {
    RpcResponse::error(id, -32602, &format!("Invalid session: {message}"))
}

#[allow(clippy::result_large_err)]
fn deserialize_required_params<T: DeserializeOwned>(
    request: &RpcRequest,
) -> Result<T, RpcResponse> {
    request
        .params
        .as_ref()
        .ok_or_else(|| RpcResponse::error(request.id, -32602, "Missing params"))
        .and_then(|params| {
            T::deserialize(params).map_err(|err| {
                RpcResponse::error(request.id, -32602, &format!("Invalid params: {err}"))
            })
        })
}

#[allow(clippy::result_large_err)]
fn deserialize_optional_params<T>(request: &RpcRequest) -> Result<T, RpcResponse>
where
    T: DeserializeOwned + Default,
{
    request.params.as_ref().map_or_else(
        || Ok(T::default()),
        |params| {
            T::deserialize(params).map_err(|err| {
                RpcResponse::error(request.id, -32602, &format!("Invalid params: {err}"))
            })
        },
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_session_id(
    id: u64,
    session: Option<String>,
) -> Result<Option<SessionId>, RpcResponse> {
    match session {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err(invalid_session_response(
                    id,
                    "Session ID cannot be empty or whitespace-only",
                ));
            }
            SessionId::try_new(trimmed)
                .map(Some)
                .map_err(|err| invalid_session_response(id, &err.to_string()))
        }
    }
}

#[allow(clippy::result_large_err)]
pub fn parse_session_selector(
    id: u64,
    session: Option<String>,
) -> Result<Option<SessionId>, RpcResponse> {
    match session {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed == "active" {
                return Ok(None);
            }
            if trimmed.is_empty() {
                return Err(invalid_session_response(
                    id,
                    "Session ID cannot be empty or whitespace-only",
                ));
            }
            SessionId::try_new(trimmed)
                .map(Some)
                .map_err(|err| invalid_session_response(id, &err.to_string()))
        }
    }
}

#[allow(clippy::result_large_err)]
pub fn parse_session_input(request: &RpcRequest) -> Result<SessionInput, RpcResponse> {
    let session_id =
        parse_session_selector(request.id, request.param_str("session").map(String::from))?;
    Ok(SessionInput { session_id })
}

pub fn domain_error_response(id: u64, err: &DomainError) -> RpcResponse {
    RpcResponse::domain_error(
        id,
        err.code(),
        &err.to_string(),
        err.category().as_str(),
        Some(err.context()),
        Some(err.suggestion()),
    )
}

pub fn session_error_response(id: u64, err: SessionError) -> RpcResponse {
    domain_error_response(id, &DomainError::from(err))
}

pub fn lock_timeout_response(id: u64, session_id: Option<&str>) -> RpcResponse {
    let err = DomainError::LockTimeout {
        session_id: session_id.map(String::from),
    };
    domain_error_response(id, &err)
}

#[allow(clippy::result_large_err)]
pub fn parse_spawn_input(request: &RpcRequest) -> Result<SpawnInput, RpcResponse> {
    let rpc_params: params::SpawnParams = deserialize_required_params(request)?;

    let command = if rpc_params.command.is_empty() {
        "bash".to_string()
    } else {
        rpc_params.command
    };

    Ok(SpawnInput {
        command,
        args: rpc_params.args,
        cwd: rpc_params.cwd,
        env: rpc_params.env,
        session_id: parse_session_id(request.id, rpc_params.session)?,
        size: rpc_params.size,
    })
}

pub fn spawn_output_to_response(id: u64, output: SpawnOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "pid": output.pid
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_snapshot_input(request: &RpcRequest) -> Result<SnapshotInput, RpcResponse> {
    let rpc_params: params::SnapshotParams = deserialize_optional_params(request)?;

    Ok(SnapshotInput {
        session_id: parse_session_selector(request.id, rpc_params.session)?,
        region: rpc_params.region,
        strip_ansi: rpc_params.strip_ansi,
        retain_ansi: rpc_params.retain_ansi,
        include_cursor: rpc_params.include_cursor,
        include_render: rpc_params.include_render || rpc_params.retain_ansi,
    })
}

pub fn snapshot_output_to_response(
    id: u64,
    output: SnapshotOutput,
    strip_ansi: bool,
    retain_ansi: bool,
) -> RpcResponse {
    use crate::common::strip_ansi_codes;

    let rendered = output.rendered;
    let compact_rendered = output.compact_rendered;
    let screenshot = if retain_ansi {
        compact_rendered
            .clone()
            .or(rendered.clone())
            .unwrap_or_else(|| output.screenshot.clone())
    } else if strip_ansi {
        strip_ansi_codes(&output.screenshot)
    } else {
        output.screenshot
    };

    let mut result = json!({
        "session_id": output.session_id.as_str(),
        "screenshot": screenshot
    });

    if let Some(cursor) = output.cursor {
        result["cursor"] = json!({
            "row": cursor.row,
            "col": cursor.col,
            "visible": cursor.visible
        });
    }

    if let Some(rendered) = rendered {
        result["rendered"] = json!(rendered);
    }
    if let Some(compact_rendered) = compact_rendered {
        result["compact_rendered"] = json!(compact_rendered);
    }

    RpcResponse::success(id, result)
}

#[allow(clippy::result_large_err)]
pub fn parse_keystroke_input(request: &RpcRequest) -> Result<KeystrokeInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeystrokeInput {
        session_id: parse_session_selector(
            request.id,
            request.param_str("session").map(String::from),
        )?,
        key,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_type_input(request: &RpcRequest) -> Result<TypeInput, RpcResponse> {
    let text = request.require_str("text")?.to_string();

    Ok(TypeInput {
        session_id: parse_session_selector(
            request.id,
            request.param_str("session").map(String::from),
        )?,
        text,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_keydown_input(request: &RpcRequest) -> Result<KeydownInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeydownInput {
        session_id: parse_session_selector(
            request.id,
            request.param_str("session").map(String::from),
        )?,
        key,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_keyup_input(request: &RpcRequest) -> Result<KeyupInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeyupInput {
        session_id: parse_session_selector(
            request.id,
            request.param_str("session").map(String::from),
        )?,
        key,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_wait_input(request: &RpcRequest) -> Result<WaitInput, RpcResponse> {
    let rpc_params: params::WaitParams = deserialize_optional_params(request)?;

    let condition = match rpc_params.condition.as_deref() {
        Some(raw) => Some(crate::domain::WaitConditionType::parse(raw).map_err(|e| {
            RpcResponse::error(request.id, -32602, &format!("Invalid condition: {e}"))
        })?),
        None => None,
    };

    if condition
        .as_ref()
        .is_some_and(agent_tui_domain::WaitConditionType::requires_text)
        && rpc_params.text.as_deref().is_none()
    {
        return Err(RpcResponse::error(
            request.id,
            -32602,
            "Invalid condition: text is required",
        ));
    }

    Ok(WaitInput {
        session_id: parse_session_selector(request.id, rpc_params.session)?,
        text: rpc_params.text,
        timeout_ms: rpc_params.timeout_ms,
        condition,
    })
}

pub fn wait_output_to_response(id: u64, output: WaitOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "found": output.found,
            "elapsed_ms": output.elapsed_ms
        }),
    )
}

pub fn kill_output_to_response(id: u64, output: KillOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str()
        }),
    )
}

pub fn sessions_output_to_response(id: u64, output: SessionsOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "sessions": output.sessions.iter().map(session_info_to_json).collect::<Vec<_>>(),
            "active_session": output.active_session.as_ref().map(agent_tui_domain::SessionId::as_str)
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_resize_input(request: &RpcRequest) -> Result<ResizeInput, RpcResponse> {
    let rpc_params: params::ResizeParams = deserialize_required_params(request)?;

    Ok(ResizeInput {
        session_id: parse_session_selector(request.id, rpc_params.session)?,
        size: rpc_params.size,
    })
}

pub fn resize_output_to_response(id: u64, output: ResizeOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str(),
            "cols": output.size.cols(),
            "rows": output.size.rows()
        }),
    )
}

pub fn restart_output_to_response(id: u64, output: RestartOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "old_session_id": output.old_session_id.as_str(),
            "new_session_id": output.new_session_id.as_str(),
            "command": output.command,
            "pid": output.pid
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_attach_input(request: &RpcRequest) -> Result<AttachInput, RpcResponse> {
    let session_id = request.require_str("session")?;
    Ok(AttachInput {
        session_id: SessionId::try_new(session_id.trim()).map_err(|err| {
            RpcResponse::error(request.id, -32602, &format!("Invalid session: {err}"))
        })?,
    })
}

pub fn attach_output_to_response(id: u64, output: &AttachOutput) -> RpcResponse {
    let session_id = output.session_id.as_str();
    let message = format!("Now attached to session {session_id}");
    RpcResponse::success(
        id,
        json!({
            "session_id": session_id,
            "success": output.success,
            "message": message
        }),
    )
}

pub fn parse_cleanup_input(request: &RpcRequest) -> CleanupInput {
    let all = request.param_bool("all", false);
    CleanupInput { all }
}

pub fn cleanup_output_to_response(id: u64, output: CleanupOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "cleaned": output.cleaned,
            "failures": output.failures.iter().map(|f| json!({
                "session_id": f.session_id.as_str(),
                "error": f.error
            })).collect::<Vec<_>>()
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_assert_input(request: &RpcRequest) -> Result<AssertInput, RpcResponse> {
    let condition_type = request.require_str("type")?;
    let value = request.require_str("value")?.to_string();

    let condition_type = crate::domain::AssertConditionType::parse(condition_type)
        .map_err(|e| RpcResponse::error(request.id, -32602, &format!("Invalid type: {e}")))?;

    Ok(AssertInput {
        session_id: parse_session_selector(
            request.id,
            request.param_str("session").map(String::from),
        )?,
        condition_type,
        value,
    })
}

pub fn assert_output_to_response(id: u64, output: AssertOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "passed": output.passed,
            "condition": output.condition
        }),
    )
}

pub fn shutdown_output_to_response(id: u64, output: ShutdownOutput) -> RpcResponse {
    RpcResponse::success(id, json!({ "acknowledged": output.acknowledged }))
}

pub fn terminal_write_output_to_response(id: u64, output: TerminalWriteOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "bytes_written": output.bytes_written,
            "success": output.success
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_terminal_write_input(request: &RpcRequest) -> Result<TerminalWriteInput, RpcResponse> {
    let rpc_params: params::PtyWriteParams = deserialize_required_params(request)?;

    let data = STANDARD
        .decode(&rpc_params.data)
        .map_err(|e| RpcResponse::error(request.id, -32602, &format!("Invalid base64: {e}")))?;

    Ok(TerminalWriteInput {
        session_id: parse_session_selector(request.id, rpc_params.session)?,
        data,
    })
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
