//! CLI entrypoint for agent-tui.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use agent_tui::Application;
use anyhow::Result;

fn main() -> Result<()> {
    let app = Application::new();
    let exit_code = app.run()?;
    std::process::exit(exit_code);
}
