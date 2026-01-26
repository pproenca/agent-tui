use std::sync::Arc;
use std::time::Duration;

use crate::common::ansi_keys;
use crate::domain::core::Element;

use crate::adapters::{core_element_to_domain, core_elements_to_domain};
use crate::domain::{
    ClearInput, ClearOutput, ClickInput, ClickOutput, CountInput, CountOutput, DoubleClickInput,
    DoubleClickOutput, ElementStateInput, FillInput, FillOutput, FindInput, FindOutput,
    FocusCheckOutput, FocusInput, FocusOutput, GetFocusedOutput, GetTextOutput, GetTitleOutput,
    GetValueOutput, IsCheckedOutput, IsEnabledOutput, MultiselectInput, MultiselectOutput,
    ScrollInput, ScrollIntoViewInput, ScrollIntoViewOutput, ScrollOutput, SelectAllInput,
    SelectAllOutput, SelectInput, SelectOutput, SessionInput, ToggleInput, ToggleOutput,
    VisibilityOutput,
};
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;
use crate::usecases::select_helpers::navigate_to_option;

#[derive(Debug, Clone, Default)]
pub struct ElementFilterCriteria {
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub focused: Option<bool>,
    pub exact: bool,
}

pub fn filter_elements<'a>(
    elements: impl IntoIterator<Item = &'a Element>,
    criteria: &ElementFilterCriteria,
) -> Vec<&'a Element> {
    elements
        .into_iter()
        .filter(|el| {
            if let Some(ref role) = criteria.role {
                let el_type = format!("{:?}", el.element_type).to_lowercase();
                if !el_type.contains(&role.to_lowercase()) {
                    return false;
                }
            }

            if let Some(ref name) = criteria.name {
                let el_label = el.label.as_deref().unwrap_or("");
                if criteria.exact {
                    if el_label != name {
                        return false;
                    }
                } else if !el_label.to_lowercase().contains(&name.to_lowercase()) {
                    return false;
                }
            }

            if let Some(ref text) = criteria.text {
                let el_text = el.label.as_deref().unwrap_or("").to_lowercase();
                if !el_text.contains(&text.to_lowercase()) {
                    return false;
                }
            }

            if let Some(focused) = criteria.focused {
                if el.focused != focused {
                    return false;
                }
            }

            true
        })
        .collect()
}

pub trait ClickUseCase: Send + Sync {
    fn execute(&self, input: ClickInput) -> Result<ClickOutput, SessionError>;
}

pub struct ClickUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ClickUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ClickUseCase for ClickUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ClickInput) -> Result<ClickOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.click(&input.element_ref)?;

        Ok(ClickOutput {
            success: true,
            message: None,
            warning: None,
        })
    }
}

pub trait FillUseCase: Send + Sync {
    fn execute(&self, input: FillInput) -> Result<FillOutput, SessionError>;
}

pub struct FillUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FillUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FillUseCase for FillUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            element_ref = %input.element_ref,
            value_len = input.value.len()
        )
    )]
    fn execute(&self, input: FillInput) -> Result<FillOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;

        session.click(&input.element_ref)?;

        session.keystroke("ctrl+a")?;
        session.type_text(&input.value)?;

        Ok(FillOutput {
            success: true,
            message: None,
        })
    }
}

pub trait FindUseCase: Send + Sync {
    fn execute(&self, input: FindInput) -> Result<FindOutput, SessionError>;
}

pub struct FindUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FindUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FindUseCase for FindUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            role = ?input.role,
            focused = ?input.focused,
            exact = input.exact,
            name_len = input.name.as_ref().map(|name| name.len()),
            text_len = input.text.as_ref().map(|text| text.len()),
            nth = ?input.nth
        )
    )]
    fn execute(&self, input: FindInput) -> Result<FindOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        let all_elements = session.detect_elements();

        let criteria = ElementFilterCriteria {
            role: input.role,
            name: input.name,
            text: input.text,
            focused: input.focused,
            exact: input.exact,
        };

        let filtered = filter_elements(all_elements.iter(), &criteria);

        let elements: Vec<_> = if let Some(nth) = input.nth {
            filtered.get(nth).into_iter().cloned().cloned().collect()
        } else {
            filtered.into_iter().cloned().collect()
        };

        let count = elements.len();
        let domain_elements = core_elements_to_domain(&elements);

        Ok(FindOutput {
            elements: domain_elements,
            count,
        })
    }
}

