use crate::detection::pattern::{deduplicate_matches, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};
use crate::detection::ElementType;
use regex::Regex;
use std::sync::OnceLock;

pub struct RatatuiDetector;

impl RatatuiDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn looks_like_ratatui(ctx: &DetectionContext) -> bool {
        let block_chars = ['█', '▓', '▒', '░', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
        let block_count = block_chars
            .iter()
            .filter(|c| ctx.screen_text.contains(**c))
            .count();

        let sparkline_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇'];
        let sparkline_count = sparkline_chars
            .iter()
            .filter(|c| ctx.screen_text.contains(**c))
            .count();

        block_count >= 3 || sparkline_count >= 3
    }
}

impl Default for RatatuiDetector {
    fn default() -> Self {
        Self::new()
    }
}

struct RatatuiPatterns {
    progress_bar: Regex,
    sparkline: Regex,
    percentage: Regex,
}

fn get_ratatui_patterns() -> &'static RatatuiPatterns {
    static PATTERNS: OnceLock<RatatuiPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| RatatuiPatterns {
        progress_bar: Regex::new(r"[█▓▒░]{3,}").unwrap(),

        sparkline: Regex::new(r"[▁▂▃▄▅▆▇█]{3,}").unwrap(),

        percentage: Regex::new(r"(\d+)\s*%").unwrap(),
    })
}

impl ElementDetectorImpl for RatatuiDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        let patterns = get_ratatui_patterns();

        for (row_idx, line) in ctx.lines.iter().enumerate() {
            let row = row_idx as u16;

            for cap in patterns.progress_bar.captures_iter(line) {
                let full_match = cap.get(0).unwrap();

                matches.push(PatternMatch {
                    element_type: ElementType::Progress,
                    label: None,
                    value: Some(full_match.as_str().to_string()),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }

            for cap in patterns.sparkline.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let start = full_match.start() as u16;

                if matches
                    .iter()
                    .any(|m| m.row == row && m.col <= start && start < m.col + m.width)
                {
                    continue;
                }

                matches.push(PatternMatch {
                    element_type: ElementType::Progress,
                    label: Some("sparkline".to_string()),
                    value: Some(full_match.as_str().to_string()),
                    row,
                    col: full_match.start() as u16,
                    width: full_match.as_str().len() as u16,
                    checked: None,
                });
            }

            for cap in patterns.percentage.captures_iter(line) {
                let full_match = cap.get(0).unwrap();
                let value = cap.get(1).map(|m| m.as_str().to_string());

                matches.push(PatternMatch {
                    element_type: ElementType::Progress,
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
        "ratatui"
    }

    fn priority(&self) -> i32 {
        5
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        Self::looks_like_ratatui(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratatui_detector_framework_name() {
        let detector = RatatuiDetector::new();
        assert_eq!(detector.framework_name(), "ratatui");
    }

    #[test]
    fn test_looks_like_ratatui_with_blocks() {
        let ctx = DetectionContext::new("Progress: [████▓▓▒▒░░░░░░░░] 50%", None);
        assert!(RatatuiDetector::looks_like_ratatui(&ctx));
    }

    #[test]
    fn test_looks_like_ratatui_with_sparkline() {
        let ctx = DetectionContext::new("Usage: ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁", None);
        assert!(RatatuiDetector::looks_like_ratatui(&ctx));
    }

    #[test]
    fn test_does_not_look_like_ratatui() {
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        assert!(!RatatuiDetector::looks_like_ratatui(&ctx));
    }

    #[test]
    fn test_detect_progress_bar() {
        let detector = RatatuiDetector::new();
        let ctx = DetectionContext::new("████████░░░░", None);
        let matches = detector.detect_patterns(&ctx);

        assert!(!matches.is_empty());
        assert!(matches
            .iter()
            .any(|m| m.element_type == ElementType::Progress));
    }

    #[test]
    fn test_detect_percentage() {
        let detector = RatatuiDetector::new();
        let ctx = DetectionContext::new("Loading: 75%", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].element_type, ElementType::Progress);
        assert_eq!(matches[0].value, Some("75".to_string()));
    }
}
