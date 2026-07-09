#![deny(clippy::all)]
#![cfg_attr(
    test,
    allow(dead_code, reason = "test fixtures expose inspection-only helpers")
)]

//! Application composition and command handling crate.

#[cfg(not(unix))]
compile_error!(
    "agent-tui is Unix-only. Supported environments are Linux, macOS, and other Unix-like systems with PTYs, Unix domain sockets, and POSIX signals."
);

use clap::CommandFactory;

pub mod app;
pub use app::*;

/// Build the clap command exposed by the facade crate and documentation tooling.
pub fn cli_command() -> clap::Command {
    app::commands::Cli::command()
}

pub mod common {
    pub use agent_tui_common::common::*;
}

pub mod domain {
    pub use agent_tui_domain::domain::*;
}

pub mod usecases {
    pub use agent_tui_usecases::usecases::*;
}

pub mod adapters {
    pub use agent_tui_adapters::adapters::*;
}

pub mod infra {
    pub use agent_tui_infra::infra::*;
}

#[cfg(test)]
pub(crate) mod test_support;