pub trait ScrollUseCase: Send + Sync {
    fn execute(&self, input: ScrollInput) -> Result<ScrollOutput, SessionError>;
}

pub struct ScrollUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ScrollUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ScrollUseCase for ScrollUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            direction = %input.direction,
            amount = input.amount
        )
    )]
    fn execute(&self, input: ScrollInput) -> Result<ScrollOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

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
            session.pty_write(key_seq)?;
        }

        Ok(ScrollOutput { success: true })
    }
}

pub trait CountUseCase: Send + Sync {
    fn execute(&self, input: CountInput) -> Result<CountOutput, SessionError>;
}

pub struct CountUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> CountUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> CountUseCase for CountUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            role = ?input.role,
            name_len = input.name.as_ref().map(|name| name.len()),
            text_len = input.text.as_ref().map(|text| text.len())
        )
    )]
    fn execute(&self, input: CountInput) -> Result<CountOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        let all_elements = session.detect_elements();

        let criteria = ElementFilterCriteria {
            role: input.role,
            name: input.name,
            text: input.text,
            ..Default::default()
        };

        let count = filter_elements(all_elements.iter(), &criteria).len();

        Ok(CountOutput { count })
    }
}

pub trait DoubleClickUseCase: Send + Sync {
    fn execute(&self, input: DoubleClickInput) -> Result<DoubleClickOutput, SessionError>;
}

pub struct DoubleClickUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> DoubleClickUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> DoubleClickUseCase for DoubleClickUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: DoubleClickInput) -> Result<DoubleClickOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        {
            session.update()?;
            session.click(&input.element_ref)?;
        }

        let subscription = session.stream_subscribe();
        let _ = subscription.wait(Some(Duration::from_millis(50)));

        {
            session.click(&input.element_ref)?;
        }

        Ok(DoubleClickOutput { success: true })
    }
}

pub trait FocusUseCase: Send + Sync {
    fn execute(&self, input: FocusInput) -> Result<FocusOutput, SessionError>;
}

pub struct FocusUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> FocusUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> FocusUseCase for FocusUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: FocusInput) -> Result<FocusOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        if session.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        session.pty_write(b"\t")?;

        Ok(FocusOutput { success: true })
    }
}

pub trait ClearUseCase: Send + Sync {
    fn execute(&self, input: ClearInput) -> Result<ClearOutput, SessionError>;
}

pub struct ClearUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ClearUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ClearUseCase for ClearUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ClearInput) -> Result<ClearOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        if session.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        session.pty_write(b"\x15")?;

        Ok(ClearOutput { success: true })
    }
}

pub trait SelectAllUseCase: Send + Sync {
    fn execute(&self, input: SelectAllInput) -> Result<SelectAllOutput, SessionError>;
}

pub struct SelectAllUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SelectAllUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SelectAllUseCase for SelectAllUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: SelectAllInput) -> Result<SelectAllOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        if session.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        session.pty_write(b"\x01")?;

        Ok(SelectAllOutput { success: true })
    }
}

pub trait ToggleUseCase: Send + Sync {
    fn execute(&self, input: ToggleInput) -> Result<ToggleOutput, SessionError>;
}

