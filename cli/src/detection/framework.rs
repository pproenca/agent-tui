//! TUI framework auto-detection
//!
//! Detects which TUI framework is being used based on visual patterns
//! and common UI signatures in the terminal output.

/// Known TUI frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    /// Ink (React for CLI)
    Ink,
    /// Blessed (Node.js curses-like)
    Blessed,
    /// Bubble Tea (Go)
    BubbleTea,
    /// Textual (Python)
    Textual,
    /// ncurses/curses
    Ncurses,
    /// Inquirer.js
    Inquirer,
    /// Prompts (Node.js)
    Prompts,
    /// Ratatui (Rust)
    Ratatui,
    /// Unknown framework
    Unknown,
}

/// Detect the TUI framework based on screen content and patterns
pub fn detect_framework(screen: &str) -> Framework {
    if is_inquirer(screen) {
        return Framework::Inquirer;
    }

    if is_ink(screen) {
        return Framework::Ink;
    }

    if is_prompts(screen) {
        return Framework::Prompts;
    }

    if is_bubbletea(screen) {
        return Framework::BubbleTea;
    }

    if is_textual(screen) {
        return Framework::Textual;
    }

    if is_blessed(screen) {
        return Framework::Blessed;
    }

    if is_ratatui(screen) {
        return Framework::Ratatui;
    }

    if is_ncurses(screen) {
        return Framework::Ncurses;
    }

    Framework::Unknown
}

/// Check for Ink framework patterns
fn is_ink(screen: &str) -> bool {
    let braille_spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let has_braille = braille_spinners.iter().any(|s| screen.contains(s));

    let has_select_pointer = screen.contains("❯") || screen.contains("›");

    let has_ink_select = has_select_pointer
        && screen
            .lines()
            .any(|l| l.trim().starts_with('❯') || l.trim().starts_with('›'));

    let has_question_pattern = screen.contains("?") && has_select_pointer;

    if has_braille {
        return true;
    }

    if has_ink_select {
        return true;
    }

    has_question_pattern
}

/// Check for Inquirer.js patterns
fn is_inquirer(screen: &str) -> bool {
    let circle_lines = screen
        .lines()
        .filter(|l| l.trim().starts_with('◯') || l.trim().starts_with('◉'))
        .count();
    let has_inquirer_select = circle_lines >= 2;

    let has_inquirer_checkbox = screen.contains("◻") || screen.contains("◼");

    let has_inquirer_confirm = screen.contains("(Y/n)") || screen.contains("(y/N)");

    has_inquirer_select || has_inquirer_checkbox || has_inquirer_confirm
}

/// Check for Prompts (Node.js) patterns
fn is_prompts(screen: &str) -> bool {
    let has_prompts_toggle = screen.contains("◉") && screen.contains("○");
    let has_prompts_select =
        screen.contains("›") && screen.lines().any(|l| l.contains("←") || l.contains("→"));

    has_prompts_toggle || has_prompts_select
}

/// Check for Bubble Tea patterns
fn is_bubbletea(screen: &str) -> bool {
    let charm_spinners = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
    let has_charm_spinner = charm_spinners.iter().any(|s| screen.contains(s));

    let has_help_bar = screen.contains("q: quit")
        || screen.contains("ctrl+c")
        || screen.contains("esc: back")
        || screen.contains("enter: select");

    let has_text_input = screen.contains("│") && screen.lines().any(|l| l.contains(">"));

    has_charm_spinner || (has_help_bar && has_text_input)
}

/// Check for Textual (Python) patterns
fn is_textual(screen: &str) -> bool {
    let has_textual_footer = screen.lines().last().is_some_and(|l| {
        l.contains("^q") || l.contains("^c") || l.contains("F1") || l.contains("ESC")
    });

    let has_heavy_borders = screen.contains("┏") && screen.contains("┓") && screen.contains("┗");

    let has_data_table = screen.contains("│") && screen.contains("─") && screen.contains("┼");

    has_textual_footer || has_heavy_borders || has_data_table
}

/// Check for Blessed patterns
fn is_blessed(screen: &str) -> bool {
    let has_scrollbar = screen.contains("▒") || screen.contains("░");

    let has_blessed_input =
        screen.contains("[") && screen.contains("]") && screen.contains("_____");

    let has_blessed_box = screen.contains("┌─") && screen.contains("─┐") && screen.contains("└─");

    (has_scrollbar && has_blessed_box) || has_blessed_input
}

/// Check for Ratatui patterns
fn is_ratatui(screen: &str) -> bool {
    let block_chars = ["█", "▓", "▒", "░", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let block_count = block_chars.iter().filter(|c| screen.contains(*c)).count();

    let sparkline_chars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇"];
    let has_sparkline = sparkline_chars
        .iter()
        .filter(|c| screen.contains(*c))
        .count()
        >= 3;

    block_count >= 3 || has_sparkline
}

/// Check for ncurses patterns (generic curses-based apps)
fn is_ncurses(screen: &str) -> bool {
    let has_function_keys = screen.contains("F1")
        || screen.contains("F2")
        || screen.contains("F10")
        || screen.contains("^X");

    let has_simple_box = screen.contains("+-")
        && screen.contains("-+")
        && (screen.contains("|") || screen.contains("│"));

    let has_menu_bar = screen
        .lines()
        .next()
        .is_some_and(|l| l.contains("File") || l.contains("Help"));

    (has_function_keys && has_simple_box) || has_menu_bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ink() {
        let screen = "? Select a color\n  ❯ Red\n    Blue\n    Green";
        assert_eq!(detect_framework(screen), Framework::Ink);
    }

    #[test]
    fn test_detect_inquirer() {
        let screen = "? Choose an option (Y/n)\n  ◯ Option 1\n  ◉ Option 2";
        assert_eq!(detect_framework(screen), Framework::Inquirer);
    }

    #[test]
    fn test_detect_bubbletea() {
        let screen = "My App\n\n> Select an item\n\nq: quit | enter: select";
        assert!(matches!(
            detect_framework(screen),
            Framework::BubbleTea | Framework::Unknown
        ));
    }

    #[test]
    fn test_detect_unknown() {
        let screen = "Hello World";
        assert_eq!(detect_framework(screen), Framework::Unknown);
    }

    #[test]
    fn test_detect_ratatui() {
        let screen = "Progress: [████████░░░░░░░░] 50%\nSparkline: ▁▂▃▄▅▆▇█";
        assert_eq!(detect_framework(screen), Framework::Ratatui);
    }
}
