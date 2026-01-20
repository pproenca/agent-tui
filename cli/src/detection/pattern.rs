//! Pattern-based element detection
//!
//! Detects TUI elements using regex patterns for common UI patterns.

use super::ElementType;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Cached regex patterns for element detection
struct PatternRegexes {
    button: [Regex; 2],
    input: [Regex; 2],
    checkbox: [Regex; 2],
    radio: [Regex; 1],
    select: [Regex; 2],
    menu_item: [Regex; 2],
    list_item: [Regex; 2],
    spinner: [Regex; 2],
    progress: [Regex; 2],
}

fn get_patterns() -> &'static PatternRegexes {
    static PATTERNS: OnceLock<PatternRegexes> = OnceLock::new();
    PATTERNS.get_or_init(|| PatternRegexes {
        button: [
            // [Button Text] - square brackets
            Regex::new(r"\[\s*([^\[\]_]+?)\s*\]").unwrap(),
            // <Button Text> - angle brackets
            Regex::new(r"<\s*([^<>]+?)\s*>").unwrap(),
        ],
        input: [
            // Label: [value___] - input with label and underscores
            Regex::new(r"([A-Za-z\s]+):\s*\[([^\]]*?)_*\]").unwrap(),
            // [value___] - input with underscores
            Regex::new(r"\[([^\]]*?)_{3,}\]").unwrap(),
        ],
        checkbox: [
            // [x] Label or [ ] Label
            Regex::new(r"\[([xX✓✔\s])\]\s*(.+?)(?:\s{2,}|$)").unwrap(),
            // ◉ Label or ◯ Label (filled/empty circle)
            Regex::new(r"([◉◯●○✓✔])\s+(.+?)(?:\s{2,}|$)").unwrap(),
        ],
        radio: [
            // ( ) Label or (•) Label
            Regex::new(r"\(([•o*\s])\)\s*(.+?)(?:\s{2,}|$)").unwrap(),
        ],
        select: [
            // Label: value ▼
            Regex::new(r"([A-Za-z\s]+):\s*\[?([^\]▼▾\n]+?)\s*[▼▾]\]?").unwrap(),
            // value ▼
            Regex::new(r"([^\s]+)\s+[▼▾]").unwrap(),
        ],
        menu_item: [
            // > Item or ❯ Item or › Item or ▸ Item
            Regex::new(r"^\s*([>❯›▸►●•])\s+(.+?)$").unwrap(),
            // 1. Item (numbered menu)
            Regex::new(r"^\s*(\d+)\.\s+(.+?)$").unwrap(),
        ],
        list_item: [
            // - Item or * Item
            Regex::new(r"^\s*[-*]\s+(.+?)$").unwrap(),
            // • Item
            Regex::new(r"^\s*•\s+(.+?)$").unwrap(),
        ],
        spinner: [
            // Braille spinners
            Regex::new(r"[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]").unwrap(),
            // ASCII spinners
            Regex::new(r"[\|/\-\\]\s+(.+)").unwrap(),
        ],
        progress: [
            // [████░░░░░░] or [====>    ]
            Regex::new(r"\[([█▓▒░=\->#\s]+)\]").unwrap(),
            // 50% or 50 %
            Regex::new(r"(\d+)\s*%").unwrap(),
        ],
    })
}

/// A pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub element_type: ElementType,
    pub text: String,
    pub label: Option<String>,
    pub value: Option<String>,
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub checked: Option<bool>,
}

