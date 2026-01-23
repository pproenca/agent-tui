use std::collections::HashMap;

use agent_tui_core::CursorPosition;
use agent_tui_core::Element;

use super::session_types::ErrorEntry;
use super::session_types::RecordingFrame;
use super::session_types::RecordingStatus;
use super::session_types::SessionId;
use super::session_types::SessionInfo;
use super::session_types::TraceEntry;

/// Input for spawning a new session.
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

/// Output from spawning a session.
#[derive(Debug, Clone)]
pub struct SpawnOutput {
    pub session_id: SessionId,
    pub pid: u32,
}

/// Input for taking a snapshot.
#[derive(Debug, Clone, Default)]
pub struct SnapshotInput {
    pub session_id: Option<String>,
    pub include_elements: bool,
    pub region: Option<String>,
    pub strip_ansi: bool,
    pub include_cursor: bool,
}

/// Output from taking a snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotOutput {
    pub session_id: SessionId,
    pub screen: String,
    pub elements: Option<Vec<Element>>,
    pub cursor: Option<CursorPosition>,
}

/// Input for clicking an element.
#[derive(Debug, Clone)]
pub struct ClickInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from clicking an element.
#[derive(Debug, Clone)]
pub struct ClickOutput {
    pub success: bool,
    pub message: Option<String>,
    pub warning: Option<String>,
}

/// Input for filling an element with text.
#[derive(Debug, Clone)]
pub struct FillInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub value: String,
}

/// Output from filling an element.
#[derive(Debug, Clone)]
pub struct FillOutput {
    pub success: bool,
    pub message: Option<String>,
}

/// Input for sending a keystroke.
#[derive(Debug, Clone)]
pub struct KeystrokeInput {
    pub session_id: Option<String>,
    pub key: String,
}

/// Output from sending a keystroke.
#[derive(Debug, Clone)]
pub struct KeystrokeOutput {
    pub success: bool,
}

/// Input for typing text.
#[derive(Debug, Clone)]
pub struct TypeInput {
    pub session_id: Option<String>,
    pub text: String,
}

/// Output from typing text.
#[derive(Debug, Clone)]
pub struct TypeOutput {
    pub success: bool,
}

/// Input for keydown event.
#[derive(Debug, Clone)]
pub struct KeydownInput {
    pub session_id: Option<String>,
    pub key: String,
}

/// Output from keydown event.
#[derive(Debug, Clone)]
pub struct KeydownOutput {
    pub success: bool,
}

/// Input for keyup event.
#[derive(Debug, Clone)]
pub struct KeyupInput {
    pub session_id: Option<String>,
    pub key: String,
}

/// Output from keyup event.
#[derive(Debug, Clone)]
pub struct KeyupOutput {
    pub success: bool,
}

/// Input for waiting for a condition.
#[derive(Debug, Clone)]
pub struct WaitInput {
    pub session_id: Option<String>,
    pub text: Option<String>,
    pub timeout_ms: u64,
    pub condition: Option<String>,
    pub target: Option<String>,
}

/// Output from waiting for a condition.
#[derive(Debug, Clone)]
pub struct WaitOutput {
    pub found: bool,
    pub elapsed_ms: u64,
}

/// Input for finding elements.
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

/// Output from finding elements.
#[derive(Debug, Clone)]
pub struct FindOutput {
    pub elements: Vec<Element>,
    pub count: usize,
}

/// Input for scrolling.
#[derive(Debug, Clone)]
pub struct ScrollInput {
    pub session_id: Option<String>,
    pub direction: String,
    pub amount: u16,
}

/// Output from scrolling.
#[derive(Debug, Clone)]
pub struct ScrollOutput {
    pub success: bool,
}

/// Input for resizing a session.
#[derive(Debug, Clone)]
pub struct ResizeInput {
    pub session_id: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

/// Output from resizing a session.
#[derive(Debug, Clone)]
pub struct ResizeOutput {
    pub session_id: SessionId,
    pub success: bool,
}

/// Output from listing sessions.
#[derive(Debug, Clone)]
pub struct SessionsOutput {
    pub sessions: Vec<SessionInfo>,
    pub active_session: Option<SessionId>,
}

/// Output from killing a session.
#[derive(Debug, Clone)]
pub struct KillOutput {
    pub session_id: SessionId,
    pub success: bool,
}

/// Input for getting element state.
#[derive(Debug, Clone)]
pub struct ElementStateInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output for element visibility check.
#[derive(Debug, Clone)]
pub struct VisibilityOutput {
    pub found: bool,
    pub visible: bool,
}

/// Output for element focus check.
#[derive(Debug, Clone)]
pub struct FocusCheckOutput {
    pub found: bool,
    pub focused: bool,
}

/// Output for element enabled check.
#[derive(Debug, Clone)]
pub struct IsEnabledOutput {
    pub found: bool,
    pub enabled: bool,
}

/// Output for element checked check.
#[derive(Debug, Clone)]
pub struct IsCheckedOutput {
    pub found: bool,
    pub checked: bool,
    pub message: Option<String>,
}

/// Output for getting element text.
#[derive(Debug, Clone)]
pub struct GetTextOutput {
    pub found: bool,
    pub text: String,
}

/// Output for getting element value.
#[derive(Debug, Clone)]
pub struct GetValueOutput {
    pub found: bool,
    pub value: String,
}

/// Output for getting the focused element.
#[derive(Debug, Clone)]
pub struct GetFocusedOutput {
    pub found: bool,
    pub element: Option<Element>,
}

/// Input for double-clicking an element.
#[derive(Debug, Clone)]
pub struct DoubleClickInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from double-clicking an element.
#[derive(Debug, Clone)]
pub struct DoubleClickOutput {
    pub success: bool,
}

/// Input for focusing an element.
#[derive(Debug, Clone)]
pub struct FocusInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from focusing an element.
#[derive(Debug, Clone)]
pub struct FocusOutput {
    pub success: bool,
}

/// Input for clearing an element's content.
#[derive(Debug, Clone)]
pub struct ClearInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from clearing an element.
#[derive(Debug, Clone)]
pub struct ClearOutput {
    pub success: bool,
}

/// Input for selecting all content in an element.
#[derive(Debug, Clone)]
pub struct SelectAllInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from selecting all content.
#[derive(Debug, Clone)]
pub struct SelectAllOutput {
    pub success: bool,
}

/// Input for toggling a checkbox.
#[derive(Debug, Clone)]
pub struct ToggleInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub state: Option<bool>,
}

