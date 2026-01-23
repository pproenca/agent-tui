use std::sync::OnceLock;

use regex::Regex;

use crate::vom::Component;
use crate::vom::Role;

fn legacy_ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^@([a-z]+)(\d+)$").unwrap())
}

/// Types of interactive UI elements detected by the Visual Object Model (VOM).
///
/// Some variants are not yet detectable by VOM but are reserved for:
/// - Legacy ref support (e.g., `@rb1` for Radio, `@sel1` for Select)
/// - Future VOM detection capabilities
/// - External element type mapping
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementType {
    Button,
    Input,
    Checkbox,
    /// Reserved for future VOM radio button detection. Currently mapped via legacy refs.
    Radio,
    /// Reserved for future VOM select/dropdown detection. Currently mapped via legacy refs.
    Select,
    MenuItem,
    ListItem,
    /// Reserved for future VOM spinner/loading indicator detection.
    Spinner,
    /// Reserved for future VOM progress bar detection.
    Progress,
    /// Clickable link (URL or file path).
    Link,
}

impl ElementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ElementType::Button => "button",
            ElementType::Input => "input",
            ElementType::Checkbox => "checkbox",
            ElementType::Radio => "radio",
            ElementType::Select => "select",
            ElementType::MenuItem => "menuitem",
            ElementType::ListItem => "listitem",
            ElementType::Spinner => "spinner",
            ElementType::Progress => "progress",
            ElementType::Link => "link",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub row: u16,
    pub col: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub element_ref: String,
    pub element_type: ElementType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub position: Position,
    pub focused: bool,
    pub selected: bool,
    pub checked: Option<bool>,
    pub disabled: Option<bool>,
    pub hint: Option<String>,
}

pub fn role_to_element_type(role: Role) -> ElementType {
    match role {
        Role::Button => ElementType::Button,
        Role::Tab => ElementType::Button,
        Role::Input => ElementType::Input,
        Role::Checkbox => ElementType::Checkbox,
        Role::MenuItem => ElementType::MenuItem,
        Role::StaticText => ElementType::ListItem,
        Role::Panel => ElementType::ListItem,
        Role::Status => ElementType::Spinner,
        Role::ToolBlock => ElementType::ListItem,
        Role::PromptMarker => ElementType::Input,
        Role::ProgressBar => ElementType::Progress,
        Role::Link => ElementType::Link,
        Role::ErrorMessage => ElementType::ListItem,
        Role::DiffLine => ElementType::ListItem,
        Role::CodeBlock => ElementType::ListItem,
    }
}

pub fn detect_checkbox_state(text: &str) -> Option<bool> {
    let text = text.to_lowercase();

    if text.contains("[x]") || text.contains("(x)") || text.contains("☑") || text.contains("✓")
    {
        Some(true)
    } else if text.contains("[ ]") || text.contains("( )") || text.contains("☐") {
        Some(false)
    } else {
        None
    }
}

pub fn component_to_element(
    comp: &Component,
    index: usize,
    cursor_row: u16,
    cursor_col: u16,
) -> Element {
    let focused = comp.bounds.contains(cursor_col, cursor_row);

    let checked = if comp.role == Role::Checkbox {
        detect_checkbox_state(&comp.text_content)
    } else {
        None
    };

    Element {
        element_ref: format!("@e{}", index + 1),
        element_type: role_to_element_type(comp.role),
        label: Some(comp.text_content.trim().to_string()),
        value: None,
        position: Position {
            row: comp.bounds.y,
            col: comp.bounds.x,
            width: Some(comp.bounds.width),
            height: Some(comp.bounds.height),
        },
        focused,
        selected: false,
        checked,
        disabled: None,
        hint: None,
    }
}

