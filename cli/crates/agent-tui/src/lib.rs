#![deny(clippy::all)]

//! Facade crate for the `agent-tui` binary and public CLI command API.

#[cfg(not(unix))]
compile_error!(
    "agent-tui is Unix-only. Supported environments are Linux, macOS, and other Unix-like systems with PTYs, Unix domain sockets, and POSIX signals."
);

pub use agent_tui_app::run;

/// Build the clap command for doc generation and tooling.
pub fn cli_command() -> clap::Command {
    agent_tui_app::cli_command()
}
