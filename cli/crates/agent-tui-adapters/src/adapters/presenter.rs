#![expect(clippy::print_stdout, reason = "CLI output is emitted here")]
#![expect(clippy::print_stderr, reason = "CLI output is emitted here")]

//! CLI output presenter.

use crate::adapters::RpcValue;
use crate::common::color;
use clap::ValueEnum;

/// Output format for CLI commands
#[derive(Clone, Copy, Debug, ValueEnum, Default, PartialEq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presenter {
    Text,
    Json,
}

impl From<OutputFormat> for Presenter {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Text => Self::Text,
            OutputFormat::Json => Self::Json,
        }
    }
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

const PROGRAM_NAME: &str = "agent-tui";

impl Presenter {
    pub fn present_success(&self, message: &str, warning: Option<&str>) {
        match self {
            Self::Text => {
                println!("{} {}", color::success("✓"), message);
                if let Some(warning) = warning {
                    eprintln!("{} {}", color::dim("Warning:"), warning);
                }
            }
            Self::Json => {
                let mut output = serde_json::json!({
                    "success": true,
                    "message": message
                });
                if let Some(warning) = warning {
                    output["warning"] = serde_json::json!(warning);
                }
                println!("{output:#}");
            }
        }
    }

    pub fn present_error(&self, message: &str) {
        match self {
            Self::Text => {
                eprintln!("{}: {} {}", PROGRAM_NAME, color::error("Error:"), message);
            }
            Self::Json => {
                let output = serde_json::json!({
                    "success": false,
                    "error": message
                });
                eprintln!("{output:#}");
            }
        }
    }

    pub fn present_value(&self, value: &RpcValue) {
        match self {
            Self::Text => {
                let value_ref = value.as_ref();
                if let Some(value) = value_ref.as_str() {
                    println!("{value}");
                } else if let Some(value) = value_ref.as_u64() {
                    println!("{value}");
                } else if let Some(value) = value_ref.as_bool() {
                    println!("{value}");
                } else {
                    println!("{}", value.to_pretty_json());
                }
            }
            Self::Json => println!("{}", value.to_pretty_json()),
        }
    }

    pub fn present_client_error(&self, error: &ClientErrorView) {
        match self {
            Self::Text => {
                eprintln!(
                    "{}: {} {}",
                    PROGRAM_NAME,
                    color::error("Error:"),
                    error.message
                );
                if let Some(suggestion) = error.suggestion.as_deref() {
                    eprintln!("{} {}", color::dim("Suggestion:"), suggestion);
                }
                if error.retryable {
                    eprintln!(
                        "{}",
                        color::dim("(This error may be transient - retry may succeed)")
                    );
                }
            }
            Self::Json => {
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
                eprintln!("{output:#}");
            }
        }
    }

    pub fn present_kv(&self, key: &str, value: &str) {
        match self {
            Self::Text => println!("  {key}: {value}"),
            Self::Json => {
                let output = serde_json::json!({ (key): value });
                println!("{output:#}");
            }
        }
    }

    pub fn present_session_id(&self, session_id: &str, label: Option<&str>) {
        match self {
            Self::Text => {
                if let Some(label) = label {
                    println!("{} {}", label, color::session_id(session_id));
                } else {
                    println!("{}", color::session_id(session_id));
                }
            }
            Self::Json => {
                let output = if let Some(label) = label {
                    serde_json::json!({ "label": label, "session_id": session_id })
                } else {
                    serde_json::json!({ "session_id": session_id })
                };
                println!("{output:#}");
            }
        }
    }

    pub fn present_list_header(&self, title: &str) {
        if matches!(self, Self::Text) {
            println!("{}", color::bold(title));
        }
    }

    pub fn present_list_item(&self, item: &str) {
        match self {
            Self::Text => println!("  {item}"),
            Self::Json => {
                let output = serde_json::Value::String(item.to_string());
                println!("{output:#}");
            }
        }
    }

    pub fn present_info(&self, message: &str) {
        match self {
            Self::Text => println!("{}", color::dim(message)),
            Self::Json => {
                let output = serde_json::json!({ "info": message });
                println!("{output:#}");
            }
        }
    }

    pub fn present_header(&self, text: &str) {
        if matches!(self, Self::Text) {
            println!("{}", color::bold(text));
        }
    }

    pub fn present_raw(&self, text: &str) {
        match self {
            Self::Text => println!("{text}"),
            Self::Json => {
                let output = serde_json::json!({ "output": text });
                println!("{output:#}");
            }
        }
    }

    pub fn present_wait_result(&self, result: &WaitResult) {
        match self {
            Self::Text if result.found => println!("Found after {}ms", result.elapsed_ms),
            Self::Text => println!("Timeout after {}ms - not found", result.elapsed_ms),
            Self::Json => {
                let output = serde_json::json!({
                    "found": result.found,
                    "elapsed_ms": result.elapsed_ms
                });
                println!("{output:#}");
            }
        }
    }

    pub fn present_assert_result(&self, result: &AssertResult) {
        match self {
            Self::Text if result.passed => {
                println!(
                    "{} Assertion passed: {}",
                    color::success("✓"),
                    result.condition
                );
            }
            Self::Text => {
                eprintln!(
                    "{}: {} Assertion failed: {}",
                    PROGRAM_NAME,
                    color::error("Error:"),
                    result.condition
                );
            }
            Self::Json => {
                let output = serde_json::json!({
                    "condition": result.condition,
                    "passed": result.passed
                });
                println!("{output:#}");
            }
        }
    }

    pub fn present_cleanup(&self, result: &CleanupResult) {
        match self {
            Self::Text => {
                if result.cleaned > 0 {
                    println!(
                        "{} Cleaned up {} session(s)",
                        color::success("Done:"),
                        result.cleaned
                    );
                } else if result.failures.is_empty() {
                    println!("{}", color::dim("No sessions to clean up"));
                }

                if !result.failures.is_empty() {
                    eprintln!();
                    eprintln!(
                        "{}: {} Failed to clean up {} session(s):",
                        PROGRAM_NAME,
                        color::error("Error:"),
                        result.failures.len()
                    );
                    for failure in &result.failures {
                        eprintln!(
                            "  {}: {}",
                            color::session_id(&failure.session_id),
                            failure.error
                        );
                    }
                }
            }
            Self::Json => {
                let failures: Vec<_> = result
                    .failures
                    .iter()
                    .map(|failure| {
                        serde_json::json!({
                            "session": failure.session_id,
                            "error": failure.error
                        })
                    })
                    .collect();
                let output = serde_json::json!({
                    "sessions_cleaned": result.cleaned,
                    "sessions_failed": result.failures.len(),
                    "failures": failures
                });
                println!("{output:#}");
            }
        }
    }
}

pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
}

impl SpawnResult {
    pub fn present(&self, presenter: &Presenter) {
        presenter.present_session_id(&self.session_id, Some(&color::success("Session started:")));
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
    pub fn present(&self, presenter: &Presenter) {
        if self.sessions.is_empty() {
            presenter.present_info("No active sessions");
        } else {
            presenter.present_list_header("Active sessions:");
            for session in &self.sessions {
                let is_active = self.active_session.as_ref() == Some(&session.id);
                let active_marker = if is_active {
                    color::success(" (active)")
                } else {
                    String::new()
                };
                let status = if session.running {
                    color::success("running")
                } else {
                    color::error("stopped")
                };
                let item = format!(
                    "{} - {} [{}] {}x{} pid:{}{}",
                    color::session_id(&session.id),
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
