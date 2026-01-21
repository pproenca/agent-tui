mod framework;
mod frameworks;
pub mod pattern;
mod registry;
mod traits;

use crate::terminal::ScreenBuffer;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

fn legacy_ref_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^@([a-z]+)(\d+)$").unwrap())
}

pub use framework::{detect_framework, Framework};
pub use registry::FrameworkDetector;
pub use traits::{DetectionContext, ElementDetectorImpl};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementType {
    Button,
    Input,
    Checkbox,
    Radio,
    Select,
    MenuItem,
    ListItem,
    Spinner,
    Progress,
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

impl Element {
    pub fn new(
        element_ref: String,
        element_type: ElementType,
        row: u16,
        col: u16,
        width: u16,
    ) -> Self {
        Self {
            element_ref,
            element_type,
            label: None,
            value: None,
            position: Position {
                row,
                col,
                width: Some(width),
                height: Some(1),
            },
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

    pub fn is_interactive(&self) -> bool {
        matches!(
            self.element_type,
            ElementType::Button
                | ElementType::Input
                | ElementType::Checkbox
                | ElementType::Radio
                | ElementType::Select
                | ElementType::MenuItem
        )
    }

    pub fn has_content(&self) -> bool {
        self.label
            .as_ref()
            .map(|l| !l.trim().is_empty())
            .unwrap_or(false)
            || self
                .value
                .as_ref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
    }
}

pub struct ElementDetector {
    ref_counter: usize,
    used_refs: HashSet<String>,
}

impl Default for ElementDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementDetector {
    pub fn new() -> Self {
        Self {
            ref_counter: 0,
            used_refs: HashSet::new(),
        }
    }

    pub fn detect(
        &mut self,
        screen_text: &str,
        screen_buffer: Option<&ScreenBuffer>,
    ) -> Vec<Element> {
        self.detect_with_framework(screen_text, screen_buffer, None)
    }

    pub fn detect_with_framework(
        &mut self,
        screen_text: &str,
        screen_buffer: Option<&ScreenBuffer>,
        framework_detector: Option<&FrameworkDetector>,
    ) -> Vec<Element> {
        use traits::ElementDetectorImpl;

        self.ref_counter = 0;
        self.used_refs.clear();

        let mut elements = Vec::new();

        let ctx = DetectionContext::new(screen_text, screen_buffer);

        let default_detector;
        let detector = match framework_detector {
            Some(d) => d,
            None => {
                default_detector = FrameworkDetector::detect(&ctx);
                &default_detector
            }
        };

        let pattern_matches = detector.detect_patterns(&ctx);

        for m in pattern_matches {
            let focused = if let Some(buffer) = screen_buffer {
                self.is_focused_by_style(buffer, m.row, m.col, m.width)
            } else {
                false
            };

            let element_ref = self.generate_ref(
                &m.element_type,
                m.label.as_deref(),
                m.value.as_deref(),
                m.row,
                m.col,
            );

            let mut element = Element::new(element_ref, m.element_type, m.row, m.col, m.width);
            element.label = m.label;
            element.value = m.value;
            element.focused = focused;
            element.selected = m.checked.unwrap_or(false);
            element.checked = m.checked;

            elements.push(element);
        }

        elements.sort_by(|a, b| {
            if a.position.row != b.position.row {
                a.position.row.cmp(&b.position.row)
            } else {
                a.position.col.cmp(&b.position.col)
            }
        });

        elements
    }

    fn is_focused_by_style(&self, buffer: &ScreenBuffer, row: u16, col: u16, width: u16) -> bool {
        use crate::terminal::Color;

        let row_idx = row as usize;
        if row_idx >= buffer.cells.len() {
            return false;
        }

        let row_cells = &buffer.cells[row_idx];
        let start = col as usize;
        let end = (col + width) as usize;

        row_cells
            .iter()
            .take(end.min(row_cells.len()))
            .skip(start)
            .any(|cell| {
                let style = &cell.style;

                if style.inverse {
                    return true;
                }

                if style.bold {
                    let has_colored_fg = match &style.fg_color {
                        Some(Color::Indexed(idx)) => *idx != 7 && *idx != 15,
                        Some(Color::Rgb(_, _, _)) => true,
                        Some(Color::Default) | None => false,
                    };
                    if has_colored_fg {
                        return true;
                    }
                }

                let has_highlight_bg = match &style.bg_color {
                    Some(Color::Indexed(idx)) => *idx != 0 && *idx != 16,
                    Some(Color::Rgb(r, g, b)) => *r > 20 || *g > 20 || *b > 20,
                    Some(Color::Default) | None => false,
                };
                if has_highlight_bg && cell.char != ' ' {
                    return true;
                }

                if style.underline && cell.char != ' ' && cell.char != '_' {
                    return true;
                }

                false
            })
    }

    fn generate_ref(
        &mut self,
        _element_type: &ElementType,
        _label: Option<&str>,
        _value: Option<&str>,
        _row: u16,
        _col: u16,
    ) -> String {
        self.ref_counter += 1;
        let seq_ref = format!("@e{}", self.ref_counter);
        self.used_refs.insert(seq_ref.clone());
        seq_ref
    }

    pub fn find_by_ref<'a>(&self, elements: &'a [Element], ref_str: &str) -> Option<&'a Element> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sequential_ref() {
        let mut detector = ElementDetector::new();
        let ref1 = detector.generate_ref(&ElementType::Button, Some("Submit"), None, 5, 10);
        let ref2 = detector.generate_ref(&ElementType::Input, Some("Name"), None, 6, 10);
        let ref3 = detector.generate_ref(&ElementType::Button, Some("Cancel"), None, 7, 10);

        assert_eq!(ref1, "@e1");
        assert_eq!(ref2, "@e2");
        assert_eq!(ref3, "@e3");
    }

    #[test]
    fn test_refs_reset_on_detect() {
        let mut detector = ElementDetector::new();

        let elements1 = detector.detect("[Submit] [Cancel]", None);
        assert!(elements1.iter().any(|e| e.element_ref == "@e1"));
        assert!(elements1.iter().any(|e| e.element_ref == "@e2"));

        let elements2 = detector.detect("[OK]", None);
        assert!(elements2.iter().any(|e| e.element_ref == "@e1"));
    }

    #[test]
    fn test_find_by_sequential_ref() {
        let mut detector = ElementDetector::new();
        let elements = detector.detect("[Submit] [Cancel]", None);

        assert!(detector.find_by_ref(&elements, "@e1").is_some());
        assert!(detector.find_by_ref(&elements, "@e2").is_some());
        assert!(detector.find_by_ref(&elements, "@e3").is_none());
    }

    #[test]
    fn test_find_by_legacy_ref() {
        let mut detector = ElementDetector::new();
        let elements = detector.detect("[Submit] [Cancel]", None);

        let btn1 = detector.find_by_ref(&elements, "@btn1");
        assert!(btn1.is_some());
        assert_eq!(btn1.unwrap().element_type.as_str(), "button");

        let btn2 = detector.find_by_ref(&elements, "@btn2");
        assert!(btn2.is_some());
        assert_eq!(btn2.unwrap().element_type.as_str(), "button");
    }
}
