//! Session identifier and terminal size types.

use std::fmt;
use std::ops::Deref;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionIdError {
    #[error("Session ID cannot be empty or whitespace-only")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn try_new(id: impl Into<String>) -> Result<Self, SessionIdError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SessionIdError::Empty);
        }
        Ok(Self(id))
    }

    /// Convenience constructor that panics on invalid (empty/whitespace) input.
    /// Use `try_new()` when handling untrusted input.
    #[allow(clippy::expect_used)]
    pub fn new(id: impl Into<String>) -> Self {
        Self::try_new(id).expect("SessionId must not be empty")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Deref<Target=str> is kept intentionally: it enables `.as_deref()` on
// `Option<SessionId>` throughout the codebase. The tradeoff (implicit &str
// coercion weakening the newtype boundary) is accepted for ergonomics.
impl Deref for SessionId {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TerminalSizeError {
    #[error("Columns ({cols}) must be at least {min}")]
    ColumnsTooSmall { cols: u16, min: u16 },
    #[error("Columns ({cols}) must be at most {max}")]
    ColumnsTooLarge { cols: u16, max: u16 },
    #[error("Rows ({rows}) must be at least {min}")]
    RowsTooSmall { rows: u16, min: u16 },
    #[error("Rows ({rows}) must be at most {max}")]
    RowsTooLarge { rows: u16, max: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    cols: u16,
    rows: u16,
}

impl TerminalSize {
    pub const MIN_COLS: u16 = 10;
    pub const MAX_COLS: u16 = 500;
    pub const MIN_ROWS: u16 = 2;
    pub const MAX_ROWS: u16 = 200;

    pub fn try_new(cols: u16, rows: u16) -> Result<Self, TerminalSizeError> {
        if cols < Self::MIN_COLS {
            return Err(TerminalSizeError::ColumnsTooSmall {
                cols,
                min: Self::MIN_COLS,
            });
        }
        if cols > Self::MAX_COLS {
            return Err(TerminalSizeError::ColumnsTooLarge {
                cols,
                max: Self::MAX_COLS,
            });
        }
        if rows < Self::MIN_ROWS {
            return Err(TerminalSizeError::RowsTooSmall {
                rows,
                min: Self::MIN_ROWS,
            });
        }
        if rows > Self::MAX_ROWS {
            return Err(TerminalSizeError::RowsTooLarge {
                rows,
                max: Self::MAX_ROWS,
            });
        }
        Ok(Self { cols, rows })
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn as_tuple(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub command: String,
    pub pid: u32,
    pub running: bool,
    pub created_at: String,
    pub size: TerminalSize,
}

impl SessionInfo {
    pub fn is_active(&self) -> bool {
        self.running
    }

    pub fn dimensions(&self) -> (u16, u16) {
        self.size.as_tuple()
    }

    pub fn cols(&self) -> u16 {
        self.size.cols()
    }

    pub fn rows(&self) -> u16 {
        self.size.rows()
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id = SessionId::new("test123");
        assert_eq!(id.as_str(), "test123");
    }

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

    #[test]
    fn test_session_info_creation() {
        let info = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: TerminalSize::default(),
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
            size: TerminalSize::default(),
        };
        assert!(running.is_active());

        let stopped = SessionInfo {
            id: SessionId::new("test2"),
            command: "bash".to_string(),
            pid: 1235,
            running: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: TerminalSize::default(),
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
            size: TerminalSize::try_new(120, 40).unwrap(),
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
            size: TerminalSize::default(),
        };
        assert_eq!(info.created_at(), "2024-01-01T12:30:45Z");
    }

    mod session_id_validation_tests {
        use super::*;

        #[test]
        fn test_session_id_rejects_empty_string() {
            let result = SessionId::try_new("");
            assert!(result.is_err(), "Empty string should be rejected");
        }

        #[test]
        fn test_session_id_rejects_whitespace_only() {
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
            assert!(SessionId::try_new("abc123").is_ok());
            assert!(SessionId::try_new("session-1").is_ok());
            assert!(SessionId::try_new("a").is_ok());
            assert!(SessionId::try_new("test_session").is_ok());
        }

        #[test]
        fn test_session_id_preserves_value() {
            let id = SessionId::try_new("my-session").unwrap();
            assert_eq!(id.as_str(), "my-session");
        }

        #[test]
        fn test_try_new_from_string_validates() {
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
            assert!(matches!(err, SessionIdError::Empty));
            assert!(err.to_string().contains("empty"));
        }
    }

    mod terminal_size_validation_tests {
        use super::*;

        #[test]
        fn test_terminal_size_rejects_zero_cols() {
            let result = TerminalSize::try_new(0, 24);
            assert!(result.is_err(), "Zero cols should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_zero_rows() {
            let result = TerminalSize::try_new(80, 0);
            assert!(result.is_err(), "Zero rows should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_both_zero() {
            let result = TerminalSize::try_new(0, 0);
            assert!(result.is_err(), "Both zero should be rejected");
        }

        #[test]
        fn test_terminal_size_accepts_valid() {
            let size = TerminalSize::try_new(80, 24).expect("Valid size should be accepted");
            assert_eq!(size.cols(), 80);
            assert_eq!(size.rows(), 24);
        }

        #[test]
        fn test_terminal_size_accepts_minimum() {
            let size = TerminalSize::try_new(10, 2).expect("Minimum size should be accepted");
            assert_eq!(size.cols(), 10);
            assert_eq!(size.rows(), 2);
        }

        #[test]
        fn test_terminal_size_rejects_below_minimum_cols() {
            let result = TerminalSize::try_new(9, 24);
            assert!(result.is_err(), "Below minimum cols should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_below_minimum_rows() {
            let result = TerminalSize::try_new(80, 1);
            assert!(result.is_err(), "Below minimum rows should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_too_large_cols() {
            let result = TerminalSize::try_new(501, 24);
            assert!(result.is_err(), "Cols > 500 should be rejected");
        }

        #[test]
        fn test_terminal_size_rejects_too_large_rows() {
            let result = TerminalSize::try_new(80, 201);
            assert!(result.is_err(), "Rows > 200 should be rejected");
        }

        #[test]
        fn test_terminal_size_accepts_maximum() {
            let size = TerminalSize::try_new(500, 200).expect("Maximum size should be accepted");
            assert_eq!(size.cols(), 500);
            assert_eq!(size.rows(), 200);
        }

        #[test]
        fn test_terminal_size_as_tuple() {
            let size = TerminalSize::try_new(120, 40).unwrap();
            assert_eq!(size.as_tuple(), (120, 40));
        }
    }
}
