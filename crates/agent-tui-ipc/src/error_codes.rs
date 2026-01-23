//! Semantic error codes for JSON-RPC domain errors.
//!
//! Error codes follow the JSON-RPC 2.0 specification:
//! - -32700 to -32600: Reserved protocol errors
//! - -32000 to -32099: Server errors (we use -32001 to -32020 for domain errors)

// Session-related errors
pub const SESSION_NOT_FOUND: i32 = -32001;
pub const NO_ACTIVE_SESSION: i32 = -32002;
pub const SESSION_LIMIT: i32 = -32006;
pub const LOCK_TIMEOUT: i32 = -32007;

// Element-related errors
pub const ELEMENT_NOT_FOUND: i32 = -32003;
pub const WRONG_ELEMENT_TYPE: i32 = -32004;

// Input/operation errors
pub const INVALID_KEY: i32 = -32005;
pub const PTY_ERROR: i32 = -32008;

// Wait/timing errors
pub const WAIT_TIMEOUT: i32 = -32013;

// Process errors
pub const COMMAND_NOT_FOUND: i32 = -32014;
pub const PERMISSION_DENIED: i32 = -32015;

// Daemon errors
pub const DAEMON_ERROR: i32 = -32016;
pub const PERSISTENCE_ERROR: i32 = -32017;

// Legacy generic error (for backwards compatibility)
pub const GENERIC_ERROR: i32 = -32000;

/// Error category for programmatic handling by AI agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Resource not found (session, element)
    NotFound,
    /// Invalid input parameters
    InvalidInput,
    /// Resource busy or locked
    Busy,
    /// Internal server error
    Internal,
    /// External dependency failure (PTY, process)
    External,
    /// Operation timed out
    Timeout,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::NotFound => "not_found",
            ErrorCategory::InvalidInput => "invalid_input",
            ErrorCategory::Busy => "busy",
            ErrorCategory::Internal => "internal",
            ErrorCategory::External => "external",
            ErrorCategory::Timeout => "timeout",
        }
    }
}

impl std::str::FromStr for ErrorCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_found" => Ok(ErrorCategory::NotFound),
            "invalid_input" => Ok(ErrorCategory::InvalidInput),
            "busy" => Ok(ErrorCategory::Busy),
            "internal" => Ok(ErrorCategory::Internal),
            "external" => Ok(ErrorCategory::External),
            "timeout" => Ok(ErrorCategory::Timeout),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Returns whether an error code represents a retriable operation.
///
/// Retriable errors are transient conditions that may succeed on retry:
/// - Lock timeouts (another operation in progress)
/// - Connection issues (daemon busy)
pub fn is_retryable(code: i32) -> bool {
    matches!(code, LOCK_TIMEOUT | GENERIC_ERROR)
}

/// Returns the error category for a given error code.
pub fn category_for_code(code: i32) -> ErrorCategory {
    match code {
        SESSION_NOT_FOUND | NO_ACTIVE_SESSION | ELEMENT_NOT_FOUND => ErrorCategory::NotFound,
        WRONG_ELEMENT_TYPE | INVALID_KEY => ErrorCategory::InvalidInput,
        SESSION_LIMIT | LOCK_TIMEOUT => ErrorCategory::Busy,
        PTY_ERROR | COMMAND_NOT_FOUND | PERMISSION_DENIED | DAEMON_ERROR | PERSISTENCE_ERROR => {
            ErrorCategory::External
        }
        WAIT_TIMEOUT => ErrorCategory::Timeout,
        _ => ErrorCategory::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_lock_timeout() {
        assert!(is_retryable(LOCK_TIMEOUT));
    }

    #[test]
    fn test_is_retryable_generic() {
        assert!(is_retryable(GENERIC_ERROR));
    }

    #[test]
    fn test_not_retryable_session_not_found() {
        assert!(!is_retryable(SESSION_NOT_FOUND));
    }

    #[test]
    fn test_not_retryable_element_not_found() {
        assert!(!is_retryable(ELEMENT_NOT_FOUND));
    }

    #[test]
    fn test_category_for_code_not_found() {
        assert_eq!(
            category_for_code(SESSION_NOT_FOUND),
            ErrorCategory::NotFound
        );
        assert_eq!(
            category_for_code(NO_ACTIVE_SESSION),
            ErrorCategory::NotFound
        );
        assert_eq!(
            category_for_code(ELEMENT_NOT_FOUND),
            ErrorCategory::NotFound
        );
    }

    #[test]
    fn test_category_for_code_invalid_input() {
        assert_eq!(
            category_for_code(WRONG_ELEMENT_TYPE),
            ErrorCategory::InvalidInput
        );
        assert_eq!(category_for_code(INVALID_KEY), ErrorCategory::InvalidInput);
    }

    #[test]
    fn test_category_for_code_busy() {
        assert_eq!(category_for_code(SESSION_LIMIT), ErrorCategory::Busy);
        assert_eq!(category_for_code(LOCK_TIMEOUT), ErrorCategory::Busy);
    }

    #[test]
    fn test_category_for_code_external() {
        assert_eq!(category_for_code(PTY_ERROR), ErrorCategory::External);
        assert_eq!(
            category_for_code(COMMAND_NOT_FOUND),
            ErrorCategory::External
        );
        assert_eq!(
            category_for_code(PERMISSION_DENIED),
            ErrorCategory::External
        );
        assert_eq!(category_for_code(DAEMON_ERROR), ErrorCategory::External);
        assert_eq!(
            category_for_code(PERSISTENCE_ERROR),
            ErrorCategory::External
        );
    }

    #[test]
    fn test_category_for_code_timeout() {
        assert_eq!(category_for_code(WAIT_TIMEOUT), ErrorCategory::Timeout);
    }

    #[test]
    fn test_category_as_str() {
        assert_eq!(ErrorCategory::NotFound.as_str(), "not_found");
        assert_eq!(ErrorCategory::InvalidInput.as_str(), "invalid_input");
        assert_eq!(ErrorCategory::Busy.as_str(), "busy");
        assert_eq!(ErrorCategory::Internal.as_str(), "internal");
        assert_eq!(ErrorCategory::External.as_str(), "external");
        assert_eq!(ErrorCategory::Timeout.as_str(), "timeout");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(
            "not_found".parse::<ErrorCategory>(),
            Ok(ErrorCategory::NotFound)
        );
        assert_eq!(
            "invalid_input".parse::<ErrorCategory>(),
            Ok(ErrorCategory::InvalidInput)
        );
        assert_eq!("busy".parse::<ErrorCategory>(), Ok(ErrorCategory::Busy));
        assert_eq!(
            "internal".parse::<ErrorCategory>(),
            Ok(ErrorCategory::Internal)
        );
        assert_eq!(
            "external".parse::<ErrorCategory>(),
            Ok(ErrorCategory::External)
        );
        assert_eq!(
            "timeout".parse::<ErrorCategory>(),
            Ok(ErrorCategory::Timeout)
        );
        assert!("unknown".parse::<ErrorCategory>().is_err());
    }
}
