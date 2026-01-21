//! Element detection for TUI applications
//!
//! This module provides pattern-based and style-based detection of
//! interactive UI elements in terminal output.

mod framework;
mod ink;
mod pattern;
mod region;

use crate::terminal::ScreenBuffer;
use regex::Regex;
use std::collections::HashSet;

// Framework detection (available for external use)
#[allow(unused_imports)]
pub use framework::{detect_framework, Framework};
// Ink-specific detection (available for external use)
#[allow(unused_imports)]
pub use ink::detect_ink_elements;
pub use pattern::detect_by_pattern;
// Region/modal detection (available for external use)
#[allow(unused_imports)]
pub use region::{detect_regions, find_modals, find_region_at, BorderStyle, Region};

/// Element types that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementType {
    Button,
    Input,
    Checkbox,
    Radio,
    Select,
    MenuItem,
    ListItem,
    Link,
    Spinner,
    Progress,
    Text,
    Container,
    Unknown,
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
            ElementType::Link => "link",
            ElementType::Spinner => "spinner",
            ElementType::Progress => "progress",
            ElementType::Text => "text",
            ElementType::Container => "container",
            ElementType::Unknown => "unknown",
        }
    }

    /// Get a short prefix for this element type (useful for generating compact refs)
    pub fn prefix(&self) -> &'static str {
        match self {
            ElementType::Button => "btn",
            ElementType::Input => "inp",
            ElementType::Checkbox => "cb",
            ElementType::Radio => "rb",
            ElementType::Select => "sel",
            ElementType::MenuItem => "mi",
            ElementType::ListItem => "li",
            ElementType::Link => "lnk",
            ElementType::Spinner => "spn",
            ElementType::Progress => "prg",
            ElementType::Text => "txt",
            ElementType::Container => "cnt",
            ElementType::Unknown => "el",
        }
    }
}

/// Position in the terminal
#[derive(Debug, Clone)]
pub struct Position {
    pub row: u16,
    pub col: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

/// A detected element
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
    pub options: Option<Vec<String>>,
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
            options: None,
        }
    }

    /// Returns true if this element is interactive (can be clicked, filled, toggled, etc.)
    /// Used for snapshot filtering with -i/--interactive-only flag.
    pub fn is_interactive(&self) -> bool {
        matches!(
            self.element_type,
            ElementType::Button
                | ElementType::Input
                | ElementType::Checkbox
                | ElementType::Radio
                | ElementType::Select
                | ElementType::MenuItem
                | ElementType::Link
        )
    }

    /// Returns true if this element has meaningful content (non-empty text)
    /// Used for snapshot filtering with -c/--compact flag.
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

/// Element detector
pub struct ElementDetector {
    /// Counter for sequential element refs
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

    /// Detect elements in the screen
    pub fn detect(
        &mut self,
        screen_text: &str,
        screen_buffer: Option<&ScreenBuffer>,
    ) -> Vec<Element> {
        // Reset for each snapshot (agent-browser pattern: refs are per-snapshot)
        self.ref_counter = 0;
        self.used_refs.clear();

        let mut elements = Vec::new();
        let _lines: Vec<&str> = screen_text.lines().collect();

        // Pattern-based detection
        let pattern_matches = detect_by_pattern(screen_text);

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

        // Sort elements by position (top-to-bottom, left-to-right)
        elements.sort_by(|a, b| {
            if a.position.row != b.position.row {
                a.position.row.cmp(&b.position.row)
            } else {
                a.position.col.cmp(&b.position.col)
            }
        });

        elements
    }

    /// Check if a region has styling that indicates focus
    ///
    /// Checks multiple style indicators:
    /// - Inverse (most common focus indicator)
    /// - Bold (often used for highlighting)
    /// - Underline (sometimes used for focus)
    /// - Non-default foreground color (highlighting)
    /// - Non-default background color (highlighting)
    fn is_focused_by_style(&self, buffer: &ScreenBuffer, row: u16, col: u16, width: u16) -> bool {
        use crate::terminal::Color;

        let row_idx = row as usize;
        if row_idx >= buffer.cells.len() {
            return false;
        }

        let row_cells = &buffer.cells[row_idx];
        let start = col as usize;
        let end = (col + width) as usize;

        // Check if any cell in the region has focus-indicating styles
        row_cells
            .iter()
            .take(end.min(row_cells.len()))
            .skip(start)
            .any(|cell| {
                let style = &cell.style;

                // Inverse is the strongest focus indicator
                if style.inverse {
                    return true;
                }

                // Bold combined with non-default color often indicates focus
                if style.bold {
                    // Check if we have a non-default foreground color
                    let has_colored_fg = match &style.fg_color {
                        Some(Color::Indexed(idx)) => *idx != 7 && *idx != 15, // Not white/bright white
                        Some(Color::Rgb(_, _, _)) => true,
                        Some(Color::Default) | None => false,
                    };
                    if has_colored_fg {
                        return true;
                    }
                }

                // Non-default background color (not just black) with content
                let has_highlight_bg = match &style.bg_color {
                    Some(Color::Indexed(idx)) => *idx != 0 && *idx != 16, // Not black
                    Some(Color::Rgb(r, g, b)) => *r > 20 || *g > 20 || *b > 20, // Not near-black
                    Some(Color::Default) | None => false,
                };
                if has_highlight_bg && cell.char != ' ' {
                    return true;
                }

                // Underline on interactive element text
                if style.underline && cell.char != ' ' && cell.char != '_' {
                    return true;
                }

                false
            })
    }

