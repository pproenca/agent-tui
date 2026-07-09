//! CLI entrypoint for agent-tui.

use agent_tui::Application;
use anyhow::Result;

#[cfg(not(unix))]
compile_error!(
    "agent-tui is Unix-only. Supported environments are Linux, macOS, and other Unix-like systems with PTYs, Unix domain sockets, and POSIX signals."
);

fn main() -> Result<()> {
    let app = Application::new();
    let exit_code = app.run()?;
    std::process::exit(exit_code);
}
