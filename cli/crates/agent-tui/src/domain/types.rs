//! Domain types and request/response DTOs.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use super::session_types::SessionId;
use super::session_types::SessionInfo;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid wait condition type '{invalid_value}'. Must be one of: text, stable, text_gone")]
pub struct WaitConditionTypeError {
    pub invalid_value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitConditionType {
    Text,
    Stable,
    TextGone,
}

impl WaitConditionType {
    pub fn parse(s: &str) -> Result<Self, WaitConditionTypeError> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "stable" => Ok(Self::Stable),
            "text_gone" => Ok(Self::TextGone),
            _ => Err(WaitConditionTypeError {
                invalid_value: s.to_string(),
            }),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Stable => "stable",
            Self::TextGone => "text_gone",
        }
    }

    pub fn requires_target(&self) -> bool {
        false
    }

    pub fn requires_text(&self) -> bool {
        matches!(self, Self::Text | Self::TextGone)
    }
}

impl fmt::Display for WaitConditionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for WaitConditionType {
    type Err = WaitConditionTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DomainCursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct SpawnInput {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub session_id: Option<SessionId>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct SpawnOutput {
    pub session_id: SessionId,
    pub pid: u32,
}

#[derive(Debug, Clone)]
pub struct RestartOutput {
    pub old_session_id: SessionId,
    pub new_session_id: SessionId,
    pub command: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotInput {
    pub session_id: Option<SessionId>,
    pub region: Option<String>,
    pub strip_ansi: bool,
    pub include_cursor: bool,
    pub include_render: bool,
}

#[derive(Debug, Clone)]
pub struct SnapshotOutput {
    pub session_id: SessionId,
    pub screenshot: String,
    pub cursor: Option<DomainCursorPosition>,
    pub rendered: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeystrokeInput {
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeystrokeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInput {
    pub session_id: Option<SessionId>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TypeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeydownInput {
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeydownOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeyupInput {
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeyupOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct WaitInput {
    pub session_id: Option<SessionId>,
    pub text: Option<String>,
    pub timeout_ms: u64,
    pub condition: Option<WaitConditionType>,
}

#[derive(Debug, Clone)]
pub struct WaitOutput {
    pub found: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ResizeInput {
    pub session_id: Option<SessionId>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct ResizeOutput {
    pub session_id: SessionId,
    pub success: bool,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct SessionsOutput {
    pub sessions: Vec<SessionInfo>,
    pub active_session: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct KillOutput {
    pub session_id: SessionId,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct GetTitleOutput {
    pub session_id: SessionId,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct TerminalReadInput {
    pub session_id: Option<SessionId>,
    pub max_bytes: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TerminalReadOutput {
    pub session_id: SessionId,
    pub data: Vec<u8>,
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct TerminalWriteInput {
    pub session_id: Option<SessionId>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TerminalWriteOutput {
    pub session_id: SessionId,
    pub bytes_written: usize,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SessionInput {
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct AttachInput {
    pub session_id: SessionId,
}

#[derive(Debug, Clone)]
pub struct AttachOutput {
    pub session_id: SessionId,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStartInput {
    pub session_id: Option<SessionId>,
    pub listen_addr: Option<String>,
    pub allow_remote: bool,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStartOutput {
    pub session_id: SessionId,
    pub listen_addr: String,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStopOutput {
    pub stopped: bool,
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStatusOutput {
    pub running: bool,
    pub session_id: Option<SessionId>,
    pub listen_addr: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HealthInput;

#[derive(Debug, Clone)]
pub struct HealthOutput {
    pub status: String,
    pub pid: u32,
    pub uptime_ms: u64,
    pub session_count: usize,
    pub version: String,
    pub commit: String,
    pub active_connections: usize,
    pub total_requests: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MetricsInput;

#[derive(Debug, Clone)]
pub struct MetricsOutput {
    pub requests_total: u64,
    pub errors_total: u64,
    pub lock_timeouts: u64,
    pub poison_recoveries: u64,
    pub uptime_ms: u64,
    pub active_connections: usize,
    pub session_count: usize,
}

#[derive(Debug, Clone)]
pub struct CleanupInput {
    pub all: bool,
}

#[derive(Debug, Clone)]
pub struct CleanupFailure {
    pub session_id: SessionId,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct CleanupOutput {
    pub cleaned: usize,
    pub failures: Vec<CleanupFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertConditionType {
    Text,
    Session,
}

impl AssertConditionType {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "session" => Ok(Self::Session),
            _ => Err(format!(
                "Unknown condition type: {}. Use: text or session",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Session => "session",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssertInput {
    pub session_id: Option<SessionId>,
    pub condition_type: AssertConditionType,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct AssertOutput {
    pub passed: bool,
    pub condition: String,
}

#[derive(Debug, Clone, Default)]
pub struct ShutdownInput;

#[derive(Debug, Clone)]
pub struct ShutdownOutput {
    pub acknowledged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod wait_condition_type_tests {
        use super::*;

        #[test]
        fn test_wait_condition_type_from_str_text() {
            let cond = WaitConditionType::parse("text").expect("Should parse 'text'");
            assert_eq!(cond, WaitConditionType::Text);
        }

        #[test]
        fn test_wait_condition_type_from_str_stable() {
            let cond = WaitConditionType::parse("stable").expect("Should parse 'stable'");
            assert_eq!(cond, WaitConditionType::Stable);
        }

        #[test]
        fn test_wait_condition_type_from_str_text_gone() {
            let cond = WaitConditionType::parse("text_gone").expect("Should parse 'text_gone'");
            assert_eq!(cond, WaitConditionType::TextGone);
        }

        #[test]
        fn test_wait_condition_type_from_str_invalid() {
            let result = WaitConditionType::parse("invalid");
            assert!(result.is_err(), "Invalid condition should be rejected");
        }

        #[test]
        fn test_wait_condition_type_from_str_empty() {
            let result = WaitConditionType::parse("");
            assert!(result.is_err(), "Empty string should be rejected");
        }

        #[test]
        fn test_wait_condition_type_case_insensitive() {
            assert_eq!(
                WaitConditionType::parse("TEXT").unwrap(),
                WaitConditionType::Text
            );
            assert_eq!(
                WaitConditionType::parse("STABLE").unwrap(),
                WaitConditionType::Stable
            );
        }

        #[test]
        fn test_wait_condition_type_as_str() {
            assert_eq!(WaitConditionType::Text.as_str(), "text");
            assert_eq!(WaitConditionType::Stable.as_str(), "stable");
            assert_eq!(WaitConditionType::TextGone.as_str(), "text_gone");
        }

        #[test]
        fn test_wait_condition_type_display() {
            assert_eq!(format!("{}", WaitConditionType::Text), "text");
            assert_eq!(format!("{}", WaitConditionType::Stable), "stable");
        }

        #[test]
        fn test_wait_condition_type_requires_target() {
            assert!(!WaitConditionType::Text.requires_target());
            assert!(!WaitConditionType::Stable.requires_target());
            assert!(!WaitConditionType::TextGone.requires_target());
        }

        #[test]
        fn test_wait_condition_type_requires_text() {
            assert!(WaitConditionType::Text.requires_text());
            assert!(!WaitConditionType::Stable.requires_text());
            assert!(WaitConditionType::TextGone.requires_text());
        }

        #[test]
        fn test_wait_condition_type_error_message() {
            let err = WaitConditionType::parse("invalid").unwrap_err();
            assert!(err.to_string().contains("invalid"));
            assert!(err.to_string().contains("text"));
        }
    }

    mod assert_condition_type_tests {
        use super::*;

        #[test]
        fn test_assert_condition_type_parse_text() {
            assert_eq!(
                AssertConditionType::parse("text").unwrap(),
                AssertConditionType::Text
            );
        }

        #[test]
        fn test_assert_condition_type_parse_session() {
            assert_eq!(
                AssertConditionType::parse("session").unwrap(),
                AssertConditionType::Session
            );
        }

        #[test]
        fn test_assert_condition_type_parse_invalid() {
            assert!(AssertConditionType::parse("invalid").is_err());
        }
    }
}
