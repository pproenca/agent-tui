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

mod mouse_button_tests {
    use super::*;

    #[test]
    fn test_mouse_button_parse_left() {
        assert_eq!(MouseButton::parse("left"), Some(MouseButton::Left));
    }

    #[test]
    fn test_mouse_button_parse_right() {
        assert_eq!(MouseButton::parse("right"), Some(MouseButton::Right));
    }

    #[test]
    fn test_mouse_button_parse_middle() {
        assert_eq!(MouseButton::parse("middle"), Some(MouseButton::Middle));
    }

    #[test]
    fn test_mouse_button_parse_case_insensitive() {
        assert_eq!(MouseButton::parse("LEFT"), Some(MouseButton::Left));
        assert_eq!(MouseButton::parse("Right"), Some(MouseButton::Right));
    }

    #[test]
    fn test_mouse_button_parse_invalid() {
        assert_eq!(MouseButton::parse("invalid"), None);
    }

    #[test]
    fn test_mouse_button_as_str() {
        assert_eq!(MouseButton::Left.as_str(), "left");
        assert_eq!(MouseButton::Right.as_str(), "right");
        assert_eq!(MouseButton::Middle.as_str(), "middle");
    }
}

mod mouse_event_kind_tests {
    use super::*;

    #[test]
    fn test_mouse_event_kind_parse_down() {
        assert_eq!(MouseEventKind::parse("down"), Some(MouseEventKind::Down));
    }

    #[test]
    fn test_mouse_event_kind_parse_up() {
        assert_eq!(MouseEventKind::parse("up"), Some(MouseEventKind::Up));
    }

    #[test]
    fn test_mouse_event_kind_parse_drag() {
        assert_eq!(MouseEventKind::parse("drag"), Some(MouseEventKind::Drag));
    }

    #[test]
    fn test_mouse_event_kind_parse_moved() {
        assert_eq!(MouseEventKind::parse("moved"), Some(MouseEventKind::Moved));
    }

    #[test]
    fn test_mouse_event_kind_parse_invalid() {
        assert_eq!(MouseEventKind::parse("invalid"), None);
    }

    #[test]
    fn test_mouse_event_kind_as_str() {
        assert_eq!(MouseEventKind::Down.as_str(), "down");
        assert_eq!(MouseEventKind::Up.as_str(), "up");
        assert_eq!(MouseEventKind::Drag.as_str(), "drag");
        assert_eq!(MouseEventKind::Moved.as_str(), "moved");
    }
}

mod mouse_position_tests {
    use super::*;

    #[test]
    fn test_mouse_position_new() {
        let pos = MousePosition::new(5, 10);
        assert_eq!(pos.col, 5);
        assert_eq!(pos.row, 10);
    }
}
