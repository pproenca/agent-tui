use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::repository::SessionOps;

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

pub fn check_condition<S: SessionOps>(
    session: &mut S,
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
    use crate::test_support::MockSession;
    use agent_tui_core::{Element, ElementType, Position};

    fn make_element(ref_id: &str, focused: bool, value: Option<String>) -> Element {
        Element {
            element_ref: ref_id.to_string(),
            element_type: ElementType::Button,
            label: Some("Test".to_string()),
            value,
            position: Position {
                row: 0,
                col: 0,
                width: Some(10),
                height: Some(1),
            },
            focused,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

    // ========================================================================
    // check_condition() tests using MockSession
    // ========================================================================

    #[test]
    fn test_check_condition_text_found() {
        let mut session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Text("World".to_string()),
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_text_not_found() {
        let mut session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Text("Missing".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_element_exists() {
        let elements = vec![make_element("@btn1", false, None)];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Element("@btn1".to_string()),
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_element_not_exists() {
        let mut session = MockSession::new("test");
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Element("@missing".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_focused_true() {
        let elements = vec![make_element("@input1", true, None)];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Focused("@input1".to_string()),
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_focused_false() {
        let elements = vec![make_element("@input1", false, None)];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Focused("@input1".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_focused_element_missing() {
        let mut session = MockSession::new("test");
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Focused("@missing".to_string()),
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_not_visible_when_missing() {
        let mut session = MockSession::new("test");
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::NotVisible("@modal".to_string()),
            &mut tracker,
        );

        assert!(result); // Element is not visible because it doesn't exist
    }

    #[test]
    fn test_check_condition_not_visible_when_present() {
        let elements = vec![make_element("@modal", false, None)];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::NotVisible("@modal".to_string()),
            &mut tracker,
        );

        assert!(!result); // Element IS visible, so not_visible is false
    }

    #[test]
    fn test_check_condition_text_gone_when_absent() {
        let mut session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::TextGone("Loading".to_string()),
            &mut tracker,
        );

        assert!(result); // "Loading" is not in screen, so text_gone is true
    }

    #[test]
    fn test_check_condition_text_gone_when_present() {
        let mut session = MockSession::builder("test")
            .with_screen_text("Loading...")
            .build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::TextGone("Loading".to_string()),
            &mut tracker,
        );

        assert!(!result); // "Loading" is still in screen
    }

    #[test]
    fn test_check_condition_value_matches() {
        let elements = vec![make_element("@input", false, Some("hello".to_string()))];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Value {
                element: "@input".to_string(),
                expected: "hello".to_string(),
            },
            &mut tracker,
        );

        assert!(result);
    }

    #[test]
    fn test_check_condition_value_does_not_match() {
        let elements = vec![make_element("@input", false, Some("world".to_string()))];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Value {
                element: "@input".to_string(),
                expected: "hello".to_string(),
            },
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_value_element_missing() {
        let mut session = MockSession::new("test");
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Value {
                element: "@missing".to_string(),
                expected: "hello".to_string(),
            },
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_value_element_has_no_value() {
        let elements = vec![make_element("@input", false, None)];
        let mut session = MockSession::builder("test").with_elements(elements).build();
        let mut tracker = StableTracker::new(3);

        let result = check_condition(
            &mut session,
            &WaitCondition::Value {
                element: "@input".to_string(),
                expected: "hello".to_string(),
            },
            &mut tracker,
        );

        assert!(!result);
    }

    #[test]
    fn test_check_condition_stable_requires_multiple_same_hashes() {
        let mut session = MockSession::builder("test")
            .with_screen_text("stable screen")
            .build();
        let mut tracker = StableTracker::new(3);

        // First 2 checks build up history, return false
        assert!(!check_condition(
            &mut session,
            &WaitCondition::Stable,
            &mut tracker
        ));
        assert!(!check_condition(
            &mut session,
            &WaitCondition::Stable,
            &mut tracker
        ));

        // 3rd check completes the stable window, returns true
        assert!(check_condition(
            &mut session,
            &WaitCondition::Stable,
            &mut tracker
        ));
    }

    // ========================================================================
    // WaitCondition::parse() tests
    // ========================================================================

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
