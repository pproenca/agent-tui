use super::*;

#[test]
fn test_session_id_try_new() {
    let id = SessionId::try_new("test123").expect("valid session id");
    assert_eq!(id.as_str(), "test123");
}

#[test]
fn test_session_id_display() {
    let id = SessionId::try_new("abc123").expect("valid session id");
    assert_eq!(format!("{id}"), "abc123");
}

#[test]
fn test_session_id_try_from_string() {
    let id = SessionId::try_from("test".to_string()).expect("valid session id");
    assert_eq!(id.as_str(), "test");
}

#[test]
fn test_session_id_try_from_str() {
    let id = SessionId::try_from("test").expect("valid session id");
    assert_eq!(id.as_str(), "test");
}

#[test]
fn test_session_id_as_ref() {
    let id = SessionId::try_new("test").expect("valid session id");
    let s: &str = id.as_ref();
    assert_eq!(s, "test");
}

#[test]
fn test_session_info_creation() {
    let info = SessionInfo {
        id: SessionId::try_new("test").expect("valid session id"),
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
        id: SessionId::try_new("test").expect("valid session id"),
        command: "bash".to_string(),
        pid: 1234,
        running: true,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::default(),
    };
    assert!(running.is_active());

    let stopped = SessionInfo {
        id: SessionId::try_new("test2").expect("valid session id"),
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
        id: SessionId::try_new("test").expect("valid session id"),
        command: "bash".to_string(),
        pid: 1234,
        running: true,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::try_new(120, 40).expect("valid terminal size"),
    };
    assert_eq!(info.dimensions(), (120, 40));
    assert_eq!(info.cols(), 120);
    assert_eq!(info.rows(), 40);
}

#[test]
fn test_session_info_created_at() {
    let info = SessionInfo {
        id: SessionId::try_new("test").expect("valid session id"),
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
        let id = SessionId::try_new("my-session").expect("valid session id");
        assert_eq!(id.as_str(), "my-session");
    }

    #[test]
    fn test_try_new_from_string_validates() {
        assert!(
            SessionId::try_new(String::new()).is_err(),
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
        let err = SessionId::try_new("").expect_err("empty session id should be rejected");
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
        let size = TerminalSize::try_new(120, 40).expect("valid terminal size");
        assert_eq!(size.as_tuple(), (120, 40));
    }
}
