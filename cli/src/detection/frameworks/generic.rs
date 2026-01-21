//! Generic fallback detector
//!
//! This detector wraps the existing pattern-based detection logic,
//! providing a baseline that works for any framework.

use crate::detection::pattern::{detect_by_pattern, PatternMatch};
use crate::detection::traits::{DetectionContext, ElementDetectorImpl};

/// Generic detector that uses pattern matching for any framework
///
/// This wraps the existing `detect_by_pattern` function, serving as the
/// fallback when no framework-specific detector matches.
pub struct GenericDetector;

impl GenericDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenericDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementDetectorImpl for GenericDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        detect_by_pattern(ctx.screen_text)
    }

    fn framework_name(&self) -> &'static str {
        "generic"
    }

    fn priority(&self) -> i32 {
        -100 // Lowest priority - used as fallback
    }

    fn can_detect(&self, _ctx: &DetectionContext) -> bool {
        true // Always matches as fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_detector_detects_buttons() {
        let detector = GenericDetector::new();
        let ctx = DetectionContext::new("[Submit] [Cancel]", None);
        let matches = detector.detect_patterns(&ctx);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_generic_detector_framework_name() {
        let detector = GenericDetector::new();
        assert_eq!(detector.framework_name(), "generic");
    }

    #[test]
    fn test_generic_detector_lowest_priority() {
        let detector = GenericDetector::new();
        assert!(detector.priority() < 0);
    }
}
