//! Exit code definitions.

pub const SESSION_NOT_FOUND: i32 = -32001;
pub const NO_ACTIVE_SESSION: i32 = -32002;
pub const SESSION_LIMIT: i32 = -32006;
pub const LOCK_TIMEOUT: i32 = -32007;
pub const SESSION_ALREADY_EXISTS: i32 = -32018;
pub const INVALID_INPUT: i32 = -32019;

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
        SESSION_NOT_FOUND | NO_ACTIVE_SESSION => ErrorCategory::NotFound,
        INVALID_KEY | SESSION_ALREADY_EXISTS | INVALID_INPUT => ErrorCategory::InvalidInput,
        SESSION_LIMIT | LOCK_TIMEOUT => ErrorCategory::Busy,
        PTY_ERROR | COMMAND_NOT_FOUND | PERMISSION_DENIED | DAEMON_ERROR | PERSISTENCE_ERROR => {
            ErrorCategory::External
        }
        WAIT_TIMEOUT => ErrorCategory::Timeout,
        _ => ErrorCategory::Internal,
    }
}

#[cfg(test)]
#[path = "error_codes_tests.rs"]
mod tests;
