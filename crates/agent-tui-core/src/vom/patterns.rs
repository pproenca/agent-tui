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

/// Progress bar characters that indicate the content is a progress bar, not a button.
const PROGRESS_BAR_CHARS: [char; 8] = ['=', '>', '#', '.', '█', '▓', '░', '-'];

/// Detect button-like text patterns.
///
/// Buttons are identified by bracketed text like `[Submit]`, `<OK>`, or `(Cancel)`.
/// Excludes checkbox patterns like `[x]` or `[ ]` and progress bars like `[===>  ]`.
pub fn is_button_text(text: &str) -> bool {
    if text.len() < MIN_BUTTON_LENGTH {
        return false;
    }

    if let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let trimmed = inner.trim();
        if matches!(trimmed, "x" | "X" | " " | "" | "✓" | "✔") {
            return false;
        }
        // If there are any alphabetic characters, it's a button
        if inner.chars().any(|c| c.is_alphabetic()) {
            return true;
        }
        // Exclude progress bar patterns: content is mostly progress characters (not counting spaces)
        let (progress_chars, non_space_chars) = inner.chars().fold((0, 0), |(p, n), c| {
            (
                if PROGRESS_BAR_CHARS.contains(&c) {
                    p + 1
                } else {
                    p
                },
                if !c.is_whitespace() { n + 1 } else { n },
            )
        });
        if non_space_chars > 0 && progress_chars > non_space_chars / 2 {
            return false;
        }
        return true;
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

/// Menu item dash prefix: "- " (dash followed by space).
/// Distinct from diff deletion prefix which is "-" followed by non-space.
const MENU_ITEM_DASH_PREFIX: &str = "- ";

/// Detect menu item patterns.
///
/// Menu items typically start with arrow or bullet characters.
/// Note: "- text" (dash + space) is a menu item, while "-text" is a diff deletion.
pub fn is_menu_item(text: &str) -> bool {
    text.starts_with('>')
        || text.starts_with('❯')
        || text.starts_with('›')
        || text.starts_with('→')
        || text.starts_with('▶')
        || text.starts_with("• ")
        || text.starts_with("* ")
        || text.starts_with(MENU_ITEM_DASH_PREFIX)
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
    // Safe: last() always exists when first exists (non-empty string)
    let last_char = text
        .chars()
        .last()
        .expect("non-empty string has a last char");

    ROUNDED_CORNERS.contains(&first_char) || ROUNDED_CORNERS.contains(&last_char)
}

/// Detect prompt marker patterns.
///
/// Claude Code uses ">" as the input prompt marker.
pub fn is_prompt_marker(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == ">" || trimmed == "> "
}

/// Progress bar block characters (filled and empty).
const PROGRESS_FILLED: [char; 4] = ['█', '▓', '▒', '='];
const PROGRESS_EMPTY: [char; 4] = ['░', '▒', ' ', '.'];
const PROGRESS_ARROW: char = '>';

/// Detect progress bar patterns.
///
/// Progress bars use block characters like `████░░░░` or bracket style `[===>  ]`.
pub fn is_progress_bar(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    // Bracket-style progress: [===>    ] or [####....]
    if text.starts_with('[') && text.ends_with(']') {
        let inner = &text[1..text.len() - 1];
        if inner.is_empty() {
            return false;
        }
        let progress_chars: usize = inner
            .chars()
            .filter(|c| PROGRESS_FILLED.contains(c) || *c == PROGRESS_ARROW || *c == '#')
            .count();
        let empty_chars: usize = inner
            .chars()
            .filter(|c| PROGRESS_EMPTY.contains(c) || *c == '-')
            .count();
        return progress_chars + empty_chars > inner.len() / 2;
    }

    // Block-style progress: ████░░░░
    let total_chars = text.chars().count();
    let progress_chars: usize = text
        .chars()
        .filter(|c| PROGRESS_FILLED.contains(c) || PROGRESS_EMPTY.contains(c))
        .count();

    progress_chars > total_chars / 2
}

/// Detect link patterns (URLs and file paths).
///
/// Links include:
/// - URLs: `https://example.com`, `http://...`, `file://...`
/// - File paths: `src/main.rs`, `/absolute/path.txt`, `./relative/path`
/// - File paths with line numbers: `src/main.rs:42`, `path/file.rs:123:45`
pub fn is_link(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    // URL patterns
    if text.starts_with("https://")
        || text.starts_with("http://")
        || text.starts_with("file://")
        || text.starts_with("ftp://")
    {
        return true;
    }

    // File path heuristics
    is_file_path(text)
}

/// Check if text looks like a file path.
fn is_file_path(text: &str) -> bool {
    // Strip line number suffix like :42 or :123:45
    let path_part = text.split(':').next().unwrap_or(text);

    // Absolute paths
    if path_part.starts_with('/') && path_part.len() > 1 {
        return has_file_extension(path_part) || path_part.contains('/');
    }

    // Relative paths starting with ./ or ../
    if path_part.starts_with("./") || path_part.starts_with("../") {
        return true;
    }

    // Paths containing / with file extension
    if path_part.contains('/') && has_file_extension(path_part) {
        return true;
    }

    false
}

/// Check if text ends with a common file extension.
fn has_file_extension(text: &str) -> bool {
    const EXTENSIONS: [&str; 30] = [
        ".rs", ".js", ".ts", ".tsx", ".jsx", ".py", ".go", ".java", ".c", ".cpp", ".h", ".hpp",
        ".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".html", ".css", ".sh", ".sql", ".xml",
        ".vue", ".svelte", ".rb", ".php", ".swift", ".kt", ".scala",
    ];
    EXTENSIONS.iter().any(|ext| text.ends_with(ext))
}

/// Error message prefixes.
const ERROR_PREFIXES: [&str; 6] = ["Error:", "error:", "ERROR:", "Error ", "error ", "ERROR "];
/// Failure indicator characters.
const FAILURE_CHARS: [char; 2] = ['✗', '✘'];

/// Detect error message patterns.
///
/// Error messages start with error prefixes or failure markers.
pub fn is_error_message(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    // Check error prefixes
    if ERROR_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        return true;
    }

    // Check failure markers
    if let Some(first_char) = text.chars().next() {
        if FAILURE_CHARS.contains(&first_char) {
            return true;
        }
    }

    false
}

