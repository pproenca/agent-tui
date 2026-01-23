use serde_json::Value;

use agent_tui_common::Colors;
use agent_tui_ipc::ClientError;

/// Trait for presenting output to the user.
///
/// This trait abstracts the output formatting, allowing the CLI to support
/// multiple output formats (text, JSON) without duplicating logic in handlers.
pub trait Presenter {
    /// Present a success result with optional warning.
    fn present_success(&self, message: &str, warning: Option<&str>);

    /// Present an error message.
    fn present_error(&self, message: &str);

    /// Present a structured value (for JSON output, shows the raw value).
    fn present_value(&self, value: &Value);

    /// Present a client error with suggestions.
    fn present_client_error(&self, error: &ClientError);

    /// Present a simple key-value pair.
    fn present_kv(&self, key: &str, value: &str);

    /// Present a session ID.
    fn present_session_id(&self, session_id: &str, label: Option<&str>);

    /// Present an element reference.
    fn present_element_ref(&self, element_ref: &str, info: Option<&str>);

    /// Present a list header.
    fn present_list_header(&self, title: &str);

    /// Present a list item.
    fn present_list_item(&self, item: &str);

    /// Present a dim/info message.
    fn present_info(&self, message: &str);

    /// Present a bold header.
    fn present_header(&self, text: &str);

    /// Present raw text without formatting.
    fn present_raw(&self, text: &str);
}

/// Text presenter for human-readable output.
pub struct TextPresenter;

impl Presenter for TextPresenter {
    fn present_success(&self, message: &str, warning: Option<&str>) {
        println!("{} {}", Colors::success("✓"), message);
        if let Some(w) = warning {
            eprintln!("{} {}", Colors::dim("Warning:"), w);
        }
    }

    fn present_error(&self, message: &str) {
        eprintln!("{} {}", Colors::error("Error:"), message);
    }

