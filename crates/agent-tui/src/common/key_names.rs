pub const MODIFIER_PREFIXES: &[&str] = &["ctrl", "alt", "shift", "meta", "control"];

pub const SINGLE_KEY_NAMES: &[&str] = &[
    "enter",
    "tab",
    "escape",
    "esc",
    "backspace",
    "delete",
    "arrowup",
    "arrowdown",
    "arrowleft",
    "arrowright",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
    "insert",
    "space",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
    "shift",
    "control",
    "ctrl",
    "alt",
    "meta",
];

pub fn is_key_name(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();

    if lower.contains('+') {
        if let Some(modifier) = lower.split('+').next() {
            return MODIFIER_PREFIXES.contains(&modifier);
        }
    }

    SINGLE_KEY_NAMES.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_key_name_case_insensitive() {
        assert!(is_key_name("Enter"));
        assert!(is_key_name("Tab"));
        assert!(is_key_name("Escape"));
        assert!(is_key_name("F1"));

        assert!(is_key_name("enter"));
        assert!(is_key_name("tab"));
        assert!(is_key_name("escape"));
        assert!(is_key_name("f1"));

        assert!(is_key_name("ENTER"));
        assert!(is_key_name("TAB"));
        assert!(is_key_name("ESCAPE"));
        assert!(is_key_name("F1"));

        assert!(is_key_name("EnTeR"));
        assert!(is_key_name("ArrowUp"));
        assert!(is_key_name("arrowup"));
        assert!(is_key_name("ARROWUP"));
    }

    #[test]
    fn test_is_key_name_modifier_combos_case_insensitive() {
        assert!(is_key_name("Ctrl+c"));
        assert!(is_key_name("Alt+F4"));
        assert!(is_key_name("Shift+Tab"));

        assert!(is_key_name("ctrl+c"));
        assert!(is_key_name("alt+f4"));
        assert!(is_key_name("shift+tab"));

        assert!(is_key_name("CTRL+C"));
        assert!(is_key_name("ALT+F4"));
        assert!(is_key_name("SHIFT+TAB"));
    }

    #[test]
    fn test_is_key_name_text_not_detected() {
        assert!(!is_key_name("hello"));
        assert!(!is_key_name("Hello World"));
        assert!(!is_key_name("test123"));
        assert!(!is_key_name("a"));
    }

    #[test]
    fn test_is_key_name_edge_cases() {
        assert!(!is_key_name(""));

        assert!(is_key_name("Ctrl+"));
        assert!(is_key_name("Alt+"));

        assert!(!is_key_name("Invalid+X"));
        assert!(!is_key_name("foo+bar"));

        assert!(!is_key_name("こんにちは"));
        assert!(!is_key_name("你好"));
        assert!(!is_key_name("🎉"));

        assert!(!is_key_name(" "));
        assert!(!is_key_name("  "));
        assert!(!is_key_name("\t"));

        assert!(!is_key_name("1"));
        assert!(!is_key_name("123"));

        assert!(!is_key_name("Enter1"));
        assert!(!is_key_name("Tab!"));
    }
}
