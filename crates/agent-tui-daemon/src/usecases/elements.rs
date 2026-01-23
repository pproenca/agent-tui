use std::sync::Arc;
use std::thread;
use std::time::Duration;

use agent_tui_common::mutex_lock_or_recover;

use crate::ansi_keys;
use crate::domain::{
    ClearInput, ClearOutput, ClickInput, ClickOutput, CountInput, CountOutput, DoubleClickInput,
    DoubleClickOutput, ElementStateInput, FillInput, FillOutput, FindInput, FindOutput,
    FocusCheckOutput, FocusInput, FocusOutput, GetFocusedOutput, GetTextOutput, GetTitleOutput,
    GetValueOutput, IsCheckedOutput, IsEnabledOutput, MultiselectInput, MultiselectOutput,
    ScrollInput, ScrollIntoViewInput, ScrollIntoViewOutput, ScrollOutput, SelectAllInput,
    SelectAllOutput, SelectInput, SelectOutput, ToggleInput, ToggleOutput, VisibilityOutput,
};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::select_helpers::navigate_to_option;

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

/// Use case for scrolling.
pub trait ScrollUseCase: Send + Sync {
    fn execute(&self, input: ScrollInput) -> Result<ScrollOutput, SessionError>;
}

/// Implementation of the scroll use case.
pub struct ScrollUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ScrollUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ScrollUseCase for ScrollUseCaseImpl<R> {
    fn execute(&self, input: ScrollInput) -> Result<ScrollOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let key_seq: &[u8] = match input.direction.as_str() {
            "up" => ansi_keys::UP,
            "down" => ansi_keys::DOWN,
            "left" => ansi_keys::LEFT,
            "right" => ansi_keys::RIGHT,
            _ => {
                return Err(SessionError::InvalidKey(format!(
                    "Invalid direction: {}",
                    input.direction
                )));
            }
        };

        for _ in 0..input.amount {
            session_guard.pty_write(key_seq)?;
        }

        Ok(ScrollOutput { success: true })
    }
}

/// Use case for counting elements.
pub trait CountUseCase: Send + Sync {
    fn execute(&self, input: CountInput) -> Result<CountOutput, SessionError>;
}

/// Implementation of the count use case.
pub struct CountUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> CountUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> CountUseCase for CountUseCaseImpl<R> {
    fn execute(&self, input: CountInput) -> Result<CountOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        let all_elements = session_guard.detect_elements();

        let count = all_elements
            .iter()
            .filter(|el| {
                if let Some(ref role) = input.role {
                    let el_type = format!("{:?}", el.element_type).to_lowercase();
                    if !el_type.contains(&role.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref name) = input.name {
                    let el_label = el.label.as_deref().unwrap_or("");
                    if !el_label.to_lowercase().contains(&name.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref text) = input.text {
                    let el_text = el.label.as_deref().unwrap_or("").to_lowercase();
                    if !el_text.contains(&text.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .count();

        Ok(CountOutput { count })
    }
}

/// Use case for double-clicking an element.
pub trait DoubleClickUseCase: Send + Sync {
    fn execute(&self, input: DoubleClickInput) -> Result<DoubleClickOutput, SessionError>;
}

/// Implementation of the double-click use case.
pub struct DoubleClickUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> DoubleClickUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> DoubleClickUseCase for DoubleClickUseCaseImpl<R> {
    fn execute(&self, input: DoubleClickInput) -> Result<DoubleClickOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        {
            let mut session_guard = mutex_lock_or_recover(&session);
            session_guard.update()?;
            session_guard.click(&input.element_ref)?;
        }

        thread::sleep(Duration::from_millis(50));

        {
            let mut session_guard = mutex_lock_or_recover(&session);
            session_guard.click(&input.element_ref)?;
        }

        Ok(DoubleClickOutput { success: true })
    }
}

/// Use case for focusing an element.
pub trait FocusUseCase: Send + Sync {
    fn execute(&self, input: FocusInput) -> Result<FocusOutput, SessionError>;
}

/// Implementation of the focus use case.
pub struct FocusUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FocusUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FocusUseCase for FocusUseCaseImpl<R> {
    fn execute(&self, input: FocusInput) -> Result<FocusOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        // Verify element exists
        if session_guard.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        // Send Tab to focus (standard terminal focus navigation)
        session_guard.pty_write(b"\t")?;

        Ok(FocusOutput { success: true })
    }
}

/// Use case for clearing an element's content.
pub trait ClearUseCase: Send + Sync {
    fn execute(&self, input: ClearInput) -> Result<ClearOutput, SessionError>;
}

/// Implementation of the clear use case.
pub struct ClearUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ClearUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ClearUseCase for ClearUseCaseImpl<R> {
    fn execute(&self, input: ClearInput) -> Result<ClearOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        // Verify element exists
        if session_guard.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        // Send Ctrl+U to clear line
        session_guard.pty_write(b"\x15")?;

        Ok(ClearOutput { success: true })
    }
}

/// Use case for selecting all content in an element.
pub trait SelectAllUseCase: Send + Sync {
    fn execute(&self, input: SelectAllInput) -> Result<SelectAllOutput, SessionError>;
}

/// Implementation of the select all use case.
pub struct SelectAllUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SelectAllUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SelectAllUseCase for SelectAllUseCaseImpl<R> {
    fn execute(&self, input: SelectAllInput) -> Result<SelectAllOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        // Verify element exists
        if session_guard.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        // Send Ctrl+A to select all
        session_guard.pty_write(b"\x01")?;

        Ok(SelectAllOutput { success: true })
    }
}

/// Use case for toggling a checkbox/radio.
pub trait ToggleUseCase: Send + Sync {
    fn execute(&self, input: ToggleInput) -> Result<ToggleOutput, SessionError>;
}

/// Implementation of the toggle use case.
pub struct ToggleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ToggleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ToggleUseCase for ToggleUseCaseImpl<R> {
    fn execute(&self, input: ToggleInput) -> Result<ToggleOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        let current_checked = match session_guard.find_element(&input.element_ref) {
            Some(el) => {
                let el_type = el.element_type.as_str();
                if el_type != "checkbox" && el_type != "radio" {
                    return Err(SessionError::WrongElementType {
                        element_ref: input.element_ref.clone(),
                        expected: "checkbox/radio".to_string(),
                        actual: el_type.to_string(),
                    });
                }
                el.checked.unwrap_or(false)
            }
            None => return Err(SessionError::ElementNotFound(input.element_ref)),
        };

