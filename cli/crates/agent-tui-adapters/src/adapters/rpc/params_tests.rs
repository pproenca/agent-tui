use super::*;
use serde_json::json;

#[test]
fn test_snapshot_params_serialization() {
    let params = SnapshotParams {
        session: Some("abc".to_string()),
        region: None,
        strip_ansi: true,
        retain_ansi: false,
        include_cursor: false,
        include_render: true,
    };

    let json = serde_json::to_value(&params).expect("snapshot params should serialize");
    assert!(json.get("session").is_some());
    assert_eq!(json["strip_ansi"], true);
    assert_eq!(json["retain_ansi"], false);
    assert_eq!(json["include_cursor"], false);
    assert_eq!(json["include_render"], true);
}

#[test]
fn test_wait_params_defaults() {
    let params = WaitParams::default();
    assert_eq!(params.timeout_ms, 30000);
    assert!(params.text.is_none());
    assert!(params.condition.is_none());
}

#[test]
fn test_spawn_params_serialization_flattens_terminal_size() {
    let params = SpawnParams {
        command: "bash".to_string(),
        args: vec!["-lc".to_string(), "echo hello".to_string()],
        cwd: Some("/tmp".to_string()),
        env: Some(HashMap::from([("FOO".to_string(), "bar".to_string())])),
        session: Some("session-1".to_string()),
        size: TerminalSize::try_new(120, 40).expect("valid terminal size"),
    };

    let json = serde_json::to_value(&params).expect("spawn params should serialize");
    assert_eq!(json["cols"], 120);
    assert_eq!(json["rows"], 40);
    assert_eq!(json["command"], "bash");
    assert_eq!(json["env"]["FOO"], "bar");
}

#[test]
fn test_spawn_params_reject_invalid_terminal_size() {
    let err = serde_json::from_value::<SpawnParams>(json!({
        "command": "bash",
        "cols": 9,
        "rows": 24
    }))
    .expect_err("invalid terminal size should be rejected");

    assert!(err.to_string().contains("Columns (9) must be at least 10"));
}
