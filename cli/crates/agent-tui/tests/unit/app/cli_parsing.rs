use agent_tui::app::commands::{
    ActionOperation, Cli, Commands, OutputFormat, ScrollDirection, ToggleState,
};
use clap::Parser;
use clap::error::ErrorKind;

#[test]
fn action_requires_ref() {
    let err = Cli::try_parse_from(["agent-tui", "action"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn action_defaults_to_click_when_only_ref_provided() {
    let cli = Cli::parse_from(["agent-tui", "action", "@btn1"]);
    let Commands::Action {
        element_ref,
        operation,
    } = cli.command
    else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    assert_eq!(element_ref, "@btn1");
    assert!(operation.is_none());
}

#[test]
fn action_click_accepts_element_ref() {
    let cli = Cli::parse_from(["agent-tui", "action", "@btn1", "click"]);
    let Commands::Action { element_ref, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    assert_eq!(element_ref, "@btn1");
}

#[test]
fn action_fill_requires_value() {
    let err = Cli::try_parse_from(["agent-tui", "action", "@inp1", "fill"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn action_fill_accepts_value() {
    let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "fill", "test value"]);
    let Commands::Action {
        element_ref,
        operation,
    } = cli.command
    else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    assert_eq!(element_ref, "@inp1");
    let Some(ActionOperation::Fill { value }) = operation else {
        panic!("Expected Fill operation, got {:?}", operation);
    };
    assert_eq!(value, "test value");
}

#[test]
fn action_fill_accepts_empty_value() {
    let cli = Cli::parse_from(["agent-tui", "action", "@inp1", "fill", ""]);
    let Commands::Action { operation, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    let Some(ActionOperation::Fill { value }) = operation else {
        panic!("Expected Fill operation, got {:?}", operation);
    };
    assert!(value.is_empty());
}

#[test]
fn action_scroll_requires_direction() {
    let err = Cli::try_parse_from(["agent-tui", "action", "@e1", "scroll"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn action_scroll_accepts_valid_directions() {
    for direction in ["up", "down", "left", "right"] {
        let cli = Cli::parse_from(["agent-tui", "action", "@e1", "scroll", direction]);
        let Commands::Action { operation, .. } = cli.command else {
            panic!("Expected Action command, got {:?}", cli.command);
        };
        let Some(ActionOperation::Scroll {
            direction: parsed, ..
        }) = operation
        else {
            panic!("Expected Scroll operation, got {:?}", operation);
        };
        match direction {
            "up" => assert!(matches!(parsed, ScrollDirection::Up)),
            "down" => assert!(matches!(parsed, ScrollDirection::Down)),
            "left" => assert!(matches!(parsed, ScrollDirection::Left)),
            "right" => assert!(matches!(parsed, ScrollDirection::Right)),
            _ => unreachable!("direction validated above"),
        }
    }
}

#[test]
fn action_scroll_accepts_amount() {
    let cli = Cli::parse_from(["agent-tui", "action", "@e1", "scroll", "down", "10"]);
    let Commands::Action { operation, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    let Some(ActionOperation::Scroll { amount, .. }) = operation else {
        panic!("Expected Scroll operation, got {:?}", operation);
    };
    assert_eq!(amount, 10);
}

#[test]
fn action_select_requires_option() {
    let err = Cli::try_parse_from(["agent-tui", "action", "@sel1", "select"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn action_select_accepts_options() {
    let cli = Cli::parse_from(["agent-tui", "action", "@sel1", "select", "Option 1"]);
    let Commands::Action { operation, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    let Some(ActionOperation::Select { options }) = operation else {
        panic!("Expected Select operation, got {:?}", operation);
    };
    assert_eq!(options, vec!["Option 1".to_string()]);
}

#[test]
fn action_select_accepts_multiple_options() {
    let cli = Cli::parse_from([
        "agent-tui",
        "action",
        "@sel1",
        "select",
        "Option 1",
        "Option 2",
    ]);
    let Commands::Action { operation, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    let Some(ActionOperation::Select { options }) = operation else {
        panic!("Expected Select operation, got {:?}", operation);
    };
    assert_eq!(
        options,
        vec!["Option 1".to_string(), "Option 2".to_string()]
    );
}

#[test]
fn action_toggle_accepts_state() {
    let cli = Cli::parse_from(["agent-tui", "action", "@cb1", "toggle", "on"]);
    let Commands::Action { operation, .. } = cli.command else {
        panic!("Expected Action command, got {:?}", cli.command);
    };
    let Some(ActionOperation::Toggle { state }) = operation else {
        panic!("Expected Toggle operation, got {:?}", operation);
    };
    assert!(matches!(state, Some(ToggleState::On)));
}

#[test]
fn input_requires_value_or_modifier() {
    let err = Cli::try_parse_from(["agent-tui", "input"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn input_accepts_valid_keys() {
    for key in ["Enter", "Tab", "Escape", "ArrowUp", "F1", "Ctrl+c"] {
        let cli = Cli::parse_from(["agent-tui", "input", key]);
        let Commands::Input { .. } = cli.command else {
            panic!("Expected Input command, got {:?}", cli.command);
        };
    }
}

#[test]
fn input_accepts_text() {
    let cli = Cli::parse_from(["agent-tui", "input", "Hello World"]);
    let Commands::Input { value, .. } = cli.command else {
        panic!("Expected Input command, got {:?}", cli.command);
    };
    assert_eq!(value, "Hello World");
}

#[test]
fn input_rejects_hold_and_release_together() {
    let err = Cli::try_parse_from(["agent-tui", "input", "Shift", "--hold", "--release"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn wait_requires_a_condition() {
    let err = Cli::try_parse_from(["agent-tui", "wait"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn wait_accepts_stable_condition() {
    let cli = Cli::parse_from(["agent-tui", "wait", "--stable"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert!(params.stable);
}

#[test]
fn wait_accepts_timeout() {
    let cli = Cli::parse_from(["agent-tui", "wait", "-t", "5000", "--stable"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.timeout, 5000);
}

#[test]
fn wait_accepts_element_ref() {
    let cli = Cli::parse_from(["agent-tui", "wait", "-e", "@btn1"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.element, Some("@btn1".to_string()));
}

#[test]
fn run_requires_command() {
    let err = Cli::try_parse_from(["agent-tui", "run"])
        .err()
        .expect("expected parse error");
    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn run_accepts_size_options() {
    let cli = Cli::parse_from(["agent-tui", "run", "--cols", "80", "--rows", "24", "bash"]);
    let Commands::Run { cols, rows, .. } = cli.command else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(cols, 80);
    assert_eq!(rows, 24);
}

#[test]
fn run_accepts_cwd_option() {
    let cli = Cli::parse_from(["agent-tui", "run", "-d", "/tmp", "bash"]);
    let Commands::Run { cwd, .. } = cli.command else {
        panic!("Expected Run command, got {:?}", cli.command);
    };
    assert_eq!(cwd.unwrap().to_string_lossy(), "/tmp");
}

#[test]
fn sessions_all_flag_is_rejected() {
    let err = Cli::try_parse_from(["agent-tui", "sessions", "--all"])
        .err()
        .expect("expected parse error");
    assert!(matches!(
        err.kind(),
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand
    ));
}

#[test]
fn format_json_option_is_parsed() {
    let cli = Cli::parse_from(["agent-tui", "-f", "json", "sessions"]);
    assert_eq!(cli.format, OutputFormat::Json);
}

#[test]
fn format_text_option_is_parsed() {
    let cli = Cli::parse_from(["agent-tui", "-f", "text", "sessions"]);
    assert_eq!(cli.format, OutputFormat::Text);
}

#[test]
fn session_option_is_parsed() {
    let cli = Cli::parse_from(["agent-tui", "-s", "my-session", "screenshot"]);
    assert_eq!(cli.session, Some("my-session".to_string()));
}

#[test]
fn wait_params_round_trip() {
    let cli = Cli::parse_from(["agent-tui", "wait", "Continue"]);
    let Commands::Wait { params } = cli.command else {
        panic!("Expected Wait command, got {:?}", cli.command);
    };
    assert_eq!(params.text, Some("Continue".to_string()));
    assert_eq!(params.timeout, 30_000);
}
