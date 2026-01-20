//! Color output utilities for agent-tui CLI
//!
//! Respects NO_COLOR environment variable and provides consistent styling.

use std::sync::OnceLock;

/// Global flag to control color output
static NO_COLOR: OnceLock<bool> = OnceLock::new();

/// Initialize color state based on environment and CLI flags
pub fn init(no_color_flag: bool) {
    let _ = NO_COLOR
        .set(no_color_flag || std::env::var("NO_COLOR").is_ok() || !atty::is(atty::Stream::Stdout));
}

/// Check if colors are disabled
pub fn is_disabled() -> bool {
    *NO_COLOR.get().unwrap_or(&false)
}

/// ANSI color codes
mod codes {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const DIM: &str = "\x1b[90m";
    pub const BOLD: &str = "\x1b[1m";
}

/// Color helper functions
pub struct Colors;

impl Colors {
    /// Success messages (green)
    pub fn success(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::GREEN, text, codes::RESET)
        }
    }

    /// Error messages (red)
    pub fn error(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::RED, text, codes::RESET)
        }
    }

    /// Info messages (cyan)
    pub fn info(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::CYAN, text, codes::RESET)
        }
    }

    /// Warning messages (yellow)
    pub fn warning(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::YELLOW, text, codes::RESET)
        }
    }

    /// Dimmed text (gray)
    pub fn dim(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::DIM, text, codes::RESET)
        }
    }

    /// Bold text
    pub fn bold(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::BOLD, text, codes::RESET)
        }
    }

    /// Element references (cyan, for @inp1, @btn1, etc.)
    pub fn element_ref(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::CYAN, text, codes::RESET)
        }
    }

    /// Session ID (bold cyan)
    pub fn session_id(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}{}", codes::BOLD, codes::CYAN, text, codes::RESET)
        }
    }

    /// Status indicator based on boolean
    pub fn status(running: bool) -> String {
        if running {
            Self::success("running")
        } else {
            Self::dim("exited")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colors_disabled() {
        let _ = NO_COLOR.set(true);
        assert_eq!(Colors::success("test"), "test");
        assert_eq!(Colors::error("test"), "test");
        assert_eq!(Colors::element_ref("@inp1"), "@inp1");
    }
}
