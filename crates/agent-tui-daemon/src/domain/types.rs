use std::collections::HashMap;

use agent_tui_core::CursorPosition;
use agent_tui_core::Element;

use super::session_types::ErrorEntry;
use super::session_types::RecordingFrame;
use super::session_types::RecordingStatus;
use super::session_types::SessionId;
use super::session_types::SessionInfo;
use super::session_types::TraceEntry;

#[derive(Debug, Clone)]
pub struct SpawnInput {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub session_id: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct SpawnOutput {
    pub session_id: SessionId,
    pub pid: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotInput {
    pub session_id: Option<String>,
    pub include_elements: bool,
    pub region: Option<String>,
    pub strip_ansi: bool,
    pub include_cursor: bool,
}

#[derive(Debug, Clone)]
pub struct SnapshotOutput {
    pub session_id: SessionId,
    pub screen: String,
    pub elements: Option<Vec<Element>>,
    pub cursor: Option<CursorPosition>,
}

#[derive(Debug, Clone)]
pub struct ClickInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct ClickOutput {
    pub success: bool,
    pub message: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FillInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct FillOutput {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeystrokeInput {
    pub session_id: Option<String>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeystrokeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInput {
    pub session_id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TypeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeydownInput {
    pub session_id: Option<String>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeydownOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeyupInput {
    pub session_id: Option<String>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeyupOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct WaitInput {
    pub session_id: Option<String>,
    pub text: Option<String>,
    pub timeout_ms: u64,
    pub condition: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WaitOutput {
    pub found: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct FindInput {
    pub session_id: Option<String>,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub placeholder: Option<String>,
    pub focused: Option<bool>,
    pub nth: Option<usize>,
    pub exact: bool,
}

#[derive(Debug, Clone)]
pub struct FindOutput {
    pub elements: Vec<Element>,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct ScrollInput {
    pub session_id: Option<String>,
    pub direction: String,
    pub amount: u16,
}

#[derive(Debug, Clone)]
pub struct ScrollOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ResizeInput {
    pub session_id: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct ResizeOutput {
    pub session_id: SessionId,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SessionsOutput {
    pub sessions: Vec<SessionInfo>,
    pub active_session: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct KillOutput {
    pub session_id: SessionId,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ElementStateInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct VisibilityOutput {
    pub found: bool,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct FocusCheckOutput {
    pub found: bool,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct IsEnabledOutput {
    pub found: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct IsCheckedOutput {
    pub found: bool,
    pub checked: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetTextOutput {
    pub found: bool,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GetValueOutput {
    pub found: bool,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct GetFocusedOutput {
    pub found: bool,
    pub element: Option<Element>,
}

#[derive(Debug, Clone)]
pub struct DoubleClickInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct DoubleClickOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct FocusInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct FocusOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ClearInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct ClearOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SelectAllInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct SelectAllOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ToggleInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub state: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ToggleOutput {
    pub success: bool,
    pub checked: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SelectInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub option: String,
}

#[derive(Debug, Clone)]
pub struct SelectOutput {
    pub success: bool,
    pub selected_option: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MultiselectInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MultiselectOutput {
    pub success: bool,
    pub selected_options: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordStartInput {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordStopInput {
    pub session_id: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordStatusInput {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordStartOutput {
    pub session_id: SessionId,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct RecordStopOutput {
    pub session_id: SessionId,
    pub frame_count: usize,
    pub frames: Vec<RecordingFrame>,
    pub format: String,
    pub cols: u16,
    pub rows: u16,
}

pub type RecordStatusOutput = RecordingStatus;

#[derive(Debug, Clone)]
pub struct TraceInput {
    pub session_id: Option<String>,
    pub start: bool,
    pub stop: bool,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct TraceOutput {
    pub tracing: bool,
    pub entries: Vec<TraceEntry>,
}

#[derive(Debug, Clone)]
pub struct ConsoleInput {
    pub session_id: Option<String>,
    pub count: usize,
    pub clear: bool,
}

#[derive(Debug, Clone)]
pub struct ConsoleOutput {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ErrorsInput {
    pub session_id: Option<String>,
    pub count: usize,
    pub clear: bool,
}

#[derive(Debug, Clone)]
pub struct ErrorsOutput {
    pub errors: Vec<ErrorEntry>,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
pub struct CountInput {
    pub session_id: Option<String>,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CountOutput {
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct GetTitleOutput {
    pub session_id: SessionId,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct ScrollIntoViewInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct ScrollIntoViewOutput {
    pub success: bool,
    pub scrolls_needed: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PtyReadInput {
    pub session_id: Option<String>,
    pub max_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct PtyReadOutput {
    pub session_id: SessionId,
    pub data: String,
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct PtyWriteInput {
    pub session_id: Option<String>,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct PtyWriteOutput {
    pub session_id: SessionId,
    pub bytes_written: usize,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SessionInput {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct HealthInput;

#[derive(Debug, Clone)]
pub struct HealthOutput {
    pub status: String,
    pub pid: u32,
    pub uptime_ms: u64,
    pub session_count: usize,
    pub version: String,
    pub active_connections: usize,
    pub total_requests: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MetricsInput;

#[derive(Debug, Clone)]
pub struct MetricsOutput {
    pub requests_total: u64,
    pub errors_total: u64,
    pub lock_timeouts: u64,
    pub poison_recoveries: u64,
    pub uptime_ms: u64,
    pub active_connections: usize,
    pub session_count: usize,
}
