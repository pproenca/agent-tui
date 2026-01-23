use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{ClickInput, ClickOutput, FillInput, FillOutput, FindInput, FindOutput};
use crate::error::SessionError;
use crate::repository::SessionRepository;

/// Use case for clicking an element.
pub trait ClickUseCase: Send + Sync {
    fn execute(&self, input: ClickInput) -> Result<ClickOutput, SessionError>;
}

/// Implementation of the click use case.
pub struct ClickUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ClickUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ClickUseCase for ClickUseCaseImpl<R> {
    fn execute(&self, input: ClickInput) -> Result<ClickOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.click(&input.element_ref)?;

        Ok(ClickOutput {
            success: true,
            message: None,
            warning: None,
        })
    }
}

/// Use case for filling an element with text.
pub trait FillUseCase: Send + Sync {
    fn execute(&self, input: FillInput) -> Result<FillOutput, SessionError>;
}

/// Implementation of the fill use case.
pub struct FillUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FillUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FillUseCase for FillUseCaseImpl<R> {
    fn execute(&self, input: FillInput) -> Result<FillOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;

        // Click on the element first to focus it
        session_guard.click(&input.element_ref)?;

        // Clear existing content and type new value
        session_guard.keystroke("ctrl+a")?;
        session_guard.type_text(&input.value)?;

        Ok(FillOutput {
            success: true,
            message: None,
        })
    }
}

/// Use case for finding elements.
pub trait FindUseCase: Send + Sync {
    fn execute(&self, input: FindInput) -> Result<FindOutput, SessionError>;
}

/// Implementation of the find use case.
pub struct FindUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FindUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FindUseCase for FindUseCaseImpl<R> {
    fn execute(&self, input: FindInput) -> Result<FindOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        let all_elements = session_guard.detect_elements();

        let filtered: Vec<_> = all_elements
            .iter()
            .filter(|el| {
                // Filter by role/element_type if specified
                if let Some(ref role) = input.role {
                    let el_type = format!("{:?}", el.element_type).to_lowercase();
                    if !el_type.contains(&role.to_lowercase()) {
                        return false;
                    }
                }

                // Filter by name/label if specified
                if let Some(ref name) = input.name {
                    let el_label = el.label.as_deref().unwrap_or("");
                    if input.exact {
                        if el_label != name {
                            return false;
                        }
                    } else if !el_label.to_lowercase().contains(&name.to_lowercase()) {
                        return false;
                    }
                }

                // Filter by text if specified
                if let Some(ref text) = input.text {
                    let el_text = el.label.as_deref().unwrap_or("").to_lowercase();
                    if !el_text.contains(&text.to_lowercase()) {
                        return false;
                    }
                }

                // Filter by focused if specified
                if let Some(focused) = input.focused {
                    if el.focused != focused {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Handle nth selection
        let elements = if let Some(nth) = input.nth {
            if nth < filtered.len() {
                vec![filtered[nth].clone()]
            } else {
                vec![]
            }
        } else {
            filtered
        };

        let count = elements.len();

        Ok(FindOutput { elements, count })
    }
}
