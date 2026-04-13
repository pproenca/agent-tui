use super::*;
use crate::test_support::MockSession;
use crate::usecases::ports::SessionError;

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
fn test_check_condition_does_not_attempt_hidden_refresh() {
    let session = MockSession::builder("test")
        .with_screen_text("Ready")
        .with_update_error(SessionError::NoActiveSession)
        .build();
    let mut tracker = StableTracker::new(3);

    let result = check_condition(
        &session,
        &WaitCondition::Text("Ready".to_string()),
        &mut tracker,
    );

    assert!(result);
}

#[test]
fn test_wait_condition_parse_text() {
    let cond = WaitCondition::parse(Some(WaitConditionType::Text), Some("hello"))
        .expect("text condition should parse");
    assert!(matches!(cond, WaitCondition::Text(t) if t == "hello"));
}

#[test]
fn test_wait_condition_parse_text_gone() {
    let cond = WaitCondition::parse(Some(WaitConditionType::TextGone), Some("loading"))
        .expect("text_gone condition should parse");
    assert!(matches!(cond, WaitCondition::TextGone(t) if t == "loading"));
}

#[test]
fn test_wait_condition_parse_stable() {
    let cond = WaitCondition::parse(Some(WaitConditionType::Stable), None)
        .expect("stable condition should parse");
    assert!(matches!(cond, WaitCondition::Stable));
}

#[test]
fn test_wait_condition_parse_none_defaults_to_text() {
    let cond =
        WaitCondition::parse(None, Some("hello")).expect("text default condition should parse");
    assert!(matches!(cond, WaitCondition::Text(t) if t == "hello"));
}

#[test]
fn test_wait_condition_parse_none_none_defaults_to_stable() {
    let cond = WaitCondition::parse(None, None).expect("stable default condition should parse");
    assert!(matches!(cond, WaitCondition::Stable));
}

#[test]
fn test_wait_condition_parse_text_missing_text_returns_error() {
    let result = WaitCondition::parse(Some(WaitConditionType::Text), None);
    assert!(result.is_err());
}

#[test]
fn test_wait_condition_parse_text_gone_missing_text_returns_error() {
    let result = WaitCondition::parse(Some(WaitConditionType::TextGone), None);
    assert!(result.is_err());
}
