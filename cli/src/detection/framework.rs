#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    Ink,
    Blessed,
    BubbleTea,
    Textual,
    Ncurses,
    Inquirer,
    Prompts,
    Ratatui,
    Unknown,
}

// Character signature constants for framework detection
mod signatures {
    pub const BRAILLE_SPINNERS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    pub const CHARM_SPINNERS: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
    pub const BLOCK_CHARS: &[&str] = &["█", "▓", "▒", "░", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    pub const SPARKLINE_CHARS: &[&str] = &["▁", "▂", "▃", "▄", "▅", "▆", "▇"];
    pub const SELECT_POINTERS: &[char] = &['❯', '›'];
    pub const INQUIRER_CIRCLES: &[char] = &['◯', '◉'];
    pub const HEAVY_BORDERS: &[&str] = &["┏", "┓", "┗"];
}

fn has_any(screen: &str, chars: &[&str]) -> bool {
    chars.iter().any(|c| screen.contains(c))
}

fn has_all(screen: &str, chars: &[&str]) -> bool {
    chars.iter().all(|c| screen.contains(c))
}

fn count_matches(screen: &str, chars: &[&str]) -> usize {
    chars.iter().filter(|c| screen.contains(*c)).count()
}

fn line_starts_with_any(screen: &str, chars: &[char]) -> bool {
    screen.lines().any(|l| {
        let trimmed = l.trim();
        chars.iter().any(|c| trimmed.starts_with(*c))
    })
}

type Detector = fn(&str) -> bool;

const DETECTORS: &[(Framework, Detector)] = &[
    (Framework::Inquirer, is_inquirer),
    (Framework::Ink, is_ink),
    (Framework::Prompts, is_prompts),
    (Framework::BubbleTea, is_bubbletea),
    (Framework::Textual, is_textual),
    (Framework::Blessed, is_blessed),
    (Framework::Ratatui, is_ratatui),
    (Framework::Ncurses, is_ncurses),
];

pub fn detect_framework(screen: &str) -> Framework {
    DETECTORS
        .iter()
        .find(|(_, detect)| detect(screen))
        .map(|(fw, _)| *fw)
        .unwrap_or(Framework::Unknown)
}

fn is_ink(screen: &str) -> bool {
    use signatures::*;
    let has_spinner = has_any(screen, BRAILLE_SPINNERS);
    let has_pointer = SELECT_POINTERS.iter().any(|c| screen.contains(*c));
    let has_select = has_pointer && line_starts_with_any(screen, SELECT_POINTERS);
    has_spinner || has_select || (screen.contains("?") && has_pointer)
}

fn is_inquirer(screen: &str) -> bool {
    use signatures::*;
    let circle_count = screen
        .lines()
        .filter(|l| INQUIRER_CIRCLES.iter().any(|c| l.trim().starts_with(*c)))
        .count();
    circle_count >= 2
        || screen.contains("◻")
        || screen.contains("◼")
        || screen.contains("(Y/n)")
        || screen.contains("(y/N)")
}

fn is_prompts(screen: &str) -> bool {
    let toggle = screen.contains("◉") && screen.contains("○");
    let select = screen.contains("›") && screen.lines().any(|l| l.contains("←") || l.contains("→"));
    toggle || select
}

fn is_bubbletea(screen: &str) -> bool {
    use signatures::*;
    let has_spinner = has_any(screen, CHARM_SPINNERS);
    let has_help = ["q: quit", "ctrl+c", "esc: back", "enter: select"]
        .iter()
        .any(|s| screen.contains(s));
    let has_input = screen.contains("│") && screen.lines().any(|l| l.contains(">"));
    has_spinner || (has_help && has_input)
}

fn is_textual(screen: &str) -> bool {
    use signatures::*;
    let footer_keys = ["^q", "^c", "F1", "ESC"];
    let has_footer = screen
        .lines()
        .last()
        .is_some_and(|l| footer_keys.iter().any(|k| l.contains(k)));
    let has_borders = has_all(screen, HEAVY_BORDERS);
    let has_table = has_all(screen, &["│", "─", "┼"]);
    has_footer || has_borders || has_table
}

fn is_blessed(screen: &str) -> bool {
    let has_scrollbar = screen.contains("▒") || screen.contains("░");
    let has_input = has_all(screen, &["[", "]"]) && screen.contains("_____");
    let has_box = has_all(screen, &["┌─", "─┐", "└─"]);
    (has_scrollbar && has_box) || has_input
}

fn is_ratatui(screen: &str) -> bool {
    use signatures::*;
    count_matches(screen, BLOCK_CHARS) >= 3 || count_matches(screen, SPARKLINE_CHARS) >= 3
}

fn is_ncurses(screen: &str) -> bool {
    let has_fkeys = ["F1", "F2", "F10", "^X"].iter().any(|k| screen.contains(k));
    let has_box = has_all(screen, &["+-", "-+"]) && (screen.contains("|") || screen.contains("│"));
    let has_menu = screen
        .lines()
        .next()
        .is_some_and(|l| l.contains("File") || l.contains("Help"));
    (has_fkeys && has_box) || has_menu
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
