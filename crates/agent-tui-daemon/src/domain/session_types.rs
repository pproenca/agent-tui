//! Domain types for session management.
//!
//! These types belong to the domain layer and should not depend on infrastructure.
//!
//! IMPORTANT: Domain types must not depend on framework crates like serde or uuid.
//! Serialization is handled by adapter layers at the boundaries.

/// Unique identifier for a session.
///
/// This is a value object that wraps a string identifier. ID generation
/// happens in the infrastructure layer (see `IdGenerator`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new SessionId from a string.
    ///
    /// Use this when accepting IDs from external sources or from
    /// infrastructure-generated IDs.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the session ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

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

    // Note: test_session_info_to_json was moved to adapters/snapshot_adapters.rs
    // since serialization is now handled at the adapter layer.
}
