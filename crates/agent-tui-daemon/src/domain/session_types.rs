//! Domain types for session management.
//!
//! These types belong to the domain layer and should not depend on infrastructure.
//!
//! IMPORTANT: Domain types must not depend on framework crates like serde or uuid.
//! Serialization is handled by adapter layers at the boundaries.

use std::fmt;
use std::ops::Deref;

/// Error returned when SessionId validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdError {
    pub message: String,
}

impl fmt::Display for SessionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SessionIdError {}

/// Unique identifier for a session.
///
/// This is a value object that wraps a string identifier. ID generation
/// happens in the infrastructure layer (see `IdGenerator`).
///
/// # Invariants
/// - Session ID must not be empty
/// - Session ID must not be whitespace-only
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new SessionId from a string, validating that it's not empty.
    ///
    /// Use this when accepting IDs from external sources or from
    /// infrastructure-generated IDs.
    ///
    /// # Errors
    /// Returns `SessionIdError` if the ID is empty or whitespace-only.
    pub fn try_new(id: impl Into<String>) -> Result<Self, SessionIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SessionIdError {
                message: "Session ID cannot be empty or whitespace-only".to_string(),
            });
        }
        Ok(Self(id))
    }

    /// Create a new SessionId without validation.
    ///
    /// # Safety
    /// This bypasses validation. Use only when the ID is known to be valid
    /// (e.g., from infrastructure ID generation).
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the session ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for SessionId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Keep From implementations for backwards compatibility with infrastructure code
// that generates valid IDs. These bypass validation - use try_new() for
// user-provided IDs that need validation.
impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Error returned when TerminalSize validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSizeError {
    pub message: String,
}

impl fmt::Display for TerminalSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TerminalSizeError {}

/// Terminal dimensions with validation.
///
/// # Invariants
/// - Columns must be between 10 and 500
/// - Rows must be between 2 and 200
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    cols: u16,
    rows: u16,
}

impl TerminalSize {
    /// Minimum number of columns.
    pub const MIN_COLS: u16 = 10;
    /// Maximum number of columns.
    pub const MAX_COLS: u16 = 500;
    /// Minimum number of rows.
    pub const MIN_ROWS: u16 = 2;
    /// Maximum number of rows.
    pub const MAX_ROWS: u16 = 200;

    /// Create a new TerminalSize with validation.
    ///
    /// # Errors
    /// Returns `TerminalSizeError` if dimensions are outside valid bounds.
    pub fn new(cols: u16, rows: u16) -> Result<Self, TerminalSizeError> {
        if cols < Self::MIN_COLS {
            return Err(TerminalSizeError {
                message: format!("Columns ({}) must be at least {}", cols, Self::MIN_COLS),
            });
        }
        if cols > Self::MAX_COLS {
            return Err(TerminalSizeError {
                message: format!("Columns ({}) must be at most {}", cols, Self::MAX_COLS),
            });
        }
        if rows < Self::MIN_ROWS {
            return Err(TerminalSizeError {
                message: format!("Rows ({}) must be at least {}", rows, Self::MIN_ROWS),
            });
        }
        if rows > Self::MAX_ROWS {
            return Err(TerminalSizeError {
                message: format!("Rows ({}) must be at most {}", rows, Self::MAX_ROWS),
            });
        }
        Ok(Self { cols, rows })
    }

    /// Get the number of columns.
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Get the number of rows.
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Get dimensions as a tuple (cols, rows).
    pub fn as_tuple(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        // Safe to unwrap - 80x24 is within valid bounds
        Self::new(80, 24).unwrap()
    }
}

/// A single frame in a screen recording.
#[derive(Clone, Debug)]
pub struct RecordingFrame {
    pub timestamp_ms: u64,
    pub screen: String,
}

/// Status of the recording feature.
pub struct RecordingStatus {
    pub is_recording: bool,
    pub frame_count: usize,
    pub duration_ms: u64,
}

/// A single entry in the trace log.
#[derive(Clone, Debug)]
pub struct TraceEntry {
    pub timestamp_ms: u64,
    pub action: String,
    pub details: Option<String>,
}

/// A single error entry.
#[derive(Clone, Debug)]
pub struct ErrorEntry {
    pub timestamp: String,
    pub message: String,
    pub source: String,
}

