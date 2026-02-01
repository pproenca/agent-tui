use crate::usecases::ports::{PtyError, SessionError};

#[derive(Debug, Clone, Default)]
pub enum MockError {
    #[default]
    NoActiveSession,
    NotFound(String),
    LimitReached(usize),
    Pty(String),
}

impl MockError {
    pub fn to_session_error(&self) -> SessionError {
        match self {
            MockError::NoActiveSession => SessionError::NoActiveSession,
            MockError::NotFound(id) => SessionError::NotFound(id.clone()),
            MockError::LimitReached(max) => SessionError::LimitReached(*max),
            MockError::Pty(message) => SessionError::Pty(PtyError::Spawn(message.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_error_conversion() {
        let err = MockError::NotFound("test".to_string());
        let session_err = err.to_session_error();
        assert!(matches!(session_err, SessionError::NotFound(id) if id == "test"));

        let err = MockError::LimitReached(10);
        let session_err = err.to_session_error();
        assert!(matches!(session_err, SessionError::LimitReached(10)));

        let err = MockError::NoActiveSession;
        let session_err = err.to_session_error();
        assert!(matches!(session_err, SessionError::NoActiveSession));
    }
}
