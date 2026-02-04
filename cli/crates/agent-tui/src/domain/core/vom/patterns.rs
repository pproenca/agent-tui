//! VOM pattern definitions.

const BRAILLE_SPINNERS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const CIRCLE_SPINNERS: [char; 4] = ['◐', '◑', '◒', '◓'];

const STATUS_CHARS: [char; 4] = ['✓', '✔', '✗', '✘'];

const ROUNDED_CORNERS: [char; 4] = ['╭', '╮', '╰', '╯'];

const BOX_CHARS: [char; 22] = [
    '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '═', '║', '╔', '╗', '╚', '╝', '╠', '╣',
    '╦', '╩', '╬',
];

const MIN_BUTTON_LENGTH: usize = 3;

const PROGRESS_BAR_CHARS: [char; 8] = ['=', '>', '#', '.', '█', '▓', '░', '-'];

pub fn is_button_text(text: &str) -> bool {
    if text.len() < MIN_BUTTON_LENGTH {
        return false;
    }

    if let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let trimmed = inner.trim();
        if matches!(trimmed, "x" | "X" | " " | "" | "✓" | "✔") {
            return false;
        }

        if inner.chars().any(|c| c.is_alphabetic()) {
            return true;
        }

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

const MENU_ITEM_DASH_PREFIX: &str = "- ";

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

pub fn is_panel_border(text: &str) -> bool {
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }

    let box_count = text.chars().filter(|c| BOX_CHARS.contains(c)).count();
    box_count > total / 2
}

pub fn is_status_indicator(text: &str) -> bool {
    let text = text.trim();
    let Some(first_char) = text.chars().next() else {
        return false;
    };

    BRAILLE_SPINNERS.contains(&first_char)
        || CIRCLE_SPINNERS.contains(&first_char)
        || STATUS_CHARS.contains(&first_char)
}

pub fn is_tool_block_border(text: &str) -> bool {
    let text = text.trim();
    let Some(first_char) = text.chars().next() else {
        return false;
    };

    let last_char = text.chars().last().unwrap_or(first_char);

    ROUNDED_CORNERS.contains(&first_char) || ROUNDED_CORNERS.contains(&last_char)
}

pub fn is_prompt_marker(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == ">" || trimmed == "> "
}

const PROGRESS_FILLED: [char; 4] = ['█', '▓', '▒', '='];
const PROGRESS_EMPTY: [char; 4] = ['░', '▒', ' ', '.'];
const PROGRESS_ARROW: char = '>';

pub fn is_progress_bar(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

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

    let total_chars = text.chars().count();
    let progress_chars: usize = text
        .chars()
        .filter(|c| PROGRESS_FILLED.contains(c) || PROGRESS_EMPTY.contains(c))
        .count();

    progress_chars > total_chars / 2
}

pub fn is_link(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if text.starts_with("https://")
        || text.starts_with("http://")
        || text.starts_with("file://")
        || text.starts_with("ftp://")
    {
        return true;
    }

    is_file_path(text)
}

fn is_file_path(text: &str) -> bool {
    let path_part = text.split(':').next().unwrap_or(text);

    if path_part.starts_with('/') && path_part.len() > 1 {
        return has_file_extension(path_part) || path_part.contains('/');
    }

    if path_part.starts_with("./") || path_part.starts_with("../") {
        return true;
    }

    if path_part.contains('/') && has_file_extension(path_part) {
        return true;
    }

    false
}

fn has_file_extension(text: &str) -> bool {
    const EXTENSIONS: [&str; 30] = [
        ".rs", ".js", ".ts", ".tsx", ".jsx", ".py", ".go", ".java", ".c", ".cpp", ".h", ".hpp",
        ".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".html", ".css", ".sh", ".sql", ".xml",
        ".vue", ".svelte", ".rb", ".php", ".swift", ".kt", ".scala",
    ];
    EXTENSIONS.iter().any(|ext| text.ends_with(ext))
}

const ERROR_PREFIXES: [&str; 6] = ["Error:", "error:", "ERROR:", "Error ", "error ", "ERROR "];
const FAILURE_CHARS: [char; 2] = ['✗', '✘'];

pub fn is_error_message(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if ERROR_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
        return true;
    }

    if let Some(first_char) = text.chars().next()
        && FAILURE_CHARS.contains(&first_char)
    {
        return true;
    }

    false
}

pub fn is_diff_line(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if text.starts_with("@@") {
        return true;
    }

    if text.starts_with('+') && text.len() > 1 {
        return true;
    }

    if text.starts_with('-') && text.len() > 1 {
        return true;
    }

    false
}

const CODE_BLOCK_BORDER: char = '│';

pub fn is_code_block_border(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    if !text.contains(CODE_BLOCK_BORDER) {
        return false;
    }

    const CORNER_CHARS: [char; 8] = ['┌', '┐', '└', '┘', '╭', '╮', '╰', '╯'];
    if text.chars().any(|c| CORNER_CHARS.contains(&c)) {
        return false;
    }

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
        assert!(is_input_field("_"));

        assert!(!is_input_field("Hello"));
        assert!(!is_input_field(""));
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
        assert!(is_progress_bar("████████"));

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
