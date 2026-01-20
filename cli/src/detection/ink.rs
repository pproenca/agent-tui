//! Ink framework-specific element detection
//!
//! Detects Ink (React for CLI) specific components:
//! - TextInput
//! - SelectInput
//! - MultiSelect
//! - ConfirmInput
//! - Spinners

use super::pattern::PatternMatch;
use super::ElementType;
use regex::Regex;
use std::sync::OnceLock;

// Cached regex patterns for text input detection
fn question_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*\?\s+(.+\?)\s*(.*)$").unwrap())
}

fn label_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([A-Za-z][A-Za-z\s]*?):\s*(.*)$").unwrap())
}

// Cached regex patterns for select input detection
fn selected_item_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*([❯›>])\s+(.+?)$").unwrap())
}

fn unselected_item_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s{3,}(.+?)$").unwrap())
}

// Cached regex patterns for multi-select detection
fn multiselect_checked_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*([◉●✓✔])\s+(.+?)$").unwrap())
}

fn multiselect_unchecked_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*([◯○])\s+(.+?)$").unwrap())
}

// Cached regex patterns for confirm input detection
fn confirm_yn_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\(([YN])/([yn])\)").unwrap())
}

fn confirm_yes_no_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(Yes|No)\s*/\s*(Yes|No)\b").unwrap())
}

// Cached regex patterns for spinner detection
fn braille_spinner_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⣾⣽⣻⢿⡿⣟⣯⣷])\s*(.*)").unwrap())
}

fn dots_spinner_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\.{1,3})\s+(.+)").unwrap())
}

/// Detect Ink-specific elements
pub fn detect_ink_elements(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();

    // Detect TextInput components
    matches.extend(detect_text_input(screen));

    // Detect SelectInput components
    matches.extend(detect_select_input(screen));

    // Detect MultiSelect components
    matches.extend(detect_multi_select(screen));

    // Detect ConfirmInput components
    matches.extend(detect_confirm_input(screen));

    // Detect Spinners
    matches.extend(detect_spinners(screen));

    matches
}

/// Detect Ink TextInput components
/// Pattern: "Question? answer" or "Label: value"
fn detect_text_input(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen.lines().collect();

    let question_re = question_regex();
    let label_re = label_regex();

    for (row_idx, line) in lines.iter().enumerate() {
        // Check for question-style input
        if let Some(caps) = question_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let question = caps.get(1).map(|m| m.as_str().trim().to_string());
            let value = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .filter(|v| !v.is_empty());

            matches.push(PatternMatch {
                element_type: ElementType::Input,
                text: full_match.as_str().to_string(),
                label: question,
                value,
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: None,
            });
            continue;
        }

        // Check for label-style input
        if let Some(caps) = label_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let label = caps.get(1).map(|m| m.as_str().trim().to_string());
            let value = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .filter(|v| !v.is_empty());

            // Skip if it looks like a menu item or output line
            if label
                .as_ref()
                .map(|l| l.len() > 30 || l.contains("  "))
                .unwrap_or(false)
            {
                continue;
            }

            matches.push(PatternMatch {
                element_type: ElementType::Input,
                text: full_match.as_str().to_string(),
                label,
                value,
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: None,
            });
        }
    }

    matches
}

/// Detect Ink SelectInput components
/// Pattern: "❯ Option" or "› Option" for selected, "  Option" for unselected
fn detect_select_input(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen.lines().collect();

    let selected_re = selected_item_regex();
    let unselected_re = unselected_item_regex();

    let mut in_select_group = false;

    for (row_idx, line) in lines.iter().enumerate() {
        // Check for selected item
        if let Some(caps) = selected_re.captures(line) {
            in_select_group = true;
            let full_match = caps.get(0).unwrap();
            let label = caps.get(2).map(|m| m.as_str().trim().to_string());

            matches.push(PatternMatch {
                element_type: ElementType::MenuItem,
                text: full_match.as_str().to_string(),
                label,
                value: Some("selected".to_string()),
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: Some(true),
            });
            continue;
        }

        // Check for unselected items (only if we're in a select group)
        if in_select_group {
            if let Some(caps) = unselected_re.captures(line) {
                let full_match = caps.get(0).unwrap();
                let label = caps.get(1).map(|m| m.as_str().trim().to_string());

                // Skip empty lines or lines that don't look like options
                if label.as_ref().map(|l| l.is_empty()).unwrap_or(true) {
                    in_select_group = false;
                    continue;
                }

                matches.push(PatternMatch {
                    element_type: ElementType::MenuItem,
                    text: full_match.as_str().to_string(),
                    label,
                    value: Some("unselected".to_string()),
                    row: row_idx as u16,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(false),
                });
            } else if !line.trim().is_empty() {
                // Non-matching non-empty line ends the select group
                in_select_group = false;
            }
        }
    }

    matches
}

