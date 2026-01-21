use crate::detection::pattern::{deduplicate_matches, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};
use crate::detection::ElementType;
use regex::Regex;
use std::sync::OnceLock;

pub struct TextualDetector;

impl TextualDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn looks_like_textual(ctx: &DetectionContext) -> bool {
        let has_textual_footer = ctx.lines.last().is_some_and(|l| {
            l.contains("^q") || l.contains("^c") || l.contains("F1") || l.contains("ESC")
        });

        let has_heavy_borders = ctx.screen_text.contains('┏')
            && ctx.screen_text.contains('┓')
            && ctx.screen_text.contains('┗');

        let has_data_table = ctx.screen_text.contains('│')
            && ctx.screen_text.contains('─')
            && ctx.screen_text.contains('┼');

        has_textual_footer || has_heavy_borders || has_data_table
    }
}

impl Default for TextualDetector {
    fn default() -> Self {
        Self::new()
    }
}

struct TextualPatterns {
    footer_key: Regex,
    button: Regex,
}

fn get_textual_patterns() -> &'static TextualPatterns {
    static PATTERNS: OnceLock<TextualPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| TextualPatterns {
        footer_key: Regex::new(r"(\^[a-z]|F\d+|ESC|TAB|ENTER)\s+([A-Za-z]+)").unwrap(),

        button: Regex::new(r"\[\s+([A-Za-z][A-Za-z\s]*?)\s+\]").unwrap(),
    })
}

impl ElementDetectorImpl for TextualDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        let patterns = get_textual_patterns();

        for (row_idx, line) in ctx.lines.iter().enumerate() {
            let row = row_idx as u16;

            for cap in patterns.footer_key.captures_iter(line) {
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

            for cap in patterns.button.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let label = cap.get(1).map(|m| m.as_str().trim().to_string());

                let start = full_match.start() as u16;
                if matches
                    .iter()
                    .any(|m| m.row == row && m.col <= start && start < m.col + m.width)
                {
                    continue;
                }

                matches.push(PatternMatch {
                    element_type: ElementType::Button,
                    label,
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
        "textual"
    }

    fn priority(&self) -> i32 {
        7
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        Self::looks_like_textual(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textual_detector_framework_name() {
        let detector = TextualDetector::new();
        assert_eq!(detector.framework_name(), "textual");
    }

    #[test]
    fn test_looks_like_textual_with_footer() {
        let ctx = DetectionContext::new("My App\n\n^q Quit  F1 Help", None);
        assert!(TextualDetector::looks_like_textual(&ctx));
    }

    #[test]
    fn test_looks_like_textual_with_heavy_borders() {
        let ctx = DetectionContext::new("┏━━━━━━━━┓\n┃ Hello  ┃\n┗━━━━━━━━┛", None);
        assert!(TextualDetector::looks_like_textual(&ctx));
    }

    #[test]
    fn test_does_not_look_like_textual() {
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        assert!(!TextualDetector::looks_like_textual(&ctx));
    }

    #[test]
    fn test_detect_footer_buttons() {
        let detector = TextualDetector::new();
        let ctx = DetectionContext::new("^q Quit  F1 Help  ESC Cancel", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 3);
        assert!(matches
            .iter()
            .all(|m| m.element_type == ElementType::Button));
    }
}
