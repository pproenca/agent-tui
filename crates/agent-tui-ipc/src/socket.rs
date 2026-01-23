use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var("AGENT_TUI_SOCKET") {
        return PathBuf::from(custom_path);
    }

    std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("agent-tui.sock"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/agent-tui.sock"))
}