/// Output from toggling a checkbox.
#[derive(Debug, Clone)]
pub struct ToggleOutput {
    pub success: bool,
    pub checked: bool,
    pub message: Option<String>,
}

/// Input for selecting an option.
#[derive(Debug, Clone)]
pub struct SelectInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub option: String,
}

/// Output from selecting an option.
#[derive(Debug, Clone)]
pub struct SelectOutput {
    pub success: bool,
    pub selected_option: String,
    pub message: Option<String>,
}

/// Input for multiselect.
#[derive(Debug, Clone)]
pub struct MultiselectInput {
    pub session_id: Option<String>,
    pub element_ref: String,
    pub options: Vec<String>,
}

/// Output from multiselect.
#[derive(Debug, Clone)]
pub struct MultiselectOutput {
    pub success: bool,
    pub selected_options: Vec<String>,
    pub message: Option<String>,
}

/// Input for starting recording.
#[derive(Debug, Clone)]
pub struct RecordStartInput {
    pub session_id: Option<String>,
}

/// Input for stopping recording.
#[derive(Debug, Clone)]
pub struct RecordStopInput {
    pub session_id: Option<String>,
    pub format: Option<String>,
}

/// Input for checking recording status.
#[derive(Debug, Clone)]
pub struct RecordStatusInput {
    pub session_id: Option<String>,
}

/// Output from starting recording.
#[derive(Debug, Clone)]
pub struct RecordStartOutput {
    pub session_id: SessionId,
    pub success: bool,
}

/// Output from stopping recording.
#[derive(Debug, Clone)]
pub struct RecordStopOutput {
    pub session_id: SessionId,
    pub frame_count: usize,
    pub frames: Vec<RecordingFrame>,
    pub format: String,
    pub cols: u16,
    pub rows: u16,
}

/// Output from checking recording status.
pub type RecordStatusOutput = RecordingStatus;

/// Input for trace operations.
#[derive(Debug, Clone)]
pub struct TraceInput {
    pub session_id: Option<String>,
    pub start: bool,
    pub stop: bool,
    pub count: usize,
}

/// Output from trace operations.
#[derive(Debug, Clone)]
pub struct TraceOutput {
    pub tracing: bool,
    pub entries: Vec<TraceEntry>,
}

/// Input for console operations.
#[derive(Debug, Clone)]
pub struct ConsoleInput {
    pub session_id: Option<String>,
    pub count: usize,
    pub clear: bool,
}

/// Output from console operations.
#[derive(Debug, Clone)]
pub struct ConsoleOutput {
    pub lines: Vec<String>,
}

/// Input for error operations.
#[derive(Debug, Clone)]
pub struct ErrorsInput {
    pub session_id: Option<String>,
    pub count: usize,
    pub clear: bool,
}

/// Output from error operations.
#[derive(Debug, Clone)]
pub struct ErrorsOutput {
    pub errors: Vec<ErrorEntry>,
    pub total_count: usize,
}

/// Input for counting elements.
#[derive(Debug, Clone)]
pub struct CountInput {
    pub session_id: Option<String>,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
}

/// Output from counting elements.
#[derive(Debug, Clone)]
pub struct CountOutput {
    pub count: usize,
}

/// Output for getting session title.
#[derive(Debug, Clone)]
pub struct GetTitleOutput {
    pub session_id: SessionId,
    pub title: String,
}

/// Input for scrolling an element into view.
#[derive(Debug, Clone)]
pub struct ScrollIntoViewInput {
    pub session_id: Option<String>,
    pub element_ref: String,
}

/// Output from scrolling an element into view.
#[derive(Debug, Clone)]
pub struct ScrollIntoViewOutput {
    pub success: bool,
    pub scrolls_needed: usize,
    pub message: Option<String>,
}

/// Input for PTY read operations.
#[derive(Debug, Clone)]
pub struct PtyReadInput {
    pub session_id: Option<String>,
    pub max_bytes: usize,
}

/// Output from PTY read operations.
#[derive(Debug, Clone)]
pub struct PtyReadOutput {
    pub session_id: SessionId,
    pub data: String,
    pub bytes_read: usize,
}

/// Input for PTY write operations.
#[derive(Debug, Clone)]
pub struct PtyWriteInput {
    pub session_id: Option<String>,
    pub data: String,
}

/// Output from PTY write operations.
#[derive(Debug, Clone)]
pub struct PtyWriteOutput {
    pub session_id: SessionId,
    pub bytes_written: usize,
    pub success: bool,
}

/// Input for session-only operations (recording, health checks, etc.)
#[derive(Debug, Clone)]
pub struct SessionInput {
    pub session_id: Option<String>,
}

/// Input for health check.
#[derive(Debug, Clone, Default)]
pub struct HealthInput;

/// Output from health check.
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

/// Input for metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsInput;

/// Output from metrics.
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
