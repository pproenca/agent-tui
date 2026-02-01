use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::usecases::ports::SessionOps;

#[derive(Debug, Clone)]
pub enum WaitCondition {
    Text(String),
    Stable,
    TextGone(String),
}

impl WaitCondition {
    pub fn parse(condition: Option<&str>, text: Option<&str>) -> Option<Self> {
        match condition {
            Some("text") => text.map(|t| WaitCondition::Text(t.to_string())),
            Some("stable") => Some(WaitCondition::Stable),
            Some("text_gone") => text.map(|t| WaitCondition::TextGone(t.to_string())),
            None => text.map(|t| WaitCondition::Text(t.to_string())),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct StableTracker {
    last_hashes: VecDeque<u64>,
    required_consecutive: usize,
}

impl StableTracker {
    pub fn new(required_consecutive: usize) -> Self {
        Self {
            last_hashes: VecDeque::new(),
            required_consecutive,
        }
    }

    pub fn add_hash(&mut self, screen: &str) -> bool {
        let mut hasher = DefaultHasher::new();
        screen.hash(&mut hasher);
        let hash = hasher.finish();

        self.last_hashes.push_back(hash);

        if self.last_hashes.len() > self.required_consecutive {
            self.last_hashes.pop_front();
        }

        if self.last_hashes.len() >= self.required_consecutive {
            let first = self.last_hashes[0];
            self.last_hashes.iter().all(|&h| h == first)
        } else {
            false
        }
    }
}

pub fn check_condition<S: SessionOps + ?Sized>(
    session: &S,
    condition: &WaitCondition,
    stable_tracker: &mut StableTracker,
) -> bool {
    let _ = session.update();

    match condition {
        WaitCondition::Text(text) => {
            let screen = session.screen_text();
            screen.contains(text)
        }

        WaitCondition::Stable => {
            let screen = session.screen_text();
            stable_tracker.add_hash(&screen)
        }

        WaitCondition::TextGone(text) => {
            let screen = session.screen_text();
            !screen.contains(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecases::ports::test_support::MockSession;

    #[test]
    fn test_check_condition_text_found() {
        let session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &session,
            &WaitCondition::Text("World".to_string()),
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_text_not_found() {
        let session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &session,
            &WaitCondition::Text("Missing".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_text_gone_when_absent() {
        let session = MockSession::builder("test")
            .with_screen_text("Ready")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &session,
            &WaitCondition::TextGone("Loading".to_string()),
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_text_gone_when_present() {
        let session = MockSession::builder("test")
            .with_screen_text("Loading")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &session,
            &WaitCondition::TextGone("Loading".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_stable_requires_multiple_same_hashes() {
        let session = MockSession::builder("test")
            .with_screen_text("first")
            .build();
        let mut tracker = StableTracker::new(3);

        assert!(!check_condition(
            &session,
            &WaitCondition::Stable,
            &mut tracker
        ));
        assert!(!check_condition(
            &session,
            &WaitCondition::Stable,
            &mut tracker
        ));
        assert!(check_condition(
            &session,
            &WaitCondition::Stable,
            &mut tracker
        ));
    }

    #[test]
    fn test_wait_condition_parse_text() {
        let cond = WaitCondition::parse(Some("text"), Some("hello"));
        assert!(matches!(cond, Some(WaitCondition::Text(t)) if t == "hello"));
    }

    #[test]
    fn test_wait_condition_parse_text_gone() {
        let cond = WaitCondition::parse(Some("text_gone"), Some("loading"));
        assert!(matches!(cond, Some(WaitCondition::TextGone(t)) if t == "loading"));
    }

    #[test]
    fn test_wait_condition_parse_stable() {
        let cond = WaitCondition::parse(Some("stable"), None);
        assert!(matches!(cond, Some(WaitCondition::Stable)));
    }

    #[test]
    fn test_wait_condition_parse_none_defaults_to_text() {
        let cond = WaitCondition::parse(None, Some("hello"));
        assert!(matches!(cond, Some(WaitCondition::Text(t)) if t == "hello"));
    }

    #[test]
    fn test_wait_condition_parse_invalid() {
        let cond = WaitCondition::parse(Some("unknown"), Some("hello"));
        assert!(cond.is_none());
    }
}
