use super::frameworks::{
    BubbleTeaDetector, GenericDetector, InkDetector, InquirerDetector, RatatuiDetector,
    TextualDetector,
};
use super::pattern::PatternMatch;
use super::traits::{DetectionContext, ElementDetectorImpl};

pub enum FrameworkDetector {
    Generic(GenericDetector),
    Ink(InkDetector),
    Inquirer(InquirerDetector),
    BubbleTea(BubbleTeaDetector),
    Textual(TextualDetector),
    Ratatui(RatatuiDetector),
}

impl FrameworkDetector {
    pub fn detect(ctx: &DetectionContext) -> Self {
        if InquirerDetector::looks_like_inquirer(ctx) {
            return FrameworkDetector::Inquirer(InquirerDetector::new());
        }

        if InkDetector::looks_like_ink(ctx) {
            return FrameworkDetector::Ink(InkDetector::new());
        }

        if BubbleTeaDetector::looks_like_bubbletea(ctx) {
            return FrameworkDetector::BubbleTea(BubbleTeaDetector::new());
        }

        if TextualDetector::looks_like_textual(ctx) {
            return FrameworkDetector::Textual(TextualDetector::new());
        }

        if RatatuiDetector::looks_like_ratatui(ctx) {
            return FrameworkDetector::Ratatui(RatatuiDetector::new());
        }

        FrameworkDetector::Generic(GenericDetector::new())
    }
}

impl Default for FrameworkDetector {
    fn default() -> Self {
        FrameworkDetector::Generic(GenericDetector::new())
    }
}

macro_rules! delegate_to_detector {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            FrameworkDetector::Generic(d) => d.$method($($arg),*),
            FrameworkDetector::Ink(d) => d.$method($($arg),*),
            FrameworkDetector::Inquirer(d) => d.$method($($arg),*),
            FrameworkDetector::BubbleTea(d) => d.$method($($arg),*),
            FrameworkDetector::Textual(d) => d.$method($($arg),*),
            FrameworkDetector::Ratatui(d) => d.$method($($arg),*),
        }
    };
}

impl ElementDetectorImpl for FrameworkDetector {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch> {
        delegate_to_detector!(self, detect_patterns, ctx)
    }

    fn framework_name(&self) -> &'static str {
        delegate_to_detector!(self, framework_name)
    }

    fn priority(&self) -> i32 {
        delegate_to_detector!(self, priority)
    }

    fn can_detect(&self, ctx: &DetectionContext) -> bool {
        delegate_to_detector!(self, can_detect, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_detector_default() {
        let detector = FrameworkDetector::default();
        assert_eq!(detector.framework_name(), "generic");
    }

    #[test]
    fn test_framework_detector_auto_detect_generic() {
        let ctx = DetectionContext::new("[Submit]", None);
        let detector = FrameworkDetector::detect(&ctx);
        assert_eq!(detector.framework_name(), "generic");
    }

    #[test]
    fn test_framework_detector_auto_detect_ink() {
        let ctx = DetectionContext::new("? Select:\n  ❯ Option 1\n    Option 2", None);
        let detector = FrameworkDetector::detect(&ctx);
        assert_eq!(detector.framework_name(), "ink");
    }

    #[test]
    fn test_framework_detector_auto_detect_inquirer() {
        let ctx = DetectionContext::new("? Choose:\n  ◯ Option 1\n  ◉ Option 2", None);
        let detector = FrameworkDetector::detect(&ctx);
        assert_eq!(detector.framework_name(), "inquirer");
    }

    #[test]
    fn test_framework_detector_priority() {
        let generic = FrameworkDetector::default();
        assert!(generic.priority() < 0);

        let ink = FrameworkDetector::Ink(InkDetector::new());
        assert!(ink.priority() > 0);

        let inquirer = FrameworkDetector::Inquirer(InquirerDetector::new());
        assert!(inquirer.priority() > ink.priority());
    }

    #[test]
    fn test_framework_detector_can_detect() {
        let ctx = DetectionContext::new("  ❯ Red\n    Blue", None);
        let ink = FrameworkDetector::Ink(InkDetector::new());
        assert!(ink.can_detect(&ctx));

        let generic = FrameworkDetector::default();
        assert!(generic.can_detect(&ctx));
    }
}
