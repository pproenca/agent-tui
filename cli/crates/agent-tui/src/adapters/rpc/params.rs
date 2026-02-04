use serde::Deserialize;
use serde::Serialize;

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

impl Default for SpawnParams {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            session: None,
            cols: default_cols(),
            rows: default_rows(),
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
            session: Some("abc".to_string()),
            region: None,
            strip_ansi: true,
            include_cursor: false,
            include_render: true,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert!(json.get("session").is_some());
        assert_eq!(json.get("strip_ansi").unwrap(), true);
        assert_eq!(json.get("include_cursor").unwrap(), false);
        assert_eq!(json.get("include_render").unwrap(), true);
    }

    #[test]
    fn test_wait_params_defaults() {
        let params = WaitParams::default();
        assert_eq!(params.timeout_ms, 30000);
        assert!(params.text.is_none());
        assert!(params.condition.is_none());
    }
}