/// Detect elements using pattern matching
pub fn detect_by_pattern(screen_text: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen_text.lines().collect();

    let patterns = get_patterns();

    for (row_idx, line) in lines.iter().enumerate() {
        let row = row_idx as u16;

        // Detect buttons (but not inputs or checkboxes)
        for pattern in &patterns.button {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());

                // Skip if it looks like an input (has underscores) or checkbox
                let text = full_match.as_str();
                if text.contains('_')
                    || text.starts_with("[x]")
                    || text.starts_with("[ ]")
                    || text.starts_with("[X]")
                    || text.starts_with("[✓]")
                    || text.starts_with("[✔]")
                {
                    continue;
                }

                // Skip very short "buttons" that are likely just formatting
                if let Some(ref l) = label {
                    if l.len() < 2 {
                        continue;
                    }
                }

                matches.push(PatternMatch {
                    element_type: ElementType::Button,
                    text: text.to_string(),
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect inputs
        for pattern in &patterns.input {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());
                let value = cap
                    .get(2)
                    .map(|m| m.as_str().trim_end_matches('_').trim().to_string())
                    .filter(|v| !v.is_empty());

                matches.push(PatternMatch {
                    element_type: ElementType::Input,
                    text: full_match.as_str().to_string(),
                    label,
                    value,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect checkboxes
        for pattern in &patterns.checkbox {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_checked = matches!(marker, "x" | "X" | "✓" | "✔" | "◉" | "●");

                matches.push(PatternMatch {
                    element_type: ElementType::Checkbox,
                    text: full_match.as_str().to_string(),
                    label,
                    value: Some(if is_checked { "checked" } else { "unchecked" }.to_string()),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_checked),
                });
            }
        }

        // Detect radio buttons
        for pattern in &patterns.radio {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_selected = marker != " ";

                matches.push(PatternMatch {
                    element_type: ElementType::Radio,
                    text: full_match.as_str().to_string(),
                    label,
                    value: Some(
                        if is_selected {
                            "selected"
                        } else {
                            "unselected"
                        }
                        .to_string(),
                    ),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_selected),
                });
            }
        }

        // Detect selects/dropdowns
        for pattern in &patterns.select {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());
                let value = cap.get(2).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Select,
                    text: full_match.as_str().to_string(),
                    label,
                    value,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect menu items
        for pattern in &patterns.menu_item {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap
                    .get(2)
                    .or_else(|| cap.get(1))
                    .map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::MenuItem,
                    text: full_match.as_str().to_string(),
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect list items
        for pattern in &patterns.list_item {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::ListItem,
                    text: full_match.as_str().to_string(),
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect spinners
        for pattern in &patterns.spinner {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();

                matches.push(PatternMatch {
                    element_type: ElementType::Spinner,
                    text: full_match.as_str().to_string(),
                    label: None,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        // Detect progress bars
        for pattern in &patterns.progress {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let value = cap.get(1).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Progress,
                    text: full_match.as_str().to_string(),
                    label: None,
                    value,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }
    }

    // Deduplicate overlapping matches
    deduplicate_matches(matches)
}

/// Type priority for deduplication
fn type_priority(t: &ElementType) -> i32 {
    match t {
        ElementType::Input => 10,
        ElementType::Checkbox => 9,
        ElementType::Radio => 9,
        ElementType::Select => 8,
        ElementType::Button => 7,
        ElementType::MenuItem => 6,
        ElementType::ListItem => 5,
        ElementType::Spinner => 4,
        ElementType::Progress => 3,
        ElementType::Link => 2,
        ElementType::Text => 1,
        ElementType::Container => 1,
        ElementType::Unknown => 0,
    }
}

/// Remove overlapping matches, preferring higher-priority types
fn deduplicate_matches(mut matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
    // Sort by priority (higher first), then by position
    matches.sort_by(|a, b| {
        let priority_cmp = type_priority(&b.element_type).cmp(&type_priority(&a.element_type));
        if priority_cmp != std::cmp::Ordering::Equal {
            return priority_cmp;
        }
        if a.row != b.row {
            return a.row.cmp(&b.row);
        }
        a.col.cmp(&b.col)
    });

    let mut result = Vec::new();
    let mut occupied: HashSet<(u16, u16)> = HashSet::new();

    for m in matches {
        // Check if any position is already occupied
        let mut overlaps = false;
        for c in m.col..(m.col + m.width) {
            if occupied.contains(&(m.row, c)) {
                overlaps = true;
                break;
            }
        }

        if !overlaps {
            // Mark positions as occupied
            for c in m.col..(m.col + m.width) {
                occupied.insert((m.row, c));
            }
            result.push(m);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_button() {
        let screen = "[Submit] [Cancel]";
        let matches = detect_by_pattern(screen);

        assert_eq!(matches.len(), 2);
        assert!(matches
            .iter()
            .all(|m| m.element_type == ElementType::Button));
    }

    #[test]
    fn test_detect_checkbox() {
        let screen = "[x] Accept terms\n[ ] Subscribe to newsletter";
        let matches = detect_by_pattern(screen);

        let checkboxes: Vec<_> = matches
            .iter()
            .filter(|m| m.element_type == ElementType::Checkbox)
            .collect();

        assert_eq!(checkboxes.len(), 2);
        assert_eq!(checkboxes[0].checked, Some(true));
        assert_eq!(checkboxes[1].checked, Some(false));
    }

    #[test]
    fn test_detect_input() {
        let screen = "Name: [John Doe___]";
        let matches = detect_by_pattern(screen);

        let inputs: Vec<_> = matches
            .iter()
            .filter(|m| m.element_type == ElementType::Input)
            .collect();

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].label, Some("Name".to_string()));
        assert_eq!(inputs[0].value, Some("John Doe".to_string()));
    }

    #[test]
    fn test_detect_menu_item() {
        let screen = "  > Option 1\n    Option 2\n    Option 3";
        let matches = detect_by_pattern(screen);

        let menu_items: Vec<_> = matches
            .iter()
            .filter(|m| m.element_type == ElementType::MenuItem)
            .collect();

        assert!(!menu_items.is_empty());
    }

    #[test]
    fn test_deduplication() {
        // Input pattern should take priority over button pattern
        let screen = "[value___]";
        let matches = detect_by_pattern(screen);

        // Should only have one match (input, not button)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Input);
    }
}