pub struct ToggleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ToggleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ToggleUseCase for ToggleUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            element_ref = %input.element_ref,
            state = ?input.state
        )
    )]
    fn execute(&self, input: ToggleInput) -> Result<ToggleOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        let current_checked = match session.find_element(&input.element_ref) {
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
            session.pty_write(b" ")?;
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

pub trait SelectUseCase: Send + Sync {
    fn execute(&self, input: SelectInput) -> Result<SelectOutput, SessionError>;
}

pub struct SelectUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SelectUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SelectUseCase for SelectUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            element_ref = %input.element_ref,
            option_len = input.option.len()
        )
    )]
    fn execute(&self, input: SelectInput) -> Result<SelectOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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

        let screen_text = session.screen_text();
        navigate_to_option(session.as_ref(), &input.option, &screen_text)?;
        session.pty_write(b"\r")?;

        Ok(SelectOutput {
            success: true,
            selected_option: input.option,
            message: None,
        })
    }
}

pub trait MultiselectUseCase: Send + Sync {
    fn execute(&self, input: MultiselectInput) -> Result<MultiselectOutput, SessionError>;
}

pub struct MultiselectUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MultiselectUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MultiselectUseCase for MultiselectUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            element_ref = %input.element_ref,
            options_len = input.options.len()
        )
    )]
    fn execute(&self, input: MultiselectInput) -> Result<MultiselectOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        if session.find_element(&input.element_ref).is_none() {
            return Err(SessionError::ElementNotFound(input.element_ref));
        }

        let mut selected = Vec::new();
        let subscription = session.stream_subscribe();
        for option in &input.options {
            session.pty_write(option.as_bytes())?;
            let _ = subscription.wait(Some(Duration::from_millis(50)));
            session.pty_write(b" ")?;
            session.pty_write(&[0x15])?;
            selected.push(option.clone());
        }

        session.pty_write(b"\r")?;

        Ok(MultiselectOutput {
            success: true,
            selected_options: selected,
            message: None,
        })
    }
}

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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<GetTextOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<GetValueOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<VisibilityOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        let visible = session.find_element(&input.element_ref).is_some();
        Ok(VisibilityOutput {
            found: visible,
            visible,
        })
    }
}

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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<FocusCheckOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<IsEnabledOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ElementStateInput) -> Result<IsCheckedOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        match session.find_element(&input.element_ref) {
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

pub trait GetFocusedUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<GetFocusedOutput, SessionError>;
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
    #[tracing::instrument(skip(self, input), fields(session = ?input.session_id))]
    fn execute(&self, input: SessionInput) -> Result<GetFocusedOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;
        session.detect_elements();

        let focused_el = session
            .detect_elements()
            .iter()
            .find(|e| e.focused)
            .cloned()
            .map(|el| core_element_to_domain(&el));

        Ok(GetFocusedOutput {
            found: focused_el.is_some(),
            element: focused_el,
        })
    }
}

