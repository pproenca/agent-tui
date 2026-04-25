//! RPC router for daemon adapters.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;
use serde_json::json;

use super::usecase_container::UseCaseContainer;
use crate::adapters::daemon::handlers;
use crate::usecases::ports::SessionRepository;

pub struct Router<'a, R: SessionRepository + 'static> {
    usecases: &'a UseCaseContainer<R>,
}

impl<'a, R: SessionRepository + 'static> Router<'a, R> {
    pub fn new(usecases: &'a UseCaseContainer<R>) -> Self {
        Self { usecases }
    }

    pub fn route(&self, request: RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "ping" => RpcResponse::success(request.id, json!({ "pong": true })),

            "version" => RpcResponse::success(
                request.id,
                json!({
                    "daemon_version": env!("AGENT_TUI_VERSION"),
                    "daemon_commit": env!("AGENT_TUI_GIT_SHA")
                }),
            ),

            "spawn" => handlers::session::handle_spawn(&self.usecases.session.spawn, request),
            "kill" => handlers::session::handle_kill(&self.usecases.session.kill, request),
            "restart" => handlers::session::handle_restart(&self.usecases.session.restart, request),
            "sessions" => {
                handlers::session::handle_sessions(&self.usecases.session.sessions, request)
            }
            "resize" => handlers::session::handle_resize(&self.usecases.session.resize, request),
            "attach" => handlers::session::handle_attach(&self.usecases.session.attach, request),
            "cleanup" => handlers::session::handle_cleanup(&self.usecases.session.cleanup, request),
            "assert" => handlers::session::handle_assert(&self.usecases.session.assert, request),
            "snapshot" => {
                handlers::snapshot::handle_snapshot_uc(&self.usecases.snapshot.snapshot, request)
            }
            "keystroke" => {
                handlers::input::handle_keystroke_uc(&self.usecases.input.keystroke, request)
            }
            "keydown" => handlers::input::handle_keydown_uc(&self.usecases.input.keydown, request),
            "keyup" => handlers::input::handle_keyup_uc(&self.usecases.input.keyup, request),
            "mouse_click" => {
                handlers::input::handle_mouse_click_uc(&self.usecases.input.mouse_click, request)
            }
            "mouse_move" => {
                handlers::input::handle_mouse_move_uc(&self.usecases.input.mouse_move, request)
            }
            "mouse_down" => {
                handlers::input::handle_mouse_down_uc(&self.usecases.input.mouse_down, request)
            }
            "mouse_up" => {
                handlers::input::handle_mouse_up_uc(&self.usecases.input.mouse_up, request)
            }
            "type" => handlers::input::handle_type_uc(&self.usecases.input.type_text, request),
            "wait" => handlers::wait::handle_wait_uc(&self.usecases.wait, request),

            "pty_write" => handlers::diagnostics::handle_terminal_write_uc(
                &self.usecases.diagnostics.terminal_write,
                request,
            ),
            "shutdown" => handlers::diagnostics::handle_shutdown_uc(
                &self.usecases.diagnostics.shutdown,
                request,
            ),

            _ => RpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod tests;
