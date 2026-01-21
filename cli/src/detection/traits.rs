use crate::terminal::ScreenBuffer;

use super::pattern::PatternMatch;

pub struct DetectionContext<'a> {
    pub screen_text: &'a str,
    pub lines: Vec<&'a str>,
}

impl<'a> DetectionContext<'a> {
    pub fn new(screen_text: &'a str, _screen_buffer: Option<&'a ScreenBuffer>) -> Self {
        Self {
            screen_text,
            lines: screen_text.lines().collect(),
        }
    }
}

#[allow(dead_code)]
pub trait ElementDetectorImpl: Send + Sync {
    fn detect_patterns(&self, ctx: &DetectionContext) -> Vec<PatternMatch>;

    fn framework_name(&self) -> &'static str;

    fn priority(&self) -> i32 {
        0
    }

    fn can_detect(&self, _ctx: &DetectionContext) -> bool {
        true
    }
}