/// Information about a session.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub command: String,
    pub pid: u32,
    pub running: bool,
    pub created_at: String,
    pub size: (u16, u16),
}

impl SessionInfo {
    /// Returns true if the session is currently active/running.
    pub fn is_active(&self) -> bool {
        self.running
    }

    /// Returns the terminal dimensions as (columns, rows).
    pub fn dimensions(&self) -> (u16, u16) {
        self.size
    }

    /// Returns the number of columns in the terminal.
    pub fn cols(&self) -> u16 {
        self.size.0
    }

    /// Returns the number of rows in the terminal.
    pub fn rows(&self) -> u16 {
        self.size.1
    }

    /// Returns the creation timestamp as a string.
    ///
    /// Note: Age calculation requires datetime parsing which is handled
    /// by the adapter layer to keep domain types framework-independent.
    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

// Note: SessionInfo serialization is handled by session_info_to_json()
// in the adapters layer to keep domain types framework-independent.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id = SessionId::new("test123");
        assert_eq!(id.as_str(), "test123");
    }

    // Note: SessionId::generate() was moved to infrastructure layer.
    // See IdGenerator in session.rs for ID generation tests.

    #[test]
    fn test_session_id_display() {
        let id = SessionId::new("abc123");
        assert_eq!(format!("{}", id), "abc123");
    }

    #[test]
    fn test_session_id_from_string() {
        let id: SessionId = "test".to_string().into();
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn test_session_id_from_str() {
        let id: SessionId = "test".into();
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn test_session_id_as_ref() {
        let id = SessionId::new("test");
        let s: &str = id.as_ref();
        assert_eq!(s, "test");
    }

    // Note: SessionId serde serialization was removed from domain layer.
    // Serialization happens via adapters using .as_str() at boundaries.

    #[test]
    fn test_recording_frame_creation() {
        let frame = RecordingFrame {
            timestamp_ms: 100,
            screen: "test screen".to_string(),
        };
        assert_eq!(frame.timestamp_ms, 100);
        assert_eq!(frame.screen, "test screen");
    }

    #[test]
    fn test_recording_status_creation() {
        let status = RecordingStatus {
            is_recording: true,
            frame_count: 10,
            duration_ms: 5000,
        };
        assert!(status.is_recording);
        assert_eq!(status.frame_count, 10);
        assert_eq!(status.duration_ms, 5000);
    }

    #[test]
    fn test_trace_entry_creation() {
        let entry = TraceEntry {
            timestamp_ms: 200,
            action: "click".to_string(),
            details: Some("button1".to_string()),
        };
        assert_eq!(entry.timestamp_ms, 200);
        assert_eq!(entry.action, "click");
        assert_eq!(entry.details, Some("button1".to_string()));
    }

    #[test]
    fn test_error_entry_creation() {
        let entry = ErrorEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            message: "test error".to_string(),
            source: "test".to_string(),
        };
        assert_eq!(entry.timestamp, "2024-01-01T00:00:00Z");
        assert_eq!(entry.message, "test error");
        assert_eq!(entry.source, "test");
    }

    #[test]
    fn test_session_info_creation() {
        let info = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
        };
        assert_eq!(info.id.as_str(), "test");
        assert_eq!(info.command, "bash");
        assert_eq!(info.pid, 1234);
        assert!(info.running);
    }

    #[test]
    fn test_session_info_is_active() {
        let running = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
        };
        assert!(running.is_active());

        let stopped = SessionInfo {
            id: SessionId::new("test2"),
            command: "bash".to_string(),
            pid: 1235,
            running: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
        };
        assert!(!stopped.is_active());
    }

    #[test]
    fn test_session_info_dimensions() {
        let info = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (120, 40),
        };
        assert_eq!(info.dimensions(), (120, 40));
        assert_eq!(info.cols(), 120);
        assert_eq!(info.rows(), 40);
    }

    #[test]
    fn test_session_info_created_at() {
        let info = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T12:30:45Z".to_string(),
            size: (80, 24),
        };
        assert_eq!(info.created_at(), "2024-01-01T12:30:45Z");
    }

    // Note: test_session_info_to_json was moved to adapters/snapshot_adapters.rs
    // since serialization is now handled at the adapter layer.

    // ============================================================
    // TDD RED PHASE: SessionId Validation Tests
    // These tests should FAIL until validation is implemented.
    // ============================================================

    mod session_id_validation_tests {
        use super::*;

        #[test]
        fn test_session_id_rejects_empty_string() {
            // SessionId::new("") should return Err, not Ok
            let result = SessionId::try_new("");
            assert!(result.is_err(), "Empty string should be rejected");
        }

        #[test]
        fn test_session_id_rejects_whitespace_only() {
            // Whitespace-only strings should be rejected
            assert!(
                SessionId::try_new("   ").is_err(),
                "Whitespace-only should be rejected"
            );
            assert!(
                SessionId::try_new("\t\n").is_err(),
                "Tab/newline should be rejected"
            );
            assert!(
                SessionId::try_new("  \t  ").is_err(),
                "Mixed whitespace should be rejected"
            );
        }

        #[test]
        fn test_session_id_accepts_valid_id() {
            // Valid IDs should be accepted
            assert!(SessionId::try_new("abc123").is_ok());
            assert!(SessionId::try_new("session-1").is_ok());
            assert!(SessionId::try_new("a").is_ok()); // Single char is valid
            assert!(SessionId::try_new("test_session").is_ok());
        }

        #[test]
        fn test_session_id_preserves_value() {
            let id = SessionId::try_new("my-session").unwrap();
            assert_eq!(id.as_str(), "my-session");
        }

        #[test]
        fn test_try_new_from_string_validates() {
            // try_new should validate String input
            assert!(
                SessionId::try_new("".to_string()).is_err(),
                "try_new should validate String"
            );
            assert!(
                SessionId::try_new("valid".to_string()).is_ok(),
                "try_new should accept valid String"
            );
        }

        #[test]
        fn test_try_new_from_str_validates() {
            // try_new should validate &str input
            assert!(
                SessionId::try_new("").is_err(),
                "try_new should validate &str"
            );
            assert!(
                SessionId::try_new("valid").is_ok(),
                "try_new should accept valid &str"
            );
        }

        #[test]
        fn test_error_has_message() {
            let err = SessionId::try_new("").unwrap_err();
            assert!(!err.message.is_empty());
            assert!(err.to_string().contains("empty"));
        }
    }

    // ============================================================
    // TDD RED PHASE: TerminalSize Validation Tests
    // These tests should FAIL until TerminalSize is implemented.
    // ============================================================

    mod terminal_size_validation_tests {
        use super::*;

        #[test]
        fn test_terminal_size_rejects_zero_cols() {
            let result = TerminalSize::new(0, 24);
            assert!(result.is_err(), "Zero cols should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_zero_rows() {
            let result = TerminalSize::new(80, 0);
            assert!(result.is_err(), "Zero rows should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_both_zero() {
            let result = TerminalSize::new(0, 0);
            assert!(result.is_err(), "Both zero should be rejected");
        }

        #[test]
        fn test_terminal_size_accepts_valid() {
            let size = TerminalSize::new(80, 24).expect("Valid size should be accepted");
            assert_eq!(size.cols(), 80);
            assert_eq!(size.rows(), 24);
        }

        #[test]
        fn test_terminal_size_accepts_minimum() {
            // Minimum reasonable terminal size
            let size = TerminalSize::new(10, 2).expect("Minimum size should be accepted");
            assert_eq!(size.cols(), 10);
            assert_eq!(size.rows(), 2);
        }

        #[test]
        fn test_terminal_size_rejects_below_minimum_cols() {
            let result = TerminalSize::new(9, 24);
            assert!(result.is_err(), "Below minimum cols should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_below_minimum_rows() {
            let result = TerminalSize::new(80, 1);
            assert!(result.is_err(), "Below minimum rows should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_too_large_cols() {
            let result = TerminalSize::new(501, 24);
            assert!(result.is_err(), "Cols > 500 should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_too_large_rows() {
            let result = TerminalSize::new(80, 201);
            assert!(result.is_err(), "Rows > 200 should be rejected");
        }

        #[test]
        fn test_terminal_size_accepts_maximum() {
            let size = TerminalSize::new(500, 200).expect("Maximum size should be accepted");
            assert_eq!(size.cols(), 500);
            assert_eq!(size.rows(), 200);
        }

        #[test]
        fn test_terminal_size_as_tuple() {
            let size = TerminalSize::new(120, 40).unwrap();
            assert_eq!(size.as_tuple(), (120, 40));
        }
    }
}