    /// Generate a ref for an element
    ///
    /// Uses simple sequential refs like agent-browser: @e1, @e2, @e3
    /// This makes refs deterministic and easy for AI to reason about.
    /// Refs reset on each snapshot (expected behavior).
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

    /// Find element by ref
    ///
    /// Supports both new sequential refs (@e1, @e2) and legacy type-prefixed refs (@btn1, @inp1)
    pub fn find_by_ref<'a>(&self, elements: &'a [Element], ref_str: &str) -> Option<&'a Element> {
        let normalized = if ref_str.starts_with('@') {
            ref_str.to_string()
        } else {
            format!("@{}", ref_str)
        };

        // Exact match first (handles @e1, @e2, etc.)
        if let Some(el) = elements.iter().find(|e| e.element_ref == normalized) {
            return Some(el);
        }

        // Support legacy type-prefixed format (@btn1, @inp2, etc.) for backwards compatibility
        let legacy_re = Regex::new(r"^@([a-z]+)(\d+)$").unwrap();
        if let Some(caps) = legacy_re.captures(&normalized) {
            let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let index: usize = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);

            if index > 0 && prefix != "e" {
                // Map legacy prefix to element type
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
                    // Find nth element of this type
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

    /// Find focused element
    pub fn find_focused<'a>(&self, elements: &'a [Element]) -> Option<&'a Element> {
        elements.iter().find(|e| e.focused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_type_prefix() {
        assert_eq!(ElementType::Button.prefix(), "btn");
        assert_eq!(ElementType::Input.prefix(), "inp");
        assert_eq!(ElementType::Checkbox.prefix(), "cb");
    }

    #[test]
    fn test_generate_sequential_ref() {
        let mut detector = ElementDetector::new();
        let ref1 = detector.generate_ref(&ElementType::Button, Some("Submit"), None, 5, 10);
        let ref2 = detector.generate_ref(&ElementType::Input, Some("Name"), None, 6, 10);
        let ref3 = detector.generate_ref(&ElementType::Button, Some("Cancel"), None, 7, 10);

        // Sequential refs: @e1, @e2, @e3 (agent-browser pattern)
        assert_eq!(ref1, "@e1");
        assert_eq!(ref2, "@e2");
        assert_eq!(ref3, "@e3");
    }

    #[test]
    fn test_refs_reset_on_detect() {
        let mut detector = ElementDetector::new();

        // First detection
        let elements1 = detector.detect("[Submit] [Cancel]", None);
        assert!(elements1.iter().any(|e| e.element_ref == "@e1"));
        assert!(elements1.iter().any(|e| e.element_ref == "@e2"));

        // Second detection should reset refs
        let elements2 = detector.detect("[OK]", None);
        assert!(elements2.iter().any(|e| e.element_ref == "@e1"));
    }

    #[test]
    fn test_find_by_sequential_ref() {
        let mut detector = ElementDetector::new();
        let elements = detector.detect("[Submit] [Cancel]", None);

        // Find by sequential ref
        assert!(detector.find_by_ref(&elements, "@e1").is_some());
        assert!(detector.find_by_ref(&elements, "@e2").is_some());
        assert!(detector.find_by_ref(&elements, "@e3").is_none());
    }

    #[test]
    fn test_find_by_legacy_ref() {
        let mut detector = ElementDetector::new();
        let elements = detector.detect("[Submit] [Cancel]", None);

        // Legacy refs (@btn1, @btn2) should still work via type lookup
        let btn1 = detector.find_by_ref(&elements, "@btn1");
        assert!(btn1.is_some());
        assert_eq!(btn1.unwrap().element_type.as_str(), "button");

        let btn2 = detector.find_by_ref(&elements, "@btn2");
        assert!(btn2.is_some());
        assert_eq!(btn2.unwrap().element_type.as_str(), "button");
    }
}
