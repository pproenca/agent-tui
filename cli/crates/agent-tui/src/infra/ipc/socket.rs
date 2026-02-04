//! IPC socket path helpers.

use std::path::PathBuf;
use tracing::debug;

pub fn socket_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var("AGENT_TUI_SOCKET") {
        let path = PathBuf::from(custom_path);
        debug!(socket = %path.display(), "Using custom socket path");
        return path;
    }

    let path = std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("agent-tui.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/agent-tui.sock"));
    debug!(socket = %path.display(), "Resolved socket path");
    path
}
