#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use agent_tui::Application;

fn main() {
    let app = Application::new();
    std::process::exit(app.run());
}