/// Detect diff line patterns.
///
/// Diff lines start with `+`, `-`, or `@@`.
///
/// Note: Both `is_diff_line` and `is_menu_item` may return true for strings
/// starting with "- ". The classifier determines which role takes priority
/// based on classification order (menu items are checked before diff lines).
pub fn is_diff_line(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    // Diff header
    if text.starts_with("@@") {
        return true;
    }

    // Addition line: + followed by content
    if text.starts_with('+') && text.len() > 1 {
        return true;
    }

    // Deletion line: - followed by content
    if text.starts_with('-') && text.len() > 1 {
        return true;
    }

    false
}

/// Code block border character (vertical line).
const CODE_BLOCK_BORDER: char = '│';

/// Detect code block border patterns.
///
/// Code blocks in Claude Code use `│` vertical borders.
pub fn is_code_block_border(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    // Must contain the vertical border character
    if !text.contains(CODE_BLOCK_BORDER) {
        return false;
    }

    // Should not be a full panel border (with corners)
    const CORNER_CHARS: [char; 8] = ['┌', '┐', '└', '┘', '╭', '╮', '╰', '╯'];
    if text.chars().any(|c| CORNER_CHARS.contains(&c)) {
        return false;
    }

    // Count border vs non-border characters
    let border_count = text.chars().filter(|c| *c == CODE_BLOCK_BORDER).count();

    (1..=3).contains(&border_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    // ============================================================
    // NEW ROLE TESTS - Phase 1: RED (failing tests for new roles)
    // ============================================================

    #[test]
    fn test_progress_bar_block_style() {
        assert!(is_progress_bar("████░░░░"));
        assert!(is_progress_bar("▓▓▓▓░░░░"));
        assert!(is_progress_bar("███████░░░"));
    }

    #[test]
    fn test_progress_bar_bracket_style() {
        assert!(is_progress_bar("[===>    ]"));
        assert!(is_progress_bar("[####....]"));
        assert!(is_progress_bar("[========]"));
    }

    #[test]
    fn test_progress_bar_threshold() {
        // More than 50% progress chars should be detected
        assert!(is_progress_bar("████████"));
        // Less than 50% progress chars should not be detected
        assert!(!is_progress_bar("█ text here"));
    }

    #[test]
    fn test_progress_bar_not_regular_text() {
        assert!(!is_progress_bar("Hello World"));
        assert!(!is_progress_bar("Loading..."));
        assert!(!is_progress_bar(""));
    }

    #[test]
    fn test_link_urls() {
        assert!(is_link("https://example.com"));
        assert!(is_link("http://localhost:3000"));
        assert!(is_link("file:///path/to/file"));
        assert!(is_link("https://github.com/user/repo"));
    }

    #[test]
    fn test_link_file_paths() {
        assert!(is_link("src/main.rs"));
        assert!(is_link("/absolute/path.txt"));
        assert!(is_link("./relative/path.js"));
        assert!(is_link("../parent/file.py"));
    }

    #[test]
    fn test_link_file_paths_with_line_numbers() {
        assert!(is_link("src/main.rs:42"));
        assert!(is_link("path/file.rs:123:45"));
        assert!(is_link("/absolute/path.txt:10"));
    }

    #[test]
    fn test_link_not_regular_text() {
        assert!(!is_link("Hello World"));
        assert!(!is_link("normal text"));
        assert!(!is_link(""));
    }

    #[test]
    fn test_error_message_prefixes() {
        assert!(is_error_message("Error: something went wrong"));
        assert!(is_error_message("error: compilation failed"));
        assert!(is_error_message("ERROR: critical failure"));
    }

    #[test]
    fn test_error_message_failure_markers() {
        assert!(is_error_message("✗ Failed to connect"));
        assert!(is_error_message("✘ Error occurred"));
    }

    #[test]
    fn test_error_message_not_regular_text() {
        assert!(!is_error_message("Hello World"));
        assert!(!is_error_message("Success!"));
        assert!(!is_error_message(""));
    }

    #[test]
    fn test_diff_line_additions() {
        assert!(is_diff_line("+ added line"));
        assert!(is_diff_line("+added"));
    }

    #[test]
    fn test_diff_line_deletions() {
        assert!(is_diff_line("- removed line"));
        assert!(is_diff_line("-removed"));
    }

    #[test]
    fn test_diff_line_headers() {
        assert!(is_diff_line("@@ -1,5 +1,6 @@"));
        assert!(is_diff_line("@@"));
    }

    #[test]
    fn test_diff_line_not_regular_text() {
        assert!(!is_diff_line("Hello World"));
        assert!(!is_diff_line("normal text"));
        assert!(!is_diff_line(""));
    }

    #[test]
    fn test_code_block_border() {
        assert!(is_code_block_border("│ let x = 5;"));
        assert!(is_code_block_border("│"));
        assert!(is_code_block_border("  │ fn main() {"));
    }

    #[test]
    fn test_code_block_not_panel_border() {
        // Panel borders (full box) should not match
        assert!(!is_code_block_border("┌──────────┐"));
        assert!(!is_code_block_border("└──────────┘"));
    }

    #[test]
    fn test_code_block_not_regular_text() {
        assert!(!is_code_block_border("Hello World"));
        assert!(!is_code_block_border("normal text"));
        assert!(!is_code_block_border(""));
    }

    proptest! {
        #[test]
        fn progress_bar_detection_is_deterministic(input in ".*") {
            let result1 = is_progress_bar(&input);
            let result2 = is_progress_bar(&input);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn link_detection_is_deterministic(input in ".*") {
            let result1 = is_link(&input);
            let result2 = is_link(&input);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn error_message_detection_is_deterministic(input in ".*") {
            let result1 = is_error_message(&input);
            let result2 = is_error_message(&input);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn diff_line_detection_is_deterministic(input in ".*") {
            let result1 = is_diff_line(&input);
            let result2 = is_diff_line(&input);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn code_block_border_detection_is_deterministic(input in ".*") {
            let result1 = is_code_block_border(&input);
            let result2 = is_code_block_border(&input);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn url_links_always_detected(
            protocol in "(https?|file|ftp)://",
            domain in "[a-z]{3,10}\\.[a-z]{2,5}"
        ) {
            let url = format!("{}{}", protocol, domain);
            prop_assert!(is_link(&url), "URL should be detected: {}", url);
        }

        #[test]
        fn file_paths_with_extension_detected(
            dir in "[a-z]{2,8}",
            file in "[a-z]{2,10}",
            ext in "(rs|js|ts|py|go)"
        ) {
            let path = format!("{}/{}.{}", dir, file, ext);
            prop_assert!(is_link(&path), "File path should be detected: {}", path);
        }

        #[test]
        fn progress_bar_block_style_detected(
            filled in "[█▓]{2,10}",
            empty in "[░]{2,10}"
        ) {
            let progress = format!("{}{}", filled, empty);
            prop_assert!(is_progress_bar(&progress), "Progress bar should be detected: {}", progress);
        }

        #[test]
        fn error_prefixes_always_detected(
            prefix in "(Error:|error:|ERROR:)",
            message in "[a-z]{5,20}"
        ) {
            let error = format!("{} {}", prefix, message);
            prop_assert!(is_error_message(&error), "Error message should be detected: {}", error);
        }

        #[test]
        fn diff_addition_lines_detected(content in "[a-z]{3,20}") {
            let line = format!("+ {}", content);
            prop_assert!(is_diff_line(&line), "Diff addition should be detected: {}", line);
        }

        #[test]
        fn diff_deletion_lines_detected(content in "[a-z]{3,20}") {
            let line = format!("- {}", content);
            prop_assert!(is_diff_line(&line), "Diff deletion should be detected: {}", line);
        }
    }
}
