use super::*;

#[test]
fn test_text_presenter_success() {
    let presenter = Presenter::Text;

    presenter.present_success("Test message", None);
    presenter.present_success("Test with warning", Some("Warning text"));
}

#[test]
fn test_json_presenter_success() {
    let presenter = Presenter::Json;

    presenter.present_success("Test message", None);
    presenter.present_success("Test with warning", Some("Warning text"));
}

#[test]
fn test_text_presenter_error() {
    let presenter = Presenter::Text;
    presenter.present_error("Test error");
}

#[test]
fn test_json_presenter_error() {
    let presenter = Presenter::Json;
    presenter.present_error("Test error");
}

#[test]
fn test_spawn_result_to_json() {
    let result = SpawnResult {
        session_id: "abc123".to_string(),
        pid: 1234,
    };
    let json = result.to_json();
    assert_eq!(json.str_or("session_id", ""), "abc123");
    assert_eq!(json.u64_or("pid", 0), 1234);
}

#[test]
fn test_wait_result_struct() {
    let result = WaitResult {
        found: true,
        elapsed_ms: 150,
    };
    assert!(result.found);
    assert_eq!(result.elapsed_ms, 150);
}

#[test]
fn test_assert_result_struct() {
    let result = AssertResult {
        passed: true,
        condition: "text:hello".to_string(),
    };
    assert!(result.passed);
    assert_eq!(result.condition, "text:hello");
}

#[test]
fn test_cleanup_result_struct() {
    let result = CleanupResult {
        cleaned: 3,
        failures: vec![CleanupFailure {
            session_id: "sess1".to_string(),
            error: "session not found".to_string(),
        }],
    };
    assert_eq!(result.cleaned, 3);
    assert_eq!(result.failures.len(), 1);
}

#[test]
fn test_json_presenter_wait_result() {
    let presenter = Presenter::Json;
    let result = WaitResult {
        found: true,
        elapsed_ms: 100,
    };

    presenter.present_wait_result(&result);
}

#[test]
fn test_json_presenter_assert_result() {
    let presenter = Presenter::Json;
    let result = AssertResult {
        passed: true,
        condition: "text:hello".to_string(),
    };

    presenter.present_assert_result(&result);
}

#[test]
fn test_json_presenter_cleanup() {
    let presenter = Presenter::Json;
    let result = CleanupResult {
        cleaned: 2,
        failures: vec![],
    };

    presenter.present_cleanup(&result);
}

#[test]
fn output_format_selects_presenter_variant() {
    assert_eq!(Presenter::from(OutputFormat::Text), Presenter::Text);
    assert_eq!(Presenter::from(OutputFormat::Json), Presenter::Json);
}
