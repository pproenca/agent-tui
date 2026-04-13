#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI output presenter.

use crate::adapters::RpcValue;
use crate::common::Colors;
use clap::ValueEnum;

/// Output format for CLI commands
#[derive(Clone, Copy, Debug, ValueEnum, Default, PartialEq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

pub trait Presenter {
    fn present_success(&self, message: &str, warning: Option<&str>);

    fn present_error(&self, message: &str);

    fn present_value(&self, value: &RpcValue);

    fn present_client_error(&self, error: &ClientErrorView);

    fn present_kv(&self, key: &str, value: &str);

    fn present_session_id(&self, session_id: &str, label: Option<&str>);

    fn present_list_header(&self, title: &str);

    fn present_list_item(&self, item: &str);

    fn present_info(&self, message: &str);

    fn present_header(&self, text: &str);

    fn present_raw(&self, text: &str);

    fn present_wait_result(&self, result: &WaitResult);

    fn present_assert_result(&self, result: &AssertResult);

    fn present_cleanup(&self, result: &CleanupResult);
}

#[derive(Clone, Debug)]
pub struct ClientErrorView {
    pub message: String,
    pub suggestion: Option<String>,
    pub retryable: bool,
    pub json: Option<String>,
}

pub struct WaitResult {
    pub found: bool,
    pub elapsed_ms: u64,
}

impl WaitResult {
    pub fn from_json(value: &RpcValue) -> Self {
        Self {
            found: value.bool_or("found", false),
            elapsed_ms: value.u64_or("elapsed_ms", 0),
        }
    }
}

pub struct AssertResult {
    pub passed: bool,
    pub condition: String,
}

pub struct CleanupResult {
    pub cleaned: usize,
    pub failures: Vec<CleanupFailure>,
}

pub struct CleanupFailure {
    pub session_id: String,
    pub error: String,
}

pub struct TextPresenter;
const PROGRAM_NAME: &str = "agent-tui";

impl Presenter for TextPresenter {
    fn present_success(&self, message: &str, warning: Option<&str>) {
        println!("{} {}", Colors::success("✓"), message);
        if let Some(w) = warning {
            eprintln!("{} {}", Colors::dim("Warning:"), w);
        }
    }

    fn present_error(&self, message: &str) {
        eprintln!("{}: {} {}", PROGRAM_NAME, Colors::error("Error:"), message);
    }

    fn present_value(&self, value: &RpcValue) {
        let value_ref = value.as_ref();
        if let Some(s) = value_ref.as_str() {
            println!("{s}");
        } else if let Some(n) = value_ref.as_u64() {
            println!("{n}");
        } else if let Some(b) = value_ref.as_bool() {
            println!("{b}");
        } else {
            println!("{}", value.to_pretty_json());
        }
    }

    fn present_client_error(&self, error: &ClientErrorView) {
        eprintln!(
            "{}: {} {}",
            PROGRAM_NAME,
            Colors::error("Error:"),
            error.message
        );
        if let Some(suggestion) = error.suggestion.as_deref() {
            eprintln!("{} {}", Colors::dim("Suggestion:"), suggestion);
        }
        if error.retryable {
            eprintln!(
                "{}",
                Colors::dim("(This error may be transient - retry may succeed)")
            );
        }
    }

    fn present_kv(&self, key: &str, value: &str) {
        println!("  {key}: {value}");
    }

    fn present_session_id(&self, session_id: &str, label: Option<&str>) {
        if let Some(l) = label {
            println!("{} {}", l, Colors::session_id(session_id));
        } else {
            println!("{}", Colors::session_id(session_id));
        }
    }

    fn present_list_header(&self, title: &str) {
        println!("{}", Colors::bold(title));
    }

    fn present_list_item(&self, item: &str) {
        println!("  {item}");
    }

    fn present_info(&self, message: &str) {
        println!("{}", Colors::dim(message));
    }

    fn present_header(&self, text: &str) {
        println!("{}", Colors::bold(text));
    }

    fn present_raw(&self, text: &str) {
        println!("{text}");
    }

    fn present_wait_result(&self, result: &WaitResult) {
        if result.found {
            println!("Found after {}ms", result.elapsed_ms);
        } else {
            println!("Timeout after {}ms - not found", result.elapsed_ms);
        }
    }

    fn present_assert_result(&self, result: &AssertResult) {
        if result.passed {
            println!(
                "{} Assertion passed: {}",
                Colors::success("✓"),
                result.condition
            );
        } else {
            eprintln!(
                "{}: {} Assertion failed: {}",
                PROGRAM_NAME,
                Colors::error("Error:"),
                result.condition
            );
        }
    }

    fn present_cleanup(&self, result: &CleanupResult) {
        if result.cleaned > 0 {
            println!(
                "{} Cleaned up {} session(s)",
                Colors::success("Done:"),
                result.cleaned
            );
        } else if result.failures.is_empty() {
            println!("{}", Colors::dim("No sessions to clean up"));
        }

        if !result.failures.is_empty() {
            eprintln!();
            eprintln!(
                "{}: {} Failed to clean up {} session(s):",
                PROGRAM_NAME,
                Colors::error("Error:"),
                result.failures.len()
            );
            for failure in &result.failures {
                eprintln!(
                    "  {}: {}",
                    Colors::session_id(&failure.session_id),
                    failure.error
                );
            }
        }
    }
}

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

    fn present_value(&self, value: &RpcValue) {
        println!("{}", value.to_pretty_json());
    }

    fn present_client_error(&self, error: &ClientErrorView) {
        if let Some(json) = error.json.as_deref() {
            eprintln!("{json}");
            return;
        }

        let mut output = serde_json::json!({
            "success": false,
            "error": error.message,
            "retryable": error.retryable,
        });
        if let Some(suggestion) = error.suggestion.as_ref() {
            output["suggestion"] = serde_json::json!(suggestion);
        }
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
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

    fn present_list_header(&self, _title: &str) {}

    fn present_list_item(&self, item: &str) {
        println!("\"{item}\"");
    }

    fn present_info(&self, message: &str) {
        let output = serde_json::json!({ "info": message });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_header(&self, _text: &str) {}

    fn present_raw(&self, text: &str) {
        let output = serde_json::json!({ "output": text });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_wait_result(&self, result: &WaitResult) {
        let output = serde_json::json!({
            "found": result.found,
            "elapsed_ms": result.elapsed_ms
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_assert_result(&self, result: &AssertResult) {
        let output = serde_json::json!({
            "condition": result.condition,
            "passed": result.passed
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }

    fn present_cleanup(&self, result: &CleanupResult) {
        let failures: Vec<_> = result
            .failures
            .iter()
            .map(|f| {
                serde_json::json!({
                    "session": f.session_id,
                    "error": f.error
                })
            })
            .collect();
        let output = serde_json::json!({
            "sessions_cleaned": result.cleaned,
            "sessions_failed": result.failures.len(),
            "failures": failures
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }
}

pub fn create_presenter(format: &OutputFormat) -> Box<dyn Presenter> {
    match format {
        OutputFormat::Json => Box::new(JsonPresenter),
        OutputFormat::Text => Box::new(TextPresenter),
    }
}

pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
}

impl SpawnResult {
    pub fn present(&self, presenter: &dyn Presenter) {
        presenter.present_session_id(&self.session_id, Some(&Colors::success("Session started:")));
        presenter.present_kv("PID", &self.pid.to_string());
    }

    pub fn to_json(&self) -> RpcValue {
        RpcValue::new(serde_json::json!({
            "session_id": self.session_id,
            "pid": self.pid
        }))
    }
}

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
#[path = "presenter_tests.rs"]
mod tests;
