use crate::domain::core::{Element, ElementType, Position};

#[derive(Debug, Clone)]
pub struct ElementBuilder {
    element_ref: String,
    element_type: ElementType,
    label: Option<String>,
    value: Option<String>,
    row: u16,
    col: u16,
    width: u16,
    height: u16,
    focused: bool,
    selected: bool,
    checked: Option<bool>,
    disabled: Option<bool>,
    hint: Option<String>,
}

impl ElementBuilder {
    pub fn new(element_ref: impl Into<String>, element_type: ElementType) -> Self {
        Self {
            element_ref: element_ref.into(),
            element_type,
            label: None,
            value: None,
            row: 0,
            col: 0,
            width: 10,
            height: 1,
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

    pub fn button(element_ref: impl Into<String>) -> Self {
        Self::new(element_ref, ElementType::Button)
    }

    pub fn input(element_ref: impl Into<String>) -> Self {
        Self::new(element_ref, ElementType::Input)
    }

    pub fn checkbox(element_ref: impl Into<String>) -> Self {
        Self::new(element_ref, ElementType::Checkbox).with_checked(false)
    }

    pub fn select(element_ref: impl Into<String>) -> Self {
        Self::new(element_ref, ElementType::Select)
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn at_position(mut self, row: u16, col: u16) -> Self {
        self.row = row;
        self.col = col;
        self
    }

    pub fn focused(mut self) -> Self {
        self.focused = true;
        self
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn checked(self) -> Self {
        self.with_checked(true)
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = Some(true);
        self
    }

    pub fn build(self) -> Element {
        Element {
            element_ref: self.element_ref,
            element_type: self.element_type,
            label: self.label,
            value: self.value,
            position: Position {
                row: self.row,
                col: self.col,
                width: Some(self.width),
                height: Some(self.height),
            },
            focused: self.focused,
            selected: self.selected,
            checked: self.checked,
            disabled: self.disabled,
            hint: self.hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_builder() {
        let button = ElementBuilder::button("@e1")
            .with_label("Submit")
            .at_position(5, 10)
            .focused()
            .build();

        assert_eq!(button.element_ref, "@e1");
        assert_eq!(button.element_type, ElementType::Button);
        assert_eq!(button.label, Some("Submit".to_string()));
        assert_eq!(button.position.row, 5);
        assert_eq!(button.position.col, 10);
        assert!(button.focused);
    }

    #[test]
    fn test_checkbox_builder() {
        let checkbox = ElementBuilder::checkbox("@e2")
            .with_label("Accept terms")
            .checked()
            .build();

        assert_eq!(checkbox.element_type, ElementType::Checkbox);
        assert_eq!(checkbox.checked, Some(true));
    }

    #[test]
    fn test_input_builder_with_value() {
        let input = ElementBuilder::input("@e3")
            .with_label("Username")
            .with_value("john_doe")
            .build();

        assert_eq!(input.element_type, ElementType::Input);
        assert_eq!(input.label, Some("Username".to_string()));
        assert_eq!(input.value, Some("john_doe".to_string()));
    }

    #[test]
    fn test_select_builder() {
        let select = ElementBuilder::select("@e4").with_label("Country").build();

        assert_eq!(select.element_type, ElementType::Select);
    }

    #[test]
    fn test_disabled_element() {
        let button = ElementBuilder::button("@e5")
            .with_label("Disabled Button")
            .disabled()
            .build();

        assert_eq!(button.disabled, Some(true));
    }
}
