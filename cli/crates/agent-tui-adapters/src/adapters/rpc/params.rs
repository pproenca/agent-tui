//! RPC parameter types.

use serde::Deserialize;
use serde::Serialize;

use crate::domain::session_types::TerminalSize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnParams {
    #[serde(default)]
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TerminalSize>,
}

fn default_spawn_size() -> TerminalSize {
    TerminalSize::default()
}

impl Default for SpawnParams {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            session: None,
            size: Some(default_spawn_size()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default)]
    pub strip_ansi: bool,
    #[serde(default)]
    pub retain_ansi: bool,
    #[serde(default)]
    pub include_cursor: bool,
    #[serde(default)]
    pub include_render: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyParams {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParams {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

fn default_timeout_ms() -> u64 {
    30000
}

impl Default for WaitParams {
    fn default() -> Self {
        Self {
            session: None,
            text: None,
            timeout_ms: default_timeout_ms(),
            condition: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeParams {
    #[serde(flatten)]
    pub size: TerminalSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyWriteParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    pub data: String,
}

#[cfg(test)]
mod tests {
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
            session: Some("session-1".to_string()),
            size: Some(TerminalSize::try_new(120, 40).expect("valid terminal size")),
        };

        let json = serde_json::to_value(&params).expect("spawn params should serialize");
        assert_eq!(json["cols"], 120);
        assert_eq!(json["rows"], 40);
        assert_eq!(json["command"], "bash");
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
}
