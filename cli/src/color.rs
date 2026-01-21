use std::io::IsTerminal;
use std::sync::OnceLock;

static NO_COLOR: OnceLock<bool> = OnceLock::new();

pub fn init(no_color_flag: bool) {
    let _ = NO_COLOR
        .set(no_color_flag || std::env::var("NO_COLOR").is_ok() || !std::io::stdout().is_terminal());
}

pub fn is_disabled() -> bool {
    *NO_COLOR.get().unwrap_or(&false)
}

mod codes {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[90m";
    pub const BOLD: &str = "\x1b[1m";
}

pub struct Colors;

impl Colors {
    pub fn success(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::GREEN, text, codes::RESET)
        }
    }

    pub fn error(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::RED, text, codes::RESET)
        }
    }

    pub fn info(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::CYAN, text, codes::RESET)
        }
    }

    pub fn dim(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::DIM, text, codes::RESET)
        }
    }

    pub fn bold(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::BOLD, text, codes::RESET)
        }
    }

    pub fn element_ref(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}", codes::CYAN, text, codes::RESET)
        }
    }

    pub fn session_id(text: &str) -> String {
        if is_disabled() {
            text.to_string()
        } else {
            format!("{}{}{}{}", codes::BOLD, codes::CYAN, text, codes::RESET)
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