    fn present_value(&self, value: &Value) {
        if let Some(s) = value.as_str() {
            println!("{}", s);
        } else if let Some(n) = value.as_u64() {
            println!("{}", n);
        } else if let Some(b) = value.as_bool() {
            println!("{}", b);
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
    }

    fn present_client_error(&self, error: &ClientError) {
        eprintln!("{} {}", Colors::error("Error:"), error);
        if let Some(suggestion) = error.suggestion() {
            eprintln!("{} {}", Colors::dim("Suggestion:"), suggestion);
        }
        if error.is_retryable() {
            eprintln!(
                "{}",
                Colors::dim("(This error may be transient - retry may succeed)")
            );
        }
    }

    fn present_kv(&self, key: &str, value: &str) {
        println!("  {}: {}", key, value);
    }

    fn present_session_id(&self, session_id: &str, label: Option<&str>) {
        if let Some(l) = label {
            println!("{} {}", l, Colors::session_id(session_id));
        } else {
            println!("{}", Colors::session_id(session_id));
        }
    }

    fn present_element_ref(&self, element_ref: &str, info: Option<&str>) {
        if let Some(i) = info {
            println!("{} {}", Colors::element_ref(element_ref), i);
        } else {
            println!("{}", Colors::element_ref(element_ref));
        }
    }

    fn present_list_header(&self, title: &str) {
        println!("{}", Colors::bold(title));
    }

    fn present_list_item(&self, item: &str) {
        println!("  {}", item);
    }

    fn present_info(&self, message: &str) {
        println!("{}", Colors::dim(message));
    }

    fn present_header(&self, text: &str) {
        println!("{}", Colors::bold(text));
    }

    fn present_raw(&self, text: &str) {
        println!("{}", text);
    }
}

/// JSON presenter for machine-readable output.
pub struct JsonPresenter;

impl Presenter for JsonPresenter {
    fn present_success(&self, message: &str, warning: Option<&str>) {
        let mut output = serde_json::json!({
            "success": true,
            "message": message
        });
        if let Some(w) = warning {
            output["warning"] = serde_json::json!(w);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_error(&self, message: &str) {
        let output = serde_json::json!({
            "success": false,
            "error": message
        });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_value(&self, value: &Value) {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    }

    fn present_client_error(&self, error: &ClientError) {
        eprintln!("{}", error.to_json());
    }

    fn present_kv(&self, key: &str, value: &str) {
        let output = serde_json::json!({ key: value });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_session_id(&self, session_id: &str, label: Option<&str>) {
        let output = if let Some(l) = label {
            serde_json::json!({ "label": l, "session_id": session_id })
        } else {
            serde_json::json!({ "session_id": session_id })
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_element_ref(&self, element_ref: &str, info: Option<&str>) {
        let output = if let Some(i) = info {
            serde_json::json!({ "ref": element_ref, "info": i })
        } else {
            serde_json::json!({ "ref": element_ref })
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_list_header(&self, _title: &str) {
        // No-op for JSON - the structure conveys the meaning
    }

    fn present_list_item(&self, item: &str) {
        // In JSON mode, we'd typically collect items and output as array
        // For simple cases, output each item
        println!("\"{}\"", item);
    }

    fn present_info(&self, message: &str) {
        let output = serde_json::json!({ "info": message });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_header(&self, _text: &str) {
        // No-op for JSON
    }

    fn present_raw(&self, text: &str) {
        // For JSON, wrap in a structure
        let output = serde_json::json!({ "output": text });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }
}

/// Create a presenter based on the output format.
pub fn create_presenter(format: &crate::commands::OutputFormat) -> Box<dyn Presenter> {
    match format {
        crate::commands::OutputFormat::Json => Box::new(JsonPresenter),
        crate::commands::OutputFormat::Text => Box::new(TextPresenter),
    }
}

/// Helper struct for presenting spawn results.
pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
}

impl SpawnResult {
    pub fn present(&self, presenter: &dyn Presenter) {
        presenter.present_session_id(&self.session_id, Some(&Colors::success("Session started:")));
        presenter.present_kv("PID", &self.pid.to_string());
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "session_id": self.session_id,
            "pid": self.pid
        })
    }
}

/// Helper struct for presenting session list results.
pub struct SessionListResult {
    pub sessions: Vec<SessionListItem>,
    pub active_session: Option<String>,
}

pub struct SessionListItem {
    pub id: String,
    pub command: String,
    pub pid: u64,
    pub running: bool,
    pub cols: u64,
    pub rows: u64,
}

impl SessionListResult {
    pub fn present(&self, presenter: &dyn Presenter) {
        if self.sessions.is_empty() {
            presenter.present_info("No active sessions");
        } else {
            presenter.present_list_header("Active sessions:");
            for session in &self.sessions {
                let is_active = self.active_session.as_ref() == Some(&session.id);
                let active_marker = if is_active {
                    Colors::success(" (active)")
                } else {
                    String::new()
                };
                let status = if session.running {
                    Colors::success("running")
                } else {
                    Colors::error("stopped")
                };
                let item = format!(
                    "{} - {} [{}] {}x{} pid:{}{}",
                    Colors::session_id(&session.id),
                    session.command,
                    status,
                    session.cols,
                    session.rows,
                    session.pid,
                    active_marker
                );
                presenter.present_list_item(&item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_presenter_success() {
        let presenter = TextPresenter;
        // Just verify it doesn't panic
        presenter.present_success("Test message", None);
        presenter.present_success("Test with warning", Some("Warning text"));
    }

    #[test]
    fn test_json_presenter_success() {
        let presenter = JsonPresenter;
        // Just verify it doesn't panic
        presenter.present_success("Test message", None);
        presenter.present_success("Test with warning", Some("Warning text"));
    }

    #[test]
    fn test_text_presenter_error() {
        let presenter = TextPresenter;
        presenter.present_error("Test error");
    }

    #[test]
    fn test_json_presenter_error() {
        let presenter = JsonPresenter;
        presenter.present_error("Test error");
    }

    #[test]
    fn test_spawn_result_to_json() {
        let result = SpawnResult {
            session_id: "abc123".to_string(),
            pid: 1234,
        };
        let json = result.to_json();
        assert_eq!(json["session_id"], "abc123");
        assert_eq!(json["pid"], 1234);
    }
}