        let should_toggle = input.state != Some(current_checked);
        let new_checked = if should_toggle {
            session_guard.pty_write(b" ")?;
            !current_checked
        } else {
            current_checked
        };

        Ok(ToggleOutput {
            success: true,
            checked: new_checked,
            message: None,
        })
    }
}

/// Use case for selecting an option.
pub trait SelectUseCase: Send + Sync {
    fn execute(&self, input: SelectInput) -> Result<SelectOutput, SessionError>;
}

/// Implementation of the select use case.
pub struct SelectUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SelectUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SelectUseCase for SelectUseCaseImpl<R> {
    fn execute(&self, input: SelectInput) -> Result<SelectOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) if el.element_type.as_str() != "select" => {
                return Err(SessionError::WrongElementType {
                    element_ref: input.element_ref.clone(),
                    expected: "select".to_string(),
                    actual: el.element_type.as_str().to_string(),
                });
            }
            None => return Err(SessionError::ElementNotFound(input.element_ref.clone())),
            _ => {}
        }

        let screen_text = session_guard.screen_text();
        navigate_to_option(&mut session_guard, &input.option, &screen_text)?;
        session_guard.pty_write(b"\r")?;

        Ok(SelectOutput {
            success: true,
            selected_option: input.option,
            message: None,
        })
    }
}

/// Use case for multiselect.
pub trait MultiselectUseCase: Send + Sync {
    fn execute(&self, input: MultiselectInput) -> Result<MultiselectOutput, SessionError>;
}

/// Implementation of the multiselect use case.
pub struct MultiselectUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MultiselectUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MultiselectUseCase for MultiselectUseCaseImpl<R> {
    fn execute(&self, input: MultiselectInput) -> Result<MultiselectOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        // Verify element exists
        if session_guard.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        let mut selected = Vec::new();
        for option in &input.options {
            session_guard.pty_write(option.as_bytes())?;
            thread::sleep(Duration::from_millis(50));
            session_guard.pty_write(b" ")?; // Toggle selection
            session_guard.pty_write(&[0x15])?; // Ctrl+U to clear
            selected.push(option.clone());
        }

        session_guard.pty_write(b"\r")?; // Confirm selection

        Ok(MultiselectOutput {
            success: true,
            selected_options: selected,
            message: None,
        })
    }
}

// ============================================================================
// Element Query Use Cases
// ============================================================================

/// Use case for getting element text.
pub trait GetTextUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<GetTextOutput, SessionError>;
}

pub struct GetTextUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> GetTextUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> GetTextUseCase for GetTextUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<GetTextOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) => {
                let text = el
                    .label
                    .clone()
                    .or_else(|| el.value.clone())
                    .unwrap_or_default();
                Ok(GetTextOutput { found: true, text })
            }
            None => Ok(GetTextOutput {
                found: false,
                text: String::new(),
            }),
        }
    }
}

/// Use case for getting element value.
pub trait GetValueUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<GetValueOutput, SessionError>;
}

pub struct GetValueUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> GetValueUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> GetValueUseCase for GetValueUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<GetValueOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) => Ok(GetValueOutput {
                found: true,
                value: el.value.clone().unwrap_or_default(),
            }),
            None => Ok(GetValueOutput {
                found: false,
                value: String::new(),
            }),
        }
    }
}

/// Use case for checking element visibility.
pub trait IsVisibleUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<VisibilityOutput, SessionError>;
}

