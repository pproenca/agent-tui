//! Bubble Tea (Go) framework detector
//!
//! Detects elements specific to Bubble Tea/Charm applications using patterns like:
//! - Charm-style spinners: ⣾⣽⣻⢿⡿⣟⣯⣷
//! - Help bar: "q: quit", "ctrl+c", "esc: back"
//! - Text inputs with │ cursor

use crate::detection::pattern::{deduplicate_matches, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};
use crate::detection::ElementType;
use regex::Regex;
use std::sync::OnceLock;

/// Bubble Tea (Charm) framework detector
pub struct BubbleTeaDetector;

impl BubbleTeaDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if the screen appears to be a Bubble Tea application
    pub fn looks_like_bubbletea(ctx: &DetectionContext) -> bool {
        let charm_spinners = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
        let has_charm_spinner = charm_spinners.iter().any(|s| ctx.screen_text.contains(s));

        let has_help_bar = ctx.screen_text.contains("q: quit")
            || ctx.screen_text.contains("ctrl+c")
            || ctx.screen_text.contains("esc: back")
            || ctx.screen_text.contains("enter: select");

        let has_text_input =
            ctx.screen_text.contains('│') && ctx.lines.iter().any(|l| l.contains('>'));

        has_charm_spinner || (has_help_bar && has_text_input)
    }
}

impl Default for BubbleTeaDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached regex patterns for BubbleTea-specific elements
struct BubbleTeaPatterns {
    charm_spinner: Regex,
    help_item: Regex,
    text_input: Regex,
}

fn get_bubbletea_patterns() -> &'static BubbleTeaPatterns {
    static PATTERNS: OnceLock<BubbleTeaPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| BubbleTeaPatterns {
        charm_spinner: Regex::new(r"[⣾⣽⣻⢿⡿⣟⣯⣷]").unwrap(),

        help_item: Regex::new(r"([a-z]+(?:\+[a-z]+)?|esc|enter|tab|space):\s+([a-z]+)").unwrap(),

        text_input: Regex::new(r">\s*([^│]*?)│").unwrap(),
    })
}

impl ElementDetectorImpl for BubbleTeaDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        let patterns = get_bubbletea_patterns();

        for (row_idx, line) in ctx.lines.iter().enumerate() {
            let row = row_idx as u16;

            for cap in patterns.charm_spinner.captures_iter(line) {
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

            for cap in patterns.help_item.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let key = cap.get(1).map(|m| m.as_str().to_string());
                let action = cap.get(2).map(|m| m.as_str().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Button,
                    label: action,
                    value: key,
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }

            if let Some(cap) = patterns.text_input.captures(line) {
                let full_match = cap.get(0).unwrap();
                let value = cap
                    .get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|v| !v.is_empty());

                matches.push(PatternMatch {
                    element_type: ElementType::Input,
                    label: None,
                    value,
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
        "bubbletea"
    }

    fn priority(&self) -> i32 {
        8
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        Self::looks_like_bubbletea(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bubbletea_detector_framework_name() {
        let detector = BubbleTeaDetector::new();
        assert_eq!(detector.framework_name(), "bubbletea");
    }

    #[test]
    fn test_looks_like_bubbletea_with_spinner() {
        let ctx = DetectionContext::new("⣾ Loading...", None);
        assert!(BubbleTeaDetector::looks_like_bubbletea(&ctx));
    }

    #[test]
    fn test_looks_like_bubbletea_with_help() {
        let ctx = DetectionContext::new("My App\n\n> input│\n\nq: quit | enter: select", None);
        assert!(BubbleTeaDetector::looks_like_bubbletea(&ctx));
    }

    #[test]
    fn test_does_not_look_like_bubbletea() {
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        assert!(!BubbleTeaDetector::looks_like_bubbletea(&ctx));
    }

    #[test]
    fn test_detect_charm_spinner() {
        let detector = BubbleTeaDetector::new();
        let ctx = DetectionContext::new("⣾ Processing...", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Spinner);
    }

    #[test]
    fn test_detect_help_buttons() {
        let detector = BubbleTeaDetector::new();
        let ctx = DetectionContext::new("q: quit | enter: select", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 2);
        assert!(matches
            .iter()
            .all(|m| m.element_type == ElementType::Button));
    }
}
