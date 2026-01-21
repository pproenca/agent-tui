//! Inquirer.js framework detector
//!
//! Detects elements specific to Inquirer.js applications using patterns like:
//! - Radio buttons: ◯, ◉ at start of lines
//! - Checkboxes: ◻, ◼
//! - Confirm prompts: (Y/n), (y/N)

use crate::detection::pattern::{deduplicate_matches, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};
use crate::detection::ElementType;
use regex::Regex;
use std::sync::OnceLock;

/// Inquirer.js framework detector
pub struct InquirerDetector;

impl InquirerDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if the screen appears to be an Inquirer application
    pub fn looks_like_inquirer(ctx: &DetectionContext) -> bool {
        // Inquirer has ◯ and ◉ on separate lines (one per option)
        let circle_lines = ctx
            .lines
            .iter()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with('◯') || trimmed.starts_with('◉')
            })
            .count();
        let has_inquirer_select = circle_lines >= 2;

        let has_inquirer_checkbox = ctx.screen_text.contains('◻') || ctx.screen_text.contains('◼');

        let has_inquirer_confirm =
            ctx.screen_text.contains("(Y/n)") || ctx.screen_text.contains("(y/N)");

        has_inquirer_select || has_inquirer_checkbox || has_inquirer_confirm
    }
}

impl Default for InquirerDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached regex patterns for Inquirer-specific elements
struct InquirerPatterns {
    radio: Regex,
    checkbox: Regex,
    confirm: Regex,
}

fn get_inquirer_patterns() -> &'static InquirerPatterns {
    static PATTERNS: OnceLock<InquirerPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| InquirerPatterns {
        // Inquirer radio: ◯ or ◉ at start followed by text
        radio: Regex::new(r"^\s*([◯◉])\s+(.+?)$").unwrap(),
        // Inquirer checkbox: ◻ or ◼ at start followed by text
        checkbox: Regex::new(r"^\s*([◻◼])\s+(.+?)$").unwrap(),
        // Inquirer confirm: (Y/n) or (y/N)
        confirm: Regex::new(r"\(([Yy])/([Nn])\)").unwrap(),
    })
}

impl ElementDetectorImpl for InquirerDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        let patterns = get_inquirer_patterns();

        for (row_idx, line) in ctx.lines.iter().enumerate() {
            let row = row_idx as u16;

            // Detect Inquirer radio buttons (◉ selected, ◯ unselected)
            if let Some(cap) = patterns.radio.captures(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_selected = marker == "◉";

                matches.push(PatternMatch {
                    element_type: ElementType::Radio,
                    label,
                    value: Some(
                        if is_selected {
                            "selected"
                        } else {
                            "unselected"
                        }
                        .to_string(),
                    ),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: Some(is_selected),
                });
            }

            // Detect Inquirer checkboxes (◼ checked, ◻ unchecked)
            if let Some(cap) = patterns.checkbox.captures(line) {
                let full_match = cap.get(0).unwrap();
                let marker = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let label = cap.get(2).map(|m| m.as_str().trim().to_string());

                let is_checked = marker == "◼";

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

            // Detect Inquirer confirm prompts
            for cap in patterns.confirm.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                // Uppercase letter indicates default
                let yes_cap = cap.get(1).map(|m| m.as_str()).unwrap_or("y");
                let default_yes = yes_cap == "Y";

                matches.push(PatternMatch {
                    element_type: ElementType::Button,
                    label: Some("Confirm".to_string()),
                    value: Some(
                        if default_yes {
                            "default:yes"
                        } else {
                            "default:no"
                        }
                        .to_string(),
                    ),
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
        "inquirer"
    }

    fn priority(&self) -> i32 {
        15 // Higher than Ink (more specific patterns)
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        Self::looks_like_inquirer(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inquirer_detector_framework_name() {
        let detector = InquirerDetector::new();
        assert_eq!(detector.framework_name(), "inquirer");
    }

    #[test]
    fn test_looks_like_inquirer_with_radios() {
        let ctx = DetectionContext::new("? Choose:\n  ◯ Option 1\n  ◉ Option 2", None);
        assert!(InquirerDetector::looks_like_inquirer(&ctx));
    }

    #[test]
    fn test_looks_like_inquirer_with_confirm() {
        let ctx = DetectionContext::new("? Continue? (Y/n)", None);
        assert!(InquirerDetector::looks_like_inquirer(&ctx));
    }

    #[test]
    fn test_does_not_look_like_inquirer() {
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        assert!(!InquirerDetector::looks_like_inquirer(&ctx));
    }

    #[test]
    fn test_detect_inquirer_radios() {
        let detector = InquirerDetector::new();
        let ctx = DetectionContext::new("  ◯ Option 1\n  ◉ Option 2", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|m| m.element_type == ElementType::Radio));
        assert!(matches.iter().any(|m| m.checked == Some(false)));
        assert!(matches.iter().any(|m| m.checked == Some(true)));
    }

    #[test]
    fn test_detect_inquirer_confirm() {
        let detector = InquirerDetector::new();
        let ctx = DetectionContext::new("Continue? (Y/n)", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Button);
        assert_eq!(matches[0].value, Some("default:yes".to_string()));
    }
}