pub struct IsVisibleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> IsVisibleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> IsVisibleUseCase for IsVisibleUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<VisibilityOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        let visible = session_guard.find_element(&input.element_ref).is_some();
        Ok(VisibilityOutput {
            found: visible,
            visible,
        })
    }
}

/// Use case for checking if element is focused.
pub trait IsFocusedUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<FocusCheckOutput, SessionError>;
}

pub struct IsFocusedUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> IsFocusedUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> IsFocusedUseCase for IsFocusedUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<FocusCheckOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) => Ok(FocusCheckOutput {
                found: true,
                focused: el.focused,
            }),
            None => Ok(FocusCheckOutput {
                found: false,
                focused: false,
            }),
        }
    }
}

/// Use case for checking if element is enabled.
pub trait IsEnabledUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<IsEnabledOutput, SessionError>;
}

pub struct IsEnabledUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> IsEnabledUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> IsEnabledUseCase for IsEnabledUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<IsEnabledOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) => Ok(IsEnabledOutput {
                found: true,
                enabled: !el.disabled.unwrap_or(false),
            }),
            None => Ok(IsEnabledOutput {
                found: false,
                enabled: false,
            }),
        }
    }
}

/// Use case for checking if element is checked.
pub trait IsCheckedUseCase: Send + Sync {
    fn execute(&self, input: ElementStateInput) -> Result<IsCheckedOutput, SessionError>;
}

pub struct IsCheckedUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> IsCheckedUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> IsCheckedUseCase for IsCheckedUseCaseImpl<R> {
    fn execute(&self, input: ElementStateInput) -> Result<IsCheckedOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        match session_guard.find_element(&input.element_ref) {
            Some(el) => {
                let el_type = el.element_type.as_str();
                if el_type != "checkbox" && el_type != "radio" {
                    Ok(IsCheckedOutput {
                        found: true,
                        checked: false,
                        message: Some(format!(
                            "Element {} is a {} not a checkbox/radio.",
                            input.element_ref, el_type
                        )),
                    })
                } else {
                    Ok(IsCheckedOutput {
                        found: true,
                        checked: el.checked.unwrap_or(false),
                        message: None,
                    })
                }
            }
            None => Ok(IsCheckedOutput {
                found: false,
                checked: false,
                message: None,
            }),
        }
    }
}

/// Use case for getting the focused element.
pub trait GetFocusedUseCase: Send + Sync {
    fn execute(&self, session_id: Option<&str>) -> Result<GetFocusedOutput, SessionError>;
}

pub struct GetFocusedUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> GetFocusedUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> GetFocusedUseCase for GetFocusedUseCaseImpl<R> {
    fn execute(&self, session_id: Option<&str>) -> Result<GetFocusedOutput, SessionError> {
        let session = self.repository.resolve(session_id)?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;
        session_guard.detect_elements();

        let focused_el = session_guard
            .cached_elements()
            .iter()
            .find(|e| e.focused)
            .cloned();

        Ok(GetFocusedOutput {
            found: focused_el.is_some(),
            element: focused_el,
        })
    }
}

/// Use case for getting session title.
pub trait GetTitleUseCase: Send + Sync {
    fn execute(&self, session_id: Option<&str>) -> Result<GetTitleOutput, SessionError>;
}

pub struct GetTitleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> GetTitleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> GetTitleUseCase for GetTitleUseCaseImpl<R> {
    fn execute(&self, session_id: Option<&str>) -> Result<GetTitleOutput, SessionError> {
        let session = self.repository.resolve(session_id)?;
        let session_guard = mutex_lock_or_recover(&session);

        Ok(GetTitleOutput {
            session_id: session_guard.id.clone(),
            title: session_guard.command.clone(),
        })
    }
}

/// Use case for scrolling an element into view.
pub trait ScrollIntoViewUseCase: Send + Sync {
    fn execute(&self, input: ScrollIntoViewInput) -> Result<ScrollIntoViewOutput, SessionError>;
}

pub struct ScrollIntoViewUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ScrollIntoViewUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ScrollIntoViewUseCase for ScrollIntoViewUseCaseImpl<R> {
    fn execute(&self, input: ScrollIntoViewInput) -> Result<ScrollIntoViewOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let max_scrolls = 50;

        for scroll_count in 0..max_scrolls {
            {
                let mut session_guard = mutex_lock_or_recover(&session);
                let _ = session_guard.update();
                session_guard.detect_elements();

                if session_guard.find_element(&input.element_ref).is_some() {
                    return Ok(ScrollIntoViewOutput {
                        success: true,
                        scrolls_needed: scroll_count,
                        message: None,
                    });
                }

                session_guard.pty_write(ansi_keys::DOWN)?;
            }
            thread::sleep(Duration::from_millis(50));
        }

        Ok(ScrollIntoViewOutput {
            success: false,
            scrolls_needed: max_scrolls,
            message: Some(format!(
                "Element '{}' not found after {} scroll attempts.",
                input.element_ref, max_scrolls
            )),
        })
    }
}