pub trait GetTitleUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<GetTitleOutput, SessionError>;
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
    #[tracing::instrument(skip(self, input), fields(session = ?input.session_id))]
    fn execute(&self, input: SessionInput) -> Result<GetTitleOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        Ok(GetTitleOutput {
            session_id: session.session_id(),
            title: session.command(),
        })
    }
}

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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, element_ref = %input.element_ref)
    )]
    fn execute(&self, input: ScrollIntoViewInput) -> Result<ScrollIntoViewOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let max_scrolls = 50;

        let subscription = session.stream_subscribe();
        for scroll_count in 0..max_scrolls {
            {
                let _ = session.update();
                session.detect_elements();

                if session.find_element(&input.element_ref).is_some() {
                    return Ok(ScrollIntoViewOutput {
                        success: true,
                        scrolls_needed: scroll_count,
                        message: None,
                    });
                }

                session.pty_write(ansi_keys::DOWN)?;
            }
            let _ = subscription.wait(Some(Duration::from_millis(50)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::infra::daemon::test_support::{MockError, MockSessionRepository};

    #[test]
    fn test_click_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ClickUseCaseImpl::new(repo);

        let input = ClickInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_click_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ClickUseCaseImpl::new(repo);

        let input = ClickInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_fill_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = FillUseCaseImpl::new(repo);

        let input = FillInput {
            session_id: None,
            element_ref: "@inp1".to_string(),
            value: "test value".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_fill_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("nonexistent".to_string()))
                .build(),
        );
        let usecase = FillUseCaseImpl::new(repo);

        let input = FillInput {
            session_id: Some(SessionId::new("nonexistent")),
            element_ref: "@inp1".to_string(),
            value: "test".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_find_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = FindUseCaseImpl::new(repo);

        let input = FindInput::default();

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_find_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("unknown".to_string()))
                .build(),
        );
        let usecase = FindUseCaseImpl::new(repo);

        let input = FindInput {
            session_id: Some(SessionId::new("unknown")),
            ..Default::default()
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_toggle_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ToggleUseCaseImpl::new(repo);

        let input = ToggleInput {
            session_id: None,
            element_ref: "@cb1".to_string(),
            state: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_toggle_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ToggleUseCaseImpl::new(repo);

        let input = ToggleInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@cb1".to_string(),
            state: Some(true),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_select_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = SelectUseCaseImpl::new(repo);

        let input = SelectInput {
            session_id: None,
            element_ref: "@sel1".to_string(),
            option: "Option A".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_select_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = SelectUseCaseImpl::new(repo);

        let input = SelectInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@sel1".to_string(),
            option: "Option A".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_multiselect_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = MultiselectUseCaseImpl::new(repo);

        let input = MultiselectInput {
            session_id: None,
            element_ref: "@msel1".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_multiselect_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = MultiselectUseCaseImpl::new(repo);

        let input = MultiselectInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@msel1".to_string(),
            options: vec!["A".to_string()],
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_scroll_into_view_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ScrollIntoViewUseCaseImpl::new(repo);

        let input = ScrollIntoViewInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_scroll_into_view_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ScrollIntoViewUseCaseImpl::new(repo);

        let input = ScrollIntoViewInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_scroll_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ScrollUseCaseImpl::new(repo);

        let input = ScrollInput {
            session_id: None,
            direction: "down".to_string(),
            amount: 5,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_scroll_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ScrollUseCaseImpl::new(repo);

        let input = ScrollInput {
            session_id: Some(SessionId::new("missing")),
            direction: "up".to_string(),
            amount: 3,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_count_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = CountUseCaseImpl::new(repo);

        let input = CountInput {
            session_id: None,
            role: Some("button".to_string()),
            name: None,
            text: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_count_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("unknown".to_string()))
                .build(),
        );
        let usecase = CountUseCaseImpl::new(repo);

        let input = CountInput {
            session_id: Some(SessionId::new("unknown")),
            role: None,
            name: None,
            text: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_double_click_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = DoubleClickUseCaseImpl::new(repo);

        let input = DoubleClickInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_double_click_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = DoubleClickUseCaseImpl::new(repo);

        let input = DoubleClickInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_focus_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = FocusUseCaseImpl::new(repo);

        let input = FocusInput {
            session_id: None,
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_focus_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = FocusUseCaseImpl::new(repo);

        let input = FocusInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_clear_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ClearUseCaseImpl::new(repo);

        let input = ClearInput {
            session_id: None,
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_clear_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = ClearUseCaseImpl::new(repo);

        let input = ClearInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_select_all_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = SelectAllUseCaseImpl::new(repo);

        let input = SelectAllInput {
            session_id: None,
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_select_all_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = SelectAllUseCaseImpl::new(repo);

        let input = SelectAllInput {
            session_id: Some(SessionId::new("missing")),
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_get_text_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = GetTextUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_get_value_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = GetValueUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@inp1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_is_visible_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = IsVisibleUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_is_focused_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = IsFocusedUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_is_enabled_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = IsEnabledUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@e1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_is_checked_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = IsCheckedUseCaseImpl::new(repo);

        let input = ElementStateInput {
            session_id: None,
            element_ref: "@cb1".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_get_focused_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = GetFocusedUseCaseImpl::new(repo);

        let input = SessionInput { session_id: None };
        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_get_title_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = GetTitleUseCaseImpl::new(repo);

        let input = SessionInput { session_id: None };
        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }
}
