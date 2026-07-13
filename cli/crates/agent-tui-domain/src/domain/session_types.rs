//! Session identifier and terminal size types.

use serde::Deserialize;
use serde::Serialize;
use std::borrow::Borrow;
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

    /// Unchecked constructor for trusted internal call sites that already
    /// guarantee the invariant.
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        let id = id.into();
        debug_assert!(
            !id.trim().is_empty(),
            "SessionId::new_unchecked requires a non-empty identifier"
        );
        Self(id)
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

impl Borrow<str> for SessionId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionId {
    type Error = SessionIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<&str> for SessionId {
    type Error = SessionIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TerminalSizeWire", into = "TerminalSizeWire")]
pub struct TerminalSize {
    cols: u16,
    rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TerminalSizeWire {
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

impl TryFrom<TerminalSizeWire> for TerminalSize {
    type Error = TerminalSizeError;

    fn try_from(value: TerminalSizeWire) -> Result<Self, Self::Error> {
        Self::try_new(value.cols, value.rows)
    }
}

impl From<TerminalSize> for TerminalSizeWire {
    fn from(value: TerminalSize) -> Self {
        Self {
            cols: value.cols,
            rows: value.rows,
        }
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

#[cfg(test)]
#[path = "session_types_tests.rs"]
mod tests;
