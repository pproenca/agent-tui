//! Terminal color helpers.

use std::io::IsTerminal;
use std::sync::OnceLock;

static NO_COLOR: OnceLock<bool> = OnceLock::new();

pub fn init(no_color_flag: bool) {
    let _ = NO_COLOR.set(
        no_color_flag || std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal(),
    );
}

pub fn is_disabled() -> bool {
    *NO_COLOR.get().unwrap_or(&false)
}

mod codes {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[90m";
    pub const BOLD: &str = "\x1b[1m";
}

pub struct Colors;

fn wrap_with_ansi(text: &str, prefixes: &[&str]) -> String {
    let prefix_len: usize = prefixes.iter().map(|prefix| prefix.len()).sum();
    let mut out = String::with_capacity(prefix_len + text.len() + codes::RESET.len());
    for prefix in prefixes {
        out.push_str(prefix);
    }
    out.push_str(text);
    out.push_str(codes::RESET);
    out
}

impl Colors {
    pub fn success(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::GREEN])
        }
    }

    pub fn error(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::RED])
        }
    }

    pub fn info(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::CYAN])
        }
    }

    pub fn warning(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::YELLOW])
        }
    }

    pub fn dim(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::DIM])
        }
    }

    pub fn bold(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::BOLD])
        }
    }

    pub fn session_id(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            wrap_with_ansi(text, &[codes::BOLD, codes::CYAN])
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
    }
}
