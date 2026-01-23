use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::session::Session;

#[derive(Debug, Clone)]
pub enum WaitCondition {
    Text(String),
    Element(String),
    Focused(String),
    NotVisible(String),
    Stable,
    TextGone(String),
    Value { element: String, expected: String },
}

impl WaitCondition {
    pub fn parse(
        condition: Option<&str>,
        target: Option<&str>,
        text: Option<&str>,
    ) -> Option<Self> {
        match condition {
            Some("text") => text.map(|t| WaitCondition::Text(t.to_string())),
            Some("element") => target.map(|t| WaitCondition::Element(t.to_string())),
            Some("focused") => target.map(|t| WaitCondition::Focused(t.to_string())),
            Some("not_visible") => target.map(|t| WaitCondition::NotVisible(t.to_string())),
            Some("stable") => Some(WaitCondition::Stable),
            Some("text_gone") => target.map(|t| WaitCondition::TextGone(t.to_string())),
            Some("value") => target.and_then(|t| {
                let parts: Vec<&str> = t.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some(WaitCondition::Value {
                        element: parts[0].to_string(),
                        expected: parts[1].to_string(),
                    })
                } else {
                    text.map(|expected_value| WaitCondition::Value {
                        element: t.to_string(),
                        expected: expected_value.to_string(),
                    })
                }
            }),
            None => text.map(|t| WaitCondition::Text(t.to_string())),
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match self {
            WaitCondition::Text(t) => format!("text \"{}\"", t),
            WaitCondition::Element(e) => format!("element {}", e),
            WaitCondition::Focused(e) => format!("{} to be focused", e),
            WaitCondition::NotVisible(e) => format!("{} to disappear", e),
            WaitCondition::Stable => "screen to stabilize".to_string(),
            WaitCondition::TextGone(t) => format!("text \"{}\" to disappear", t),
            WaitCondition::Value { element, expected } => {
                format!("{} to have value \"{}\"", element, expected)
            }
        }
    }

    pub fn matched_text(&self) -> Option<String> {
        match self {
            WaitCondition::Text(t) | WaitCondition::TextGone(t) => Some(t.clone()),
            WaitCondition::Value { expected, .. } => Some(expected.clone()),
            _ => None,
        }
    }

    pub fn element_ref(&self) -> Option<String> {
        match self {
            WaitCondition::Element(e)
            | WaitCondition::Focused(e)
            | WaitCondition::NotVisible(e) => Some(e.clone()),
            WaitCondition::Value { element, .. } => Some(element.clone()),
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

pub fn check_condition(
    session: &mut Session,
    condition: &WaitCondition,
    stable_tracker: &mut StableTracker,
) -> bool {
    let _ = session.update();

    match condition {
        WaitCondition::Text(text) => {
            let screen = session.screen_text();
            screen.contains(text)
        }

        WaitCondition::Element(element_ref) => {
            session.detect_elements();
            session.find_element(element_ref).is_some()
        }

        WaitCondition::Focused(element_ref) => {
            session.detect_elements();
            session
                .find_element(element_ref)
                .map(|el| el.focused)
                .unwrap_or(false)
        }

        WaitCondition::NotVisible(element_ref) => {
            session.detect_elements();
            session.find_element(element_ref).is_none()
        }

        WaitCondition::Stable => {
            let screen = session.screen_text();
            stable_tracker.add_hash(&screen)
        }

        WaitCondition::TextGone(text) => {
            let screen = session.screen_text();
            !screen.contains(text)
        }

        WaitCondition::Value { element, expected } => {
            session.detect_elements();
            session
                .find_element(element)
                .and_then(|el| el.value.as_ref())
                .map(|v| v == expected)
                .unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_condition() {
        let cond = WaitCondition::parse(Some("text"), None, Some("hello"));
        assert!(matches!(cond, Some(WaitCondition::Text(t)) if t == "hello"));
    }

    #[test]
    fn test_parse_element_condition() {
        let cond = WaitCondition::parse(Some("element"), Some("@btn1"), None);
        assert!(matches!(cond, Some(WaitCondition::Element(e)) if e == "@btn1"));
    }

    #[test]
    fn test_parse_stable_condition() {
        let cond = WaitCondition::parse(Some("stable"), None, None);
        assert!(matches!(cond, Some(WaitCondition::Stable)));
    }

    #[test]
    fn test_parse_value_condition() {
        let cond = WaitCondition::parse(Some("value"), Some("@inp1=hello"), None);
        assert!(
            matches!(cond, Some(WaitCondition::Value { element, expected }) if element == "@inp1" && expected == "hello")
        );
    }

    #[test]
    fn test_stable_tracker() {
        let mut tracker = StableTracker::new(3);

        assert!(!tracker.add_hash("screen1"));
        assert!(!tracker.add_hash("screen2"));
        assert!(!tracker.add_hash("screen3"));

        assert!(!tracker.add_hash("stable"));
        assert!(!tracker.add_hash("stable"));
        assert!(tracker.add_hash("stable"));
    }

    #[test]
    fn test_default_to_text_condition() {
        let cond = WaitCondition::parse(None, None, Some("hello"));
        assert!(matches!(cond, Some(WaitCondition::Text(t)) if t == "hello"));
    }
}
