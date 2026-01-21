//! Ink (React for CLI) framework detector
//!
//! Detects elements specific to Ink-based applications using patterns like:
//! - Braille spinners: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
//! - Select indicators: ❯, ›
//! - Checkbox circles: ◉, ◯

use crate::detection::pattern::{deduplicate_matches, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};
use crate::detection::ElementType;
use regex::Regex;
use std::sync::OnceLock;

/// Ink framework detector
///
/// Specializes in detecting Ink-specific UI patterns that differ from
/// generic terminal patterns.
pub struct InkDetector;

impl InkDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if the screen appears to be an Ink application
    pub fn looks_like_ink(ctx: &DetectionContext) -> bool {
        let braille_spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let has_braille = braille_spinners.iter().any(|s| ctx.screen_text.contains(s));

        let has_ink_select = ctx.lines.iter().any(|l| {
            let trimmed = l.trim();
            trimmed.starts_with('❯') || trimmed.starts_with('›')
        });

        let has_question_pattern = ctx.screen_text.contains('?') && ctx.screen_text.contains('❯');

        has_braille || has_ink_select || has_question_pattern
    }
}

impl Default for InkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached regex patterns for Ink-specific elements
struct InkPatterns {
    select_item: Regex,
    checkbox: Regex,
    braille_spinner: Regex,
}

fn get_ink_patterns() -> &'static InkPatterns {
    static PATTERNS: OnceLock<InkPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| InkPatterns {
        select_item: Regex::new(r"^\s*([❯›])\s+(.+?)$").unwrap(),

        checkbox: Regex::new(r"^\s*([◉◯])\s+(.+?)$").unwrap(),

        braille_spinner: Regex::new(r"[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]").unwrap(),
    })
}

impl ElementDetectorImpl for InkDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        let patterns = get_ink_patterns();

        for (row_idx, line) in ctx.lines.iter().enumerate() {
            let row = row_idx as u16;

            if let Some(cap) = patterns.select_item.captures(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_focused = marker == "❯";

                matches.push(PatternMatch {
                    element_type: ElementType::MenuItem,
                    label,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_focused),
                });
            }

            if let Some(cap) = patterns.checkbox.captures(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_checked = marker == "◉";

                matches.push(PatternMatch {
                    element_type: ElementType::Checkbox,
                    label,
                    value: Some(if is_checked { "checked" } else { "unchecked" }.to_string()),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_checked),
                });
            }

            for cap in patterns.braille_spinner.captures_iter(line) {
                let full_match = cap.get(0).unwrap();

                matches.push(PatternMatch {
                    element_type: ElementType::Spinner,
                    label: None,
                    value: None,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }
        }

        deduplicate_matches(matches)
    }

    fn framework_name(&self) -> &'static str {
        "ink"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        Self::looks_like_ink(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ink_detector_framework_name() {
        let detector = InkDetector::new();
        assert_eq!(detector.framework_name(), "ink");
    }

    #[test]
    fn test_ink_detector_priority() {
        let detector = InkDetector::new();
        assert!(detector.priority() > 0);
    }

    #[test]
    fn test_looks_like_ink_with_select() {
        let ctx = DetectionContext::new("? Select a color\n  ❯ Red\n    Blue\n    Green", None);
        assert!(InkDetector::looks_like_ink(&ctx));
    }

    #[test]
    fn test_looks_like_ink_with_braille() {
        let ctx = DetectionContext::new("⠋ Loading...", None);
        assert!(InkDetector::looks_like_ink(&ctx));
    }

    #[test]
    fn test_does_not_look_like_ink() {
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        assert!(!InkDetector::looks_like_ink(&ctx));
    }

    #[test]
    fn test_detect_ink_select_items() {
        let detector = InkDetector::new();
        let ctx = DetectionContext::new("  ❯ Red\n    Blue\n    Green", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::MenuItem);
        assert_eq!(matches[0].label, Some("Red".to_string()));
        assert_eq!(matches[0].checked, Some(true));
    }

    #[test]
    fn test_detect_ink_checkboxes() {
        let detector = InkDetector::new();
        let ctx = DetectionContext::new("  ◉ Selected\n  ◯ Not selected", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 2);
        assert!(matches
            .iter()
            .all(|m| m.element_type == ElementType::Checkbox));
        assert!(matches.iter().any(|m| m.checked == Some(true)));
        assert!(matches.iter().any(|m| m.checked == Some(false)));
    }

    #[test]
    fn test_detect_braille_spinner() {
        let detector = InkDetector::new();
        let ctx = DetectionContext::new("⠋ Processing...", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Spinner);
    }
}
