#![deny(clippy::all)]
#![allow(dead_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! Facade crate for the `agent-tui` binary and public CLI command API.

use clap::CommandFactory;

pub use agent_tui_app::app::Application;

/// Build the clap command for doc generation and tooling.
pub fn cli_command() -> clap::Command {
    agent_tui_app::app::commands::Cli::command()
}
