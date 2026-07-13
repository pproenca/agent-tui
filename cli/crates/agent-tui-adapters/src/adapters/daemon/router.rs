//! RPC router for daemon adapters.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;
use serde_json::json;

use crate::adapters::daemon::handlers;
use crate::usecases::ports::Clock;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifier;
use std::sync::atomic::AtomicBool;

pub struct Router<'a, R: SessionRepository + ?Sized> {
    repository: &'a R,
    clock: &'a dyn Clock,
    shutdown_flag: &'a AtomicBool,
    shutdown_notifier: &'a dyn ShutdownNotifier,
}

impl<'a, R: SessionRepository + ?Sized> Router<'a, R> {
    pub fn new(
        repository: &'a R,
        clock: &'a dyn Clock,
        shutdown_flag: &'a AtomicBool,
        shutdown_notifier: &'a dyn ShutdownNotifier,
    ) -> Self {
        Self {
            repository,
            clock,
            shutdown_flag,
            shutdown_notifier,
        }
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

            "spawn" => handlers::session::handle_spawn(self.repository, request),
            "kill" => handlers::session::handle_kill(self.repository, request),
            "restart" => handlers::session::handle_restart(self.repository, request),
            "sessions" => handlers::session::handle_sessions(self.repository, request),
            "resize" => handlers::session::handle_resize(self.repository, request),
            "attach" => handlers::session::handle_attach(self.repository, request),
            "cleanup" => handlers::session::handle_cleanup(self.repository, request),
            "assert" => handlers::session::handle_assert(self.repository, request),
            "snapshot" => handlers::snapshot::handle_snapshot_uc(self.repository, request),
            "keystroke" => handlers::input::handle_keystroke_uc(self.repository, request),
            "keydown" => handlers::input::handle_keydown_uc(self.repository, request),
            "keyup" => handlers::input::handle_keyup_uc(self.repository, request),
            "type" => handlers::input::handle_type_uc(self.repository, request),
            "wait" => handlers::wait::handle_wait_uc(self.repository, self.clock, request),

            "pty_write" => {
                handlers::diagnostics::handle_terminal_write_uc(self.repository, request)
            }
            "shutdown" => handlers::diagnostics::handle_shutdown_uc(
                self.shutdown_flag,
                self.shutdown_notifier,
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
