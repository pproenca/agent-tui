use serde_json::Value;

use crate::common::Colors;
use crate::common::ValueExt;
use crate::infra::ipc::ClientError;

pub trait Presenter {
    fn present_success(&self, message: &str, warning: Option<&str>);

    fn present_error(&self, message: &str);

    fn present_value(&self, value: &Value);

    fn present_client_error(&self, error: &ClientError);

    fn present_kv(&self, key: &str, value: &str);

    fn present_session_id(&self, session_id: &str, label: Option<&str>);

    fn present_element_ref(&self, element_ref: &str, info: Option<&str>);

    fn present_list_header(&self, title: &str);

    fn present_list_item(&self, item: &str);

    fn present_info(&self, message: &str);

    fn present_header(&self, text: &str);

    fn present_raw(&self, text: &str);

    fn present_wait_result(&self, result: &WaitResult);

    fn present_assert_result(&self, result: &AssertResult);

    fn present_health(&self, health: &HealthResult);

    fn present_cleanup(&self, result: &CleanupResult);

    fn present_find(&self, result: &FindResult);
}

pub struct WaitResult {
    pub found: bool,
    pub elapsed_ms: u64,
}

impl WaitResult {
    pub fn from_json(value: &Value) -> Self {
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

pub struct HealthResult {
    pub status: String,
    pub pid: u64,
    pub uptime_ms: u64,
    pub session_count: u64,
    pub version: String,
    pub commit: String,
    pub socket_path: Option<String>,
    pub pid_file_path: Option<String>,
}

impl HealthResult {
    pub fn from_json(value: &Value, verbose: bool) -> Self {
        use crate::infra::ipc::socket_path;

        let (socket, pid_file) = if verbose {
            let socket = socket_path();
            let pid_file = socket.with_extension("pid");
            (
                Some(socket.display().to_string()),
                Some(pid_file.display().to_string()),
            )
        } else {
            (None, None)
        };

        Self {
            status: value.str_or("status", "unknown").to_string(),
            pid: value.u64_or("pid", 0),
            uptime_ms: value.u64_or("uptime_ms", 0),
            session_count: value.u64_or("session_count", 0),
            version: value.str_or("version", "?").to_string(),
            commit: value.str_or("commit", "unknown").to_string(),
            socket_path: socket,
            pid_file_path: pid_file,
        }
    }
}

pub struct CleanupResult {
    pub cleaned: usize,
    pub failures: Vec<CleanupFailure>,
}

pub struct CleanupFailure {
    pub session_id: String,
    pub error: String,
}

pub struct FindResult {
    pub count: u64,
    pub elements: Vec<ElementInfo>,
}

impl FindResult {
    pub fn from_json(value: &Value) -> Self {
        let count = value.u64_or("count", 0);
        let elements = value
            .get("elements")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|el| ElementInfo {
                        element_ref: el.str_or("ref", "").to_string(),
                        element_type: el.str_or("type", "").to_string(),
                        label: el.str_or("label", "").to_string(),
                        focused: el.bool_or("focused", false),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self { count, elements }
    }
}

pub struct ElementInfo {
    pub element_ref: String,
    pub element_type: String,
    pub label: String,
    pub focused: bool,
}

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

    fn present_wait_result(&self, result: &WaitResult) {
        if result.found {
            println!("Found after {}ms", result.elapsed_ms);
        } else {
            eprintln!("Timeout after {}ms - not found", result.elapsed_ms);
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
                "{} Assertion failed: {}",
                Colors::error("✗"),
                result.condition
            );
        }
    }

    fn present_health(&self, health: &HealthResult) {
        println!(
            "{} {}",
            Colors::bold("Daemon status:"),
            Colors::success(&health.status)
        );
        println!("  PID: {}", health.pid);
        println!("  Uptime: {}", format_uptime_ms(health.uptime_ms));
        println!("  Sessions: {}", health.session_count);
        println!("  Version: {}", Colors::dim(&health.version));
        println!("  Commit: {}", Colors::dim(&health.commit));

        if let (Some(socket), Some(pid_file)) = (&health.socket_path, &health.pid_file_path) {
            println!();
            println!("{}", Colors::bold("Connection:"));
            println!("  Socket: {}", socket);
            println!("  PID file: {}", pid_file);
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
                "{} Failed to clean up {} session(s):",
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

    fn present_find(&self, result: &FindResult) {
        if result.count == 0 {
            println!("{}", Colors::dim("No elements found"));
        } else {
            println!(
                "{} Found {} element(s):",
                Colors::success("✓"),
                result.count
            );
            for el in &result.elements {
                let focused = if el.focused {
                    Colors::success(" *focused*")
                } else {
                    String::new()
                };
                println!(
                    "  {} [{}:{}]{}",
                    Colors::element_ref(&el.element_ref),
                    el.element_type,
                    el.label,
                    focused
                );
            }
        }
    }
}

fn format_uptime_ms(uptime_ms: u64) -> String {
    let secs = uptime_ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins % 60, secs % 60)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs % 60)
    } else {
        format!("{}s", secs)
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

    fn present_list_header(&self, _title: &str) {}

    fn present_list_item(&self, item: &str) {
        println!("\"{}\"", item);
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

    fn present_health(&self, health: &HealthResult) {
        let mut output = serde_json::json!({
            "status": health.status,
            "pid": health.pid,
            "uptime_ms": health.uptime_ms,
            "session_count": health.session_count,
            "version": health.version,
            "commit": health.commit
        });
        if let Some(socket) = &health.socket_path {
            output["socket_path"] = serde_json::json!(socket);
        }
        if let Some(pid_file) = &health.pid_file_path {
            output["pid_file_path"] = serde_json::json!(pid_file);
        }
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

    fn present_find(&self, result: &FindResult) {
        let elements: Vec<_> = result
            .elements
            .iter()
            .map(|el| {
                serde_json::json!({
                    "ref": el.element_ref,
                    "type": el.element_type,
                    "label": el.label,
                    "focused": el.focused
                })
            })
            .collect();
        let output = serde_json::json!({
            "count": result.count,
            "elements": elements
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    }
}

pub fn create_presenter(format: &crate::app::commands::OutputFormat) -> Box<dyn Presenter> {
    match format {
        crate::app::commands::OutputFormat::Json => Box::new(JsonPresenter),
        crate::app::commands::OutputFormat::Text => Box::new(TextPresenter),
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

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "session_id": self.session_id,
            "pid": self.pid
        })
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

pub struct ElementView<'a>(pub &'a Value);

impl ElementView<'_> {
    pub fn ref_str(&self) -> &str {
        self.0.str_or("ref", "")
    }

    pub fn el_type(&self) -> &str {
        self.0.str_or("type", "")
    }

    pub fn label(&self) -> &str {
        self.0.str_or("label", "")
    }

    pub fn focused(&self) -> bool {
        self.0.bool_or("focused", false)
    }

    pub fn selected(&self) -> bool {
        self.0.bool_or("selected", false)
    }

    pub fn value(&self) -> Option<&str> {
        self.0.get("value").and_then(|v| v.as_str())
    }

    pub fn position(&self) -> (u64, u64) {
        let pos = self.0.get("position");
        let row = pos
            .and_then(|p| p.get("row"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let col = pos
            .and_then(|p| p.get("col"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        (row, col)
    }

    pub fn focused_indicator(&self) -> String {
        if self.focused() {
            Colors::success(" *focused*")
        } else {
            String::new()
        }
    }

    pub fn selected_indicator(&self) -> String {
        if self.selected() {
            Colors::info(" *selected*")
        } else {
            String::new()
        }
    }

    pub fn label_suffix(&self) -> String {
        if self.label().is_empty() {
            String::new()
        } else {
            format!(":{}", self.label())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_presenter_success() {
        let presenter = TextPresenter;

        presenter.present_success("Test message", None);
        presenter.present_success("Test with warning", Some("Warning text"));
    }

    #[test]
    fn test_json_presenter_success() {
        let presenter = JsonPresenter;

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

    #[test]
    fn test_wait_result_struct() {
        let result = WaitResult {
            found: true,
            elapsed_ms: 150,
        };
        assert!(result.found);
        assert_eq!(result.elapsed_ms, 150);
    }

    #[test]
    fn test_assert_result_struct() {
        let result = AssertResult {
            passed: true,
            condition: "text:hello".to_string(),
        };
        assert!(result.passed);
        assert_eq!(result.condition, "text:hello");
    }

    #[test]
    fn test_health_result_struct() {
        let result = HealthResult {
            status: "healthy".to_string(),
            pid: 1234,
            uptime_ms: 60000,
            session_count: 5,
            version: "0.3.0".to_string(),
            commit: "abc1234".to_string(),
            socket_path: Some("/tmp/agent-tui.sock".to_string()),
            pid_file_path: None,
        };
        assert_eq!(result.status, "healthy");
        assert_eq!(result.session_count, 5);
    }

    #[test]
    fn test_cleanup_result_struct() {
        let result = CleanupResult {
            cleaned: 3,
            failures: vec![CleanupFailure {
                session_id: "sess1".to_string(),
                error: "session not found".to_string(),
            }],
        };
        assert_eq!(result.cleaned, 3);
        assert_eq!(result.failures.len(), 1);
    }

    #[test]
    fn test_find_result_struct() {
        let result = FindResult {
            count: 2,
            elements: vec![
                ElementInfo {
                    element_ref: "@btn1".to_string(),
                    element_type: "button".to_string(),
                    label: "Submit".to_string(),
                    focused: true,
                },
                ElementInfo {
                    element_ref: "@btn2".to_string(),
                    element_type: "button".to_string(),
                    label: "Cancel".to_string(),
                    focused: false,
                },
            ],
        };
        assert_eq!(result.count, 2);
        assert_eq!(result.elements.len(), 2);
        assert!(result.elements[0].focused);
    }

    #[test]
    fn test_json_presenter_wait_result() {
        let presenter = JsonPresenter;
        let result = WaitResult {
            found: true,
            elapsed_ms: 100,
        };

        presenter.present_wait_result(&result);
    }

    #[test]
    fn test_json_presenter_assert_result() {
        let presenter = JsonPresenter;
        let result = AssertResult {
            passed: true,
            condition: "element:@btn1".to_string(),
        };

        presenter.present_assert_result(&result);
    }

    #[test]
    fn test_json_presenter_health() {
        let presenter = JsonPresenter;
        let health = HealthResult {
            status: "healthy".to_string(),
            pid: 1234,
            uptime_ms: 60000,
            session_count: 2,
            version: "0.3.0".to_string(),
            commit: "abc1234".to_string(),
            socket_path: None,
            pid_file_path: None,
        };

        presenter.present_health(&health);
    }

    #[test]
    fn test_json_presenter_cleanup() {
        let presenter = JsonPresenter;
        let result = CleanupResult {
            cleaned: 2,
            failures: vec![],
        };

        presenter.present_cleanup(&result);
    }

    #[test]
    fn test_json_presenter_find() {
        let presenter = JsonPresenter;
        let result = FindResult {
            count: 1,
            elements: vec![ElementInfo {
                element_ref: "@inp1".to_string(),
                element_type: "input".to_string(),
                label: "Email".to_string(),
                focused: false,
            }],
        };

        presenter.present_find(&result);
    }

    #[test]
    fn test_format_uptime_ms_seconds() {
        assert_eq!(format_uptime_ms(5000), "5s");
        assert_eq!(format_uptime_ms(45000), "45s");
    }

    #[test]
    fn test_format_uptime_ms_minutes() {
        assert_eq!(format_uptime_ms(60000), "1m 0s");
        assert_eq!(format_uptime_ms(90000), "1m 30s");
        assert_eq!(format_uptime_ms(300000), "5m 0s");
    }

    #[test]
    fn test_format_uptime_ms_hours() {
        assert_eq!(format_uptime_ms(3600000), "1h 0m 0s");
        assert_eq!(format_uptime_ms(5400000), "1h 30m 0s");
        assert_eq!(format_uptime_ms(7265000), "2h 1m 5s");
    }
}
