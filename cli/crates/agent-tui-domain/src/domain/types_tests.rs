use super::*;

mod wait_condition_type_tests {
    use super::*;

    #[test]
    fn test_wait_condition_type_from_str_text() {
        let cond = WaitConditionType::parse("text").expect("Should parse 'text'");
        assert_eq!(cond, WaitConditionType::Text);
    }

    #[test]
    fn test_wait_condition_type_from_str_stable() {
        let cond = WaitConditionType::parse("stable").expect("Should parse 'stable'");
        assert_eq!(cond, WaitConditionType::Stable);
    }

    #[test]
    fn test_wait_condition_type_from_str_text_gone() {
        let cond = WaitConditionType::parse("text_gone").expect("Should parse 'text_gone'");
        assert_eq!(cond, WaitConditionType::TextGone);
    }

    #[test]
    fn test_wait_condition_type_from_str_invalid() {
        let result = WaitConditionType::parse("invalid");
        assert!(result.is_err(), "Invalid condition should be rejected");
    }

    #[test]
    fn test_wait_condition_type_from_str_empty() {
        let result = WaitConditionType::parse("");
        assert!(result.is_err(), "Empty string should be rejected");
    }

    #[test]
    fn test_wait_condition_type_case_insensitive() {
        assert_eq!(
            WaitConditionType::parse("TEXT").expect("TEXT should parse"),
            WaitConditionType::Text
        );
        assert_eq!(
            WaitConditionType::parse("STABLE").expect("STABLE should parse"),
            WaitConditionType::Stable
        );
    }

    #[test]
    fn test_wait_condition_type_as_str() {
        assert_eq!(WaitConditionType::Text.as_str(), "text");
        assert_eq!(WaitConditionType::Stable.as_str(), "stable");
        assert_eq!(WaitConditionType::TextGone.as_str(), "text_gone");
    }

    #[test]
    fn test_wait_condition_type_display() {
        assert_eq!(format!("{}", WaitConditionType::Text), "text");
        assert_eq!(format!("{}", WaitConditionType::Stable), "stable");
    }

    #[test]
    fn test_wait_condition_type_requires_text() {
        assert!(WaitConditionType::Text.requires_text());
        assert!(!WaitConditionType::Stable.requires_text());
        assert!(WaitConditionType::TextGone.requires_text());
    }

    #[test]
    fn test_wait_condition_type_error_message() {
        let err = WaitConditionType::parse("invalid")
            .expect_err("invalid wait condition should be rejected");
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("text"));
    }
}

mod assert_condition_type_tests {
    use super::*;

    #[test]
    fn test_assert_condition_type_parse_text() {
        assert_eq!(
            AssertConditionType::parse("text").expect("text assert condition should parse"),
            AssertConditionType::Text
        );
    }

    #[test]
    fn test_assert_condition_type_parse_session() {
        assert_eq!(
            AssertConditionType::parse("session").expect("session assert condition should parse"),
            AssertConditionType::Session
        );
    }

    #[test]
    fn test_assert_condition_type_parse_invalid() {
        assert!(AssertConditionType::parse("invalid").is_err());
    }
}
