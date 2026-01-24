//! Key name constants and validation for CLI input parsing.
//!
//! This module provides a centralized definition of recognized key names
//! used by the `input` command to distinguish between key presses and text input.

/// Valid modifier prefixes for key combinations (e.g., Ctrl+C, Alt+F4).
pub const MODIFIER_PREFIXES: &[&str] = &["ctrl", "alt", "shift", "meta", "control"];

/// Single key names recognized by the CLI (case-insensitive matching).
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

/// Check if input is a recognized key name (case-insensitive).
///
/// Returns true for key names like Enter, Tab, Ctrl+C, F1, etc.
/// Returns false for text that should be typed character by character.
///
/// # Examples
///
/// ```
/// use agent_tui::common::key_names::is_key_name;
///
/// assert!(is_key_name("Enter"));
/// assert!(is_key_name("Ctrl+C"));
/// assert!(!is_key_name("hello"));
/// ```
pub fn is_key_name(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();

    // Check for modifier combinations first (e.g., Ctrl+C, Alt+F4)
    if lower.contains('+') {
        if let Some(modifier) = lower.split('+').next() {
            return MODIFIER_PREFIXES.contains(&modifier);
        }
    }

    // Single key names (case-insensitive)
    SINGLE_KEY_NAMES.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_key_name_case_insensitive() {
        // Standard casing
        assert!(is_key_name("Enter"));
        assert!(is_key_name("Tab"));
        assert!(is_key_name("Escape"));
        assert!(is_key_name("F1"));

        // Lowercase
        assert!(is_key_name("enter"));
        assert!(is_key_name("tab"));
        assert!(is_key_name("escape"));
        assert!(is_key_name("f1"));

        // Uppercase
        assert!(is_key_name("ENTER"));
        assert!(is_key_name("TAB"));
        assert!(is_key_name("ESCAPE"));
        assert!(is_key_name("F1"));

        // Mixed case
        assert!(is_key_name("EnTeR"));
        assert!(is_key_name("ArrowUp"));
        assert!(is_key_name("arrowup"));
        assert!(is_key_name("ARROWUP"));
    }

    #[test]
    fn test_is_key_name_modifier_combos_case_insensitive() {
        // Standard casing
        assert!(is_key_name("Ctrl+c"));
        assert!(is_key_name("Alt+F4"));
        assert!(is_key_name("Shift+Tab"));

        // Lowercase modifiers
        assert!(is_key_name("ctrl+c"));
        assert!(is_key_name("alt+f4"));
        assert!(is_key_name("shift+tab"));

        // Uppercase modifiers
        assert!(is_key_name("CTRL+C"));
        assert!(is_key_name("ALT+F4"));
        assert!(is_key_name("SHIFT+TAB"));
    }

    #[test]
    fn test_is_key_name_text_not_detected() {
        // Regular text should not be detected as key names
        assert!(!is_key_name("hello"));
        assert!(!is_key_name("Hello World"));
        assert!(!is_key_name("test123"));
        assert!(!is_key_name("a")); // Single character is text, not a key
    }

    #[test]
    fn test_is_key_name_edge_cases() {
        // Empty string is not a key name
        assert!(!is_key_name(""));

        // Modifier with no key after '+' is still treated as a key combo
        // (the daemon will reject it, but is_key_name just checks the pattern)
        assert!(is_key_name("Ctrl+"));
        assert!(is_key_name("Alt+"));

        // Invalid modifier prefix is not a key name
        assert!(!is_key_name("Invalid+X"));
        assert!(!is_key_name("foo+bar"));

        // Unicode text is not a key name
        assert!(!is_key_name("こんにちは"));
        assert!(!is_key_name("你好"));
        assert!(!is_key_name("🎉"));

        // Whitespace is not a key name
        assert!(!is_key_name(" "));
        assert!(!is_key_name("  "));
        assert!(!is_key_name("\t"));

        // Numbers alone are not key names
        assert!(!is_key_name("1"));
        assert!(!is_key_name("123"));

        // Key-like strings with extra characters are not key names
        assert!(!is_key_name("Enter1"));
        assert!(!is_key_name("Tab!"));
    }
}
