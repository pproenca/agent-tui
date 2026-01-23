//! Pattern detection for TUI element classification.
//!
//! This module contains pattern-matching functions used by the classifier
//! to identify specific UI elements. Patterns are grouped by their source
//! or purpose.

/// Spinner characters used in braille-style loading indicators.
/// Common in modern CLI tools including Claude Code.
const BRAILLE_SPINNERS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Circle spinner characters for progress indicators.
const CIRCLE_SPINNERS: [char; 4] = ['◐', '◑', '◒', '◓'];

/// Status indicator characters (checkmarks and crosses).
const STATUS_CHARS: [char; 4] = ['✓', '✔', '✗', '✘'];

/// Rounded corner characters used in tool block borders.
/// Claude Code uses these for tool use display blocks.
const ROUNDED_CORNERS: [char; 4] = ['╭', '╮', '╰', '╯'];

/// Box drawing characters for panel borders.
const BOX_CHARS: [char; 22] = [
    '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '═', '║', '╔', '╗', '╚', '╝', '╠', '╣',
    '╦', '╩', '╬',
];

/// Minimum length for bracketed text to be considered a button.
const MIN_BUTTON_LENGTH: usize = 3;

/// Detect button-like text patterns.
///
/// Buttons are identified by bracketed text like `[Submit]`, `<OK>`, or `(Cancel)`.
/// Excludes checkbox patterns like `[x]` or `[ ]`.
pub fn is_button_text(text: &str) -> bool {
    if text.len() < MIN_BUTTON_LENGTH {
        return false;
    }

    if let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let trimmed = inner.trim();
        return !matches!(trimmed, "x" | "X" | " " | "" | "✓" | "✔");
    }

    if let Some(inner) = text.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        let trimmed = inner.trim();
        return !matches!(trimmed, "" | " " | "o" | "O" | "●" | "◉");
    }

    text.starts_with('<') && text.ends_with('>')
}

/// Detect input field patterns.
///
/// Input fields are identified by underscore sequences like `___` or `Name: _`.
pub fn is_input_field(text: &str) -> bool {
    if text.contains("___") {
        return true;
    }

    if !text.is_empty() && text.chars().all(|ch| ch == '_') {
        return true;
    }

    if text.ends_with(": _") || text.ends_with(":_") {
        return true;
    }

    false
}

/// Detect checkbox patterns.
///
/// Supports various checkbox representations: `[x]`, `[ ]`, `☐`, `☑`, etc.
pub fn is_checkbox(text: &str) -> bool {
    matches!(
        text,
        "[x]"
            | "[X]"
            | "[ ]"
            | "[✓]"
            | "[✔]"
            | "◉"
            | "◯"
            | "●"
            | "○"
            | "◼"
            | "◻"
            | "☐"
            | "☑"
            | "☒"
    )
}

/// Detect menu item patterns.
///
/// Menu items typically start with arrow or bullet characters.
pub fn is_menu_item(text: &str) -> bool {
    text.starts_with('>')
        || text.starts_with('❯')
        || text.starts_with('›')
        || text.starts_with('→')
        || text.starts_with('▶')
        || text.starts_with("• ")
        || text.starts_with("* ")
        || text.starts_with("- ")
}

/// Detect panel border patterns.
///
/// Panels use box-drawing characters for borders.
pub fn is_panel_border(text: &str) -> bool {
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }

    let box_count = text.chars().filter(|c| BOX_CHARS.contains(c)).count();
    box_count > total / 2
}

/// Detect status indicator patterns.
///
/// Status indicators include spinners (braille, circle) and completion
/// markers (checkmarks, crosses). Common in Claude Code for "Thinking..."
/// and operation completion states.
pub fn is_status_indicator(text: &str) -> bool {
    let text = text.trim();
    let Some(first_char) = text.chars().next() else {
        return false;
    };

    BRAILLE_SPINNERS.contains(&first_char)
        || CIRCLE_SPINNERS.contains(&first_char)
        || STATUS_CHARS.contains(&first_char)
}

/// Detect tool block border patterns.
///
/// Claude Code displays tool use blocks with rounded corners (╭╮╰╯).
/// These are distinct from regular panel borders.
pub fn is_tool_block_border(text: &str) -> bool {
    let text = text.trim();
    let Some(first_char) = text.chars().next() else {
        return false;
    };
    let Some(last_char) = text.chars().last() else {
        return false;
    };

    ROUNDED_CORNERS.contains(&first_char) || ROUNDED_CORNERS.contains(&last_char)
}

/// Detect prompt marker patterns.
///
/// Claude Code uses ">" as the input prompt marker.
pub fn is_prompt_marker(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == ">" || trimmed == "> "
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_patterns() {
        assert!(is_button_text("[Submit]"));
        assert!(is_button_text("[OK]"));
        assert!(is_button_text("<Cancel>"));
        assert!(is_button_text("(Confirm)"));
        assert!(is_button_text("[Y]"));
        assert!(is_button_text("[N]"));

        assert!(!is_button_text("[x]"));
        assert!(!is_button_text("[ ]"));
        assert!(!is_button_text("[]"));
        assert!(!is_button_text("X"));
    }

    #[test]
    fn test_input_field_patterns() {
        assert!(is_input_field("Name: ___"));
        assert!(is_input_field("___________"));
        assert!(is_input_field("Value: _"));
        assert!(is_input_field("_")); // Single underscore is also an input field

        assert!(!is_input_field("Hello"));
        assert!(!is_input_field("")); // Empty is not
    }

    #[test]
    fn test_checkbox_patterns() {
        assert!(is_checkbox("[x]"));
        assert!(is_checkbox("[ ]"));
        assert!(is_checkbox("☐"));
        assert!(is_checkbox("☑"));

        assert!(!is_checkbox("[Submit]"));
        assert!(!is_checkbox("text"));
    }

    #[test]
    fn test_status_indicator_patterns() {
        assert!(is_status_indicator("⠋ Loading..."));
        assert!(is_status_indicator("✓ Done"));
        assert!(is_status_indicator("✔ Complete"));

        assert!(!is_status_indicator("Hello"));
        assert!(!is_status_indicator(""));
    }

    #[test]
    fn test_tool_block_patterns() {
        assert!(is_tool_block_border("╭─── Tool Use ───╮"));
        assert!(is_tool_block_border("╰────────────────╯"));

        assert!(!is_tool_block_border("┌─────────┐"));
        assert!(!is_tool_block_border("Hello"));
    }

    #[test]
    fn test_prompt_marker_patterns() {
        assert!(is_prompt_marker(">"));
        assert!(is_prompt_marker("> "));

        assert!(!is_prompt_marker(">>"));
        assert!(!is_prompt_marker("Hello"));
    }

    #[test]
    fn test_panel_border_patterns() {
        assert!(is_panel_border("┌──────────┐"));
        assert!(is_panel_border("│          │"));
        assert!(is_panel_border("└──────────┘"));

        assert!(!is_panel_border("Hello World"));
    }

    #[test]
    fn test_menu_item_patterns() {
        assert!(is_menu_item("> Option 1"));
        assert!(is_menu_item("❯ Selected"));
        assert!(is_menu_item("• Item"));
        assert!(is_menu_item("- List item"));

        assert!(!is_menu_item("Normal text"));
    }
}