/// Detect Ink MultiSelect components
/// Pattern: "◉ Selected Option" or "◯ Unselected Option"
fn detect_multi_select(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen.lines().collect();

    let selected_re = multiselect_checked_regex();
    let unselected_re = multiselect_unchecked_regex();

    for (row_idx, line) in lines.iter().enumerate() {
        // Check for selected checkbox
        if let Some(caps) = selected_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let label = caps.get(2).map(|m| m.as_str().trim().to_string());

            matches.push(PatternMatch {
                element_type: ElementType::Checkbox,
                text: full_match.as_str().to_string(),
                label,
                value: Some("checked".to_string()),
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: Some(true),
            });
            continue;
        }

        // Check for unselected checkbox
        if let Some(caps) = unselected_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let label = caps.get(2).map(|m| m.as_str().trim().to_string());

            matches.push(PatternMatch {
                element_type: ElementType::Checkbox,
                text: full_match.as_str().to_string(),
                label,
                value: Some("unchecked".to_string()),
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: Some(false),
            });
        }
    }

    matches
}

/// Detect Ink ConfirmInput components
/// Pattern: "Yes" / "No" or "Y" / "n" toggle
fn detect_confirm_input(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen.lines().collect();

    let confirm_re = confirm_yn_regex();
    let yes_no_re = confirm_yes_no_regex();

    for (row_idx, line) in lines.iter().enumerate() {
        // Check for (Y/n) or (y/N) style
        if let Some(caps) = confirm_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let first = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let is_yes_default = first
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            matches.push(PatternMatch {
                element_type: ElementType::Radio,
                text: full_match.as_str().to_string(),
                label: Some("Confirm".to_string()),
                value: Some(if is_yes_default { "yes" } else { "no" }.to_string()),
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: Some(is_yes_default),
            });
            continue;
        }

        // Check for Yes/No style
        if let Some(caps) = yes_no_re.captures(line) {
            let full_match = caps.get(0).unwrap();

            matches.push(PatternMatch {
                element_type: ElementType::Radio,
                text: full_match.as_str().to_string(),
                label: Some("Confirm".to_string()),
                value: None,
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: None,
            });
        }
    }

    matches
}

/// Detect Ink Spinner components
/// Pattern: Braille spinner characters
fn detect_spinners(screen: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen.lines().collect();

    let braille_re = braille_spinner_regex();
    let dots_re = dots_spinner_regex();

    for (row_idx, line) in lines.iter().enumerate() {
        // Check for braille spinner
        if let Some(caps) = braille_re.captures(line) {
            let full_match = caps.get(0).unwrap();
            let label = caps.get(2).map(|m| m.as_str().trim().to_string());

            matches.push(PatternMatch {
                element_type: ElementType::Spinner,
                text: full_match.as_str().to_string(),
                label,
                value: Some("loading".to_string()),
                row: row_idx as u16,
                col: full_match.start() as u16,
                width: full_match.as_str().len() as u16,
                checked: None,
            });
            continue;
        }

        // Check for dots spinner at start of line
        if line.trim().starts_with('.') {
            if let Some(caps) = dots_re.captures(line) {
                let full_match = caps.get(0).unwrap();
                let label = caps.get(2).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Spinner,
                    text: full_match.as_str().to_string(),
                    label,
                    value: Some("loading".to_string()),
                    row: row_idx as u16,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_select_input() {
        let screen = "Select a color:\n  ❯ Red\n    Blue\n    Green";
        let matches = detect_select_input(screen);

        assert!(!matches.is_empty());
        let selected: Vec<_> = matches.iter().filter(|m| m.checked == Some(true)).collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].label, Some("Red".to_string()));
    }

    #[test]
    fn test_detect_multi_select() {
        let screen = "Select items:\n  ◉ Item 1\n  ◯ Item 2\n  ◉ Item 3";
        let matches = detect_multi_select(screen);

        assert_eq!(matches.len(), 3);
        let checked: Vec<_> = matches.iter().filter(|m| m.checked == Some(true)).collect();
        assert_eq!(checked.len(), 2);
    }

    #[test]
    fn test_detect_confirm() {
        let screen = "Continue? (Y/n)";
        let matches = detect_confirm_input(screen);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].checked, Some(true)); // Y is uppercase = default yes
    }

    #[test]
    fn test_detect_spinner() {
        let screen = "⠋ Loading data...";
        let matches = detect_spinners(screen);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Spinner);
    }

    #[test]
    fn test_detect_text_input() {
        let screen = "? What is your name? John";
        let matches = detect_text_input(screen);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Input);
        assert_eq!(matches[0].value, Some("John".to_string()));
    }
}
