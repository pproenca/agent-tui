pub const SESSION_NOT_FOUND: i32 = -32001;
pub const NO_ACTIVE_SESSION: i32 = -32002;
pub const SESSION_LIMIT: i32 = -32006;
pub const LOCK_TIMEOUT: i32 = -32007;

pub const ELEMENT_NOT_FOUND: i32 = -32003;
pub const WRONG_ELEMENT_TYPE: i32 = -32004;

pub const INVALID_KEY: i32 = -32005;
pub const PTY_ERROR: i32 = -32008;

pub const WAIT_TIMEOUT: i32 = -32013;

pub const COMMAND_NOT_FOUND: i32 = -32014;
pub const PERMISSION_DENIED: i32 = -32015;

pub const DAEMON_ERROR: i32 = -32016;
pub const PERSISTENCE_ERROR: i32 = -32017;

pub const GENERIC_ERROR: i32 = -32000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    NotFound,
    InvalidInput,
    Busy,
    Internal,
    External,
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

pub fn is_retryable(code: i32) -> bool {
    matches!(code, LOCK_TIMEOUT | GENERIC_ERROR)
}

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
