use crate::infra::ipc::RpcRequest;
use tracing::{Span, debug_span};

pub use crate::adapters::session_error_response;

pub fn handler_span(request: &RpcRequest, handler: &'static str) -> Span {
    let session = request.param_str("session");
    debug_span!(
        "rpc_handler",
        handler = handler,
        request_id = request.id,
        session = ?session
    )
}