pub fn find_element_by_ref<'a>(elements: &'a [Element], ref_str: &str) -> Option<&'a Element> {
    let normalized = if ref_str.starts_with('@') {
        ref_str.to_string()
    } else {
        format!("@{}", ref_str)
    };

    if let Some(el) = elements.iter().find(|e| e.element_ref == normalized) {
        return Some(el);
    }

    if let Some(caps) = legacy_ref_regex().captures(&normalized) {
        let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let index: usize = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        if index > 0 && prefix != "e" {
            let target_type = match prefix {
                "btn" => Some("button"),
                "inp" => Some("input"),
                "cb" => Some("checkbox"),
                "rb" => Some("radio"),
                "sel" => Some("select"),
                "mi" => Some("menuitem"),
                "li" => Some("listitem"),
                "lnk" => Some("link"),
                _ => None,
            };

            if let Some(type_str) = target_type {
                let matching: Vec<_> = elements
                    .iter()
                    .filter(|e| e.element_type.as_str() == type_str)
                    .collect();

                if index <= matching.len() {
                    return Some(matching[index - 1]);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vom::Rect;
    use uuid::Uuid;

    fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
        Component {
            id: Uuid::new_v4(),
            role,
            bounds: Rect::new(x, y, width, 1),
            text_content: text.to_string(),
            visual_hash: 0,
            selected: false,
        }
    }

    fn make_element(ref_str: &str, element_type: ElementType) -> Element {
        Element {
            element_ref: ref_str.to_string(),
            element_type,
            label: Some("test".to_string()),
            value: None,
            position: Position {
                row: 0,
                col: 0,
                width: Some(10),
                height: Some(1),
            },
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

    #[test]
    fn test_find_element_by_ref_sequential() {
        let elements = vec![
            make_element("@e1", ElementType::Button),
            make_element("@e2", ElementType::Input),
            make_element("@e3", ElementType::Checkbox),
        ];

        assert_eq!(
            find_element_by_ref(&elements, "@e1").map(|e| &e.element_ref),
            Some(&"@e1".to_string())
        );
        assert_eq!(
            find_element_by_ref(&elements, "@e2").map(|e| &e.element_ref),
            Some(&"@e2".to_string())
        );
        assert_eq!(
            find_element_by_ref(&elements, "e3").map(|e| &e.element_ref),
            Some(&"@e3".to_string())
        );
        assert!(find_element_by_ref(&elements, "@e4").is_none());
    }

    #[test]
    fn test_find_element_by_ref_legacy_prefix() {
        let elements = vec![
            make_element("@e1", ElementType::Button),
            make_element("@e2", ElementType::Button),
            make_element("@e3", ElementType::Input),
            make_element("@e4", ElementType::Checkbox),
        ];

        assert_eq!(
            find_element_by_ref(&elements, "@btn1").map(|e| &e.element_ref),
            Some(&"@e1".to_string())
        );

        assert_eq!(
            find_element_by_ref(&elements, "@btn2").map(|e| &e.element_ref),
            Some(&"@e2".to_string())
        );

        assert_eq!(
            find_element_by_ref(&elements, "@inp1").map(|e| &e.element_ref),
            Some(&"@e3".to_string())
        );

        assert_eq!(
            find_element_by_ref(&elements, "@cb1").map(|e| &e.element_ref),
            Some(&"@e4".to_string())
        );

        assert!(find_element_by_ref(&elements, "@btn3").is_none());
    }

    #[test]
    fn test_component_to_element_basic() {
        let comp = make_component(Role::Button, "Click me", 5, 10, 8);
        let element = component_to_element(&comp, 0, 0, 0);

        assert_eq!(element.element_ref, "@e1");
        assert_eq!(element.element_type, ElementType::Button);
        assert_eq!(element.label, Some("Click me".to_string()));
        assert_eq!(element.position.row, 10);
        assert_eq!(element.position.col, 5);
        assert_eq!(element.position.width, Some(8));
        assert!(!element.focused);
    }

    #[test]
    fn test_component_to_element_checkbox_checked() {
        let comp = make_component(Role::Checkbox, "[x] Enabled", 0, 0, 11);
        let element = component_to_element(&comp, 0, 0, 0);

        assert_eq!(element.element_type, ElementType::Checkbox);
        assert_eq!(element.checked, Some(true));
    }

    #[test]
    fn test_component_to_element_checkbox_unchecked() {
        let comp = make_component(Role::Checkbox, "[ ] Disabled", 0, 0, 12);
        let element = component_to_element(&comp, 0, 0, 0);

        assert_eq!(element.element_type, ElementType::Checkbox);
        assert_eq!(element.checked, Some(false));
    }

    #[test]
    fn test_role_to_element_type_mapping() {
        assert_eq!(role_to_element_type(Role::Button), ElementType::Button);
        assert_eq!(role_to_element_type(Role::Tab), ElementType::Button);
        assert_eq!(role_to_element_type(Role::Input), ElementType::Input);
        assert_eq!(role_to_element_type(Role::Checkbox), ElementType::Checkbox);
        assert_eq!(role_to_element_type(Role::MenuItem), ElementType::MenuItem);
        assert_eq!(
            role_to_element_type(Role::StaticText),
            ElementType::ListItem
        );
        assert_eq!(role_to_element_type(Role::Panel), ElementType::ListItem);
    }
}
