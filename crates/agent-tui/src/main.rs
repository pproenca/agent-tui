use agent_tui::Application;

fn main() {
    let app = Application::new();
    std::process::exit(app.run());
}
