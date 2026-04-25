//! RPC parameter types.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::domain::session_types::TerminalSize;
use crate::domain::session_types::TerminalSizeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SpawnParamsWire", into = "SpawnParamsWire")]
pub struct SpawnParams {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub session: Option<String>,
    pub size: TerminalSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpawnParamsWire {
    #[serde(default)]
    command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<String>,
    #[serde(default = "default_cols")]
    cols: u16,
    #[serde(default = "default_rows")]
    rows: u16,
}

fn default_spawn_size() -> TerminalSize {
    TerminalSize::default()
}

fn default_cols() -> u16 {
    TerminalSize::default().cols()
}

fn default_rows() -> u16 {
    TerminalSize::default().rows()
}

impl Default for SpawnParams {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            env: None,
            session: None,
            size: default_spawn_size(),
        }
    }
}

impl TryFrom<SpawnParamsWire> for SpawnParams {
    type Error = TerminalSizeError;

    fn try_from(value: SpawnParamsWire) -> Result<Self, Self::Error> {
        Ok(Self {
            command: value.command,
            args: value.args,
            cwd: value.cwd,
            env: value.env,
            session: value.session,
            size: TerminalSize::try_new(value.cols, value.rows)?,
        })
    }
}

impl From<SpawnParams> for SpawnParamsWire {
    fn from(value: SpawnParams) -> Self {
        Self {
            command: value.command,
            args: value.args,
            cwd: value.cwd,
            env: value.env,
            session: value.session,
            cols: value.size.cols(),
            rows: value.size.rows(),
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
pub struct MouseParams {
    pub col: u16,
    pub row: u16,
    #[serde(default = "default_mouse_button")]
    pub button: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

fn default_mouse_button() -> String {
    "left".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseMoveParams {
    pub col: u16,
    pub row: u16,
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
#[path = "params_tests.rs"]
mod tests;
