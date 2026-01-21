use super::ElementType;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

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
            Regex::new(r"\[\s*([^\[\]_]+?)\s*\]").unwrap(),
            Regex::new(r"<\s*([^<>]+?)\s*>").unwrap(),
        ],
        input: [
            Regex::new(r"([A-Za-z\s]+):\s*\[([^\]]*?)_*\]").unwrap(),
            Regex::new(r"\[([^\]]*?)_{3,}\]").unwrap(),
        ],
        checkbox: [
            Regex::new(r"\[([xX✓✔\s])\]\s*(.+?)(?:\s{2,}|$)").unwrap(),
            Regex::new(r"([◉◯●○✓✔])\s+(.+?)(?:\s{2,}|$)").unwrap(),
        ],
        radio: [Regex::new(r"\(([•o*\s])\)\s*(.+?)(?:\s{2,}|$)").unwrap()],
        select: [
            Regex::new(r"([A-Za-z\s]+):\s*\[?([^\]▼▾\n]+?)\s*[▼▾]\]?").unwrap(),
            Regex::new(r"([^\s]+)\s+[▼▾]").unwrap(),
        ],
        menu_item: [
            Regex::new(r"^\s*([>❯›▸►●•])\s+(.+?)$").unwrap(),
            Regex::new(r"^\s*(\d+)\.\s+(.+?)$").unwrap(),
        ],
        list_item: [
            Regex::new(r"^\s*[-*]\s+(.+?)$").unwrap(),
            Regex::new(r"^\s*•\s+(.+?)$").unwrap(),
        ],
        spinner: [
            Regex::new(r"[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]").unwrap(),
            Regex::new(r"[\|/\-\\]\s+(.+)").unwrap(),
        ],
        progress: [
            Regex::new(r"\[([█▓▒░=\->#\s]+)\]").unwrap(),
            Regex::new(r"(\d+)\s*%").unwrap(),
        ],
    })
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub element_type: ElementType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub checked: Option<bool>,
}

pub fn detect_by_pattern(screen_text: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let lines: Vec<&str> = screen_text.lines().collect();

    let patterns = get_patterns();

    for (row_idx, line) in lines.iter().enumerate() {
        let row = row_idx as u16;

        for pattern in &patterns.button {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());

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

                if let Some(ref l) = label {
                    if l.len() < 2 {
                        continue;
                    }
                }

                matches.push(PatternMatch {
                    element_type: ElementType::Button,
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

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
                    label,
                    value,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        for pattern in &patterns.checkbox {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_checked = matches!(marker, "x" | "X" | "✓" | "✔" | "◉" | "●");

                matches.push(PatternMatch {
                    element_type: ElementType::Checkbox,
                    label,
                    value: Some(if is_checked { "checked" } else { "unchecked" }.to_string()),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_checked),
                });
            }
        }

        for pattern in &patterns.radio {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_selected = marker != " ";

                matches.push(PatternMatch {
                    element_type: ElementType::Radio,
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

        for pattern in &patterns.select {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());
                let value = cap.get(2).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Select,
                    label,
                    value,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        for pattern in &patterns.menu_item {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap
                    .get(2)
                    .or_else(|| cap.get(1))
                    .map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::MenuItem,
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        for pattern in &patterns.list_item {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::ListItem,
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        for pattern in &patterns.spinner {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();

                matches.push(PatternMatch {
                    element_type: ElementType::Spinner,
                    label: None,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        for pattern in &patterns.progress {
            for cap in pattern.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let value = cap.get(1).map(|m| m.as_str().trim().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Progress,
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

    deduplicate_matches(matches)
}

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
    }
}

pub fn deduplicate_matches(mut matches: Vec<PatternMatch>) -> Vec<PatternMatch> {
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
        let mut overlaps = false;
        for c in m.col..(m.col + m.width) {
            if occupied.contains(&(m.row, c)) {
                overlaps = true;
                break;
            }
        }

        if !overlaps {
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
        let screen = "[value___]";
        let matches = detect_by_pattern(screen);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Input);
    }
}
