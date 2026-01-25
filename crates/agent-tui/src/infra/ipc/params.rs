use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnParams {
    #[serde(default)]
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

fn default_cols() -> u16 {
    80
}
fn default_rows() -> u16 {
    24
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub include_elements: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default)]
    pub strip_ansi: bool,
    #[serde(default)]
    pub include_cursor: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessibilitySnapshotParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRefParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaitParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

fn default_timeout_ms() -> u64 {
    30000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
    #[serde(default)]
    pub exact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeParams {
    pub cols: u16,
    pub rows: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LivePreviewStartParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,
    #[serde(default)]
    pub allow_remote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub option: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiselectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollParams {
    pub direction: String,
    #[serde(default = "default_scroll_amount")]
    pub amount: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

fn default_scroll_amount() -> u16 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CountParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordStopParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub start: bool,
    #[serde(default)]
    pub stop: bool,
    #[serde(default = "default_trace_count")]
    pub count: usize,
}

fn default_trace_count() -> usize {
    100
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsoleParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default = "default_console_count")]
    pub count: usize,
    #[serde(default)]
    pub clear: bool,
}

fn default_console_count() -> usize {
    50
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default = "default_errors_count")]
    pub count: usize,
    #[serde(default)]
    pub clear: bool,
}

fn default_errors_count() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyReadParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

fn default_max_bytes() -> usize {
    4096
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

    #[test]
    fn test_snapshot_params_serialization() {
        let params = SnapshotParams {
            session: Some("test-session".to_string()),
            include_elements: true,
            include_cursor: true,
            ..Default::default()
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["session"], "test-session");
        assert_eq!(json["include_elements"], true);
        assert_eq!(json["include_cursor"], true);
    }

    #[test]
    fn test_snapshot_params_deserialization() {
        let json = serde_json::json!({
            "session": "abc123",
            "include_elements": true,
            "include_cursor": true
        });

        let params: SnapshotParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.session, Some("abc123".to_string()));
        assert!(params.include_elements);
        assert!(params.include_cursor);
    }

    #[test]
    fn test_snapshot_params_defaults() {
        let json = serde_json::json!({});
        let params: SnapshotParams = serde_json::from_value(json).unwrap();

        assert_eq!(params.session, None);
        assert!(!params.include_elements);
        assert!(!params.include_cursor);
        assert!(!params.strip_ansi);
        assert_eq!(params.region, None);
    }

    #[test]
    fn test_element_ref_params_rename() {
        let params = ElementRefParams {
            element_ref: "@btn1".to_string(),
            session: None,
        };

        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["ref"], "@btn1");
        assert!(json.get("element_ref").is_none());
    }

    #[test]
    fn test_element_ref_params_deserialization() {
        let json = serde_json::json!({
            "ref": "@inp1",
            "session": "sess-1"
        });

        let params: ElementRefParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.element_ref, "@inp1");
        assert_eq!(params.session, Some("sess-1".to_string()));
    }

    #[test]
    fn test_spawn_params_defaults() {
        let json = serde_json::json!({});
        let params: SpawnParams = serde_json::from_value(json).unwrap();

        assert_eq!(params.command, "");
        assert!(params.args.is_empty());
        assert_eq!(params.cols, 80);
        assert_eq!(params.rows, 24);
    }

    #[test]
    fn test_wait_params_defaults() {
        let json = serde_json::json!({});
        let params: WaitParams = serde_json::from_value(json).unwrap();

        assert_eq!(params.timeout_ms, 30000);
        assert_eq!(params.text, None);
        assert_eq!(params.condition, None);
    }
}
