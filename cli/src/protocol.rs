use serde::{Deserialize, Serialize};

/// JSON-RPC request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Request {
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// Method names
pub const METHOD_SPAWN: &str = "spawn";
pub const METHOD_SNAPSHOT: &str = "snapshot";
pub const METHOD_CLICK: &str = "click";
pub const METHOD_FILL: &str = "fill";
pub const METHOD_KEYSTROKE: &str = "keystroke";
pub const METHOD_TYPE: &str = "type";
pub const METHOD_WAIT: &str = "wait";
pub const METHOD_KILL: &str = "kill";
pub const METHOD_SESSIONS: &str = "sessions";
pub const METHOD_PING: &str = "ping";
pub const METHOD_HEALTH: &str = "health";
// Extended methods
pub const METHOD_SELECT: &str = "select";
pub const METHOD_SCROLL: &str = "scroll";
pub const METHOD_FOCUS: &str = "focus";
pub const METHOD_CLEAR: &str = "clear";
pub const METHOD_GET_TEXT: &str = "get_text";
pub const METHOD_GET_VALUE: &str = "get_value";
pub const METHOD_IS_VISIBLE: &str = "is_visible";
pub const METHOD_IS_FOCUSED: &str = "is_focused";
pub const METHOD_SCREEN: &str = "screen";
pub const METHOD_TOGGLE: &str = "toggle";
pub const METHOD_RESIZE: &str = "resize";
pub const METHOD_ATTACH: &str = "attach";
// Recording and debugging methods
pub const METHOD_RECORD_START: &str = "record_start";
pub const METHOD_RECORD_STOP: &str = "record_stop";
pub const METHOD_RECORD_STATUS: &str = "record_status";
pub const METHOD_TRACE: &str = "trace";
pub const METHOD_CONSOLE: &str = "console";
// Semantic locator method
pub const METHOD_FIND: &str = "find";
// Scroll into view method (agent-browser parity)
pub const METHOD_SCROLL_INTO_VIEW: &str = "scroll_into_view";
// Get focused element method
pub const METHOD_GET_FOCUSED: &str = "get_focused";
// Get session title method
pub const METHOD_GET_TITLE: &str = "get_title";
// Restart session method (TUI equivalent of browser reload)
pub const METHOD_RESTART: &str = "restart";
// State query methods (browser parity)
pub const METHOD_IS_ENABLED: &str = "is_enabled";
pub const METHOD_IS_CHECKED: &str = "is_checked";
pub const METHOD_COUNT: &str = "count";
// Double-click method (agent-browser parity)
pub const METHOD_DBL_CLICK: &str = "dbl_click";
// Select all method (agent-browser parity)
pub const METHOD_SELECT_ALL: &str = "select_all";
// Key hold/release methods (agent-browser parity)
pub const METHOD_KEYDOWN: &str = "keydown";
pub const METHOD_KEYUP: &str = "keyup";
// Multi-select method (agent-browser parity)
pub const METHOD_MULTISELECT: &str = "multiselect";
// Errors method (agent-browser parity)
pub const METHOD_ERRORS: &str = "errors";
// PTY I/O methods for interactive attach
pub const METHOD_PTY_READ: &str = "pty_read";
pub const METHOD_PTY_WRITE: &str = "pty_write";

// Spawn params
#[derive(Debug, Serialize, Deserialize)]
pub struct SpawnParams {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub cols: Option<u16>,
    #[serde(default)]
    pub rows: Option<u16>,
}

// Snapshot params
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub include_elements: bool,
    #[serde(default)]
    pub format: SnapshotFormat,
    // Filtering options
    #[serde(default)]
    pub interactive_only: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotFormat {
    #[default]
    Text,
    Json,
    /// Accessibility-tree format (agent-browser style)
    Tree,
}

// Click params
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Double-click params (agent-browser parity)
#[derive(Debug, Serialize, Deserialize)]
pub struct DblClickParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Select all params (agent-browser parity)
#[derive(Debug, Serialize, Deserialize)]
pub struct SelectAllParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Fill params
#[derive(Debug, Serialize, Deserialize)]
pub struct FillParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Keystroke params
#[derive(Debug, Serialize, Deserialize)]
pub struct KeystrokeParams {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// KeyDown params (hold a key)
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyDownParams {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// KeyUp params (release a key)
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyUpParams {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Type params
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeParams {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Wait params
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    // New wait condition types
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<WaitCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitCondition {
    Text,
    Element,
    Focused,
    NotVisible,
    Stable,
    TextGone,
    Value,
}

// Kill params
#[derive(Debug, Serialize, Deserialize)]
pub struct KillParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// Restart params (TUI equivalent of browser reload)
#[derive(Debug, Serialize, Deserialize)]
pub struct RestartParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

// === Extended Command Params ===

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub option: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiSelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub options: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollParams {
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "ref")]
    pub element_ref: Option<String>,
    pub direction: String,
    #[serde(default)]
    pub amount: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FocusParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTextParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetValueParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsVisibleParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsFocusedParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsEnabledParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsCheckedParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub strip_ansi: bool,
    #[serde(default)]
    pub include_cursor: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Force specific state: true to check, false to uncheck, None to toggle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeParams {
    pub cols: u16,
    pub rows: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachParams {
    pub session: String,
}

// PTY I/O params for interactive attach
#[derive(Debug, Serialize, Deserialize)]
pub struct PtyReadParams {
    pub session: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PtyWriteParams {
    pub session: String,
    /// Base64-encoded data to write to PTY
    pub data: String,
}

// Response types
#[derive(Debug, Serialize, Deserialize)]
pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotResult {
    pub session_id: String,
    pub screen: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elements: Option<Vec<Element>>,
    pub cursor: Option<CursorPosition>,
    pub size: TerminalSize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Element {
    #[serde(rename = "ref")]
    pub element_ref: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub position: Position,
    pub focused: bool,
    pub selected: bool,
    // Enhanced properties
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children_refs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    Button,
    Input,
    Checkbox,
    Radio,
    Select,
    MenuItem,
    ListItem,
    Link,
    Spinner,
    Progress,
    Text,
    Container,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub row: u16,
    pub col: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClickResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DblClickResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectAllResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FillResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeystrokeResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyDownResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyUpResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeResult {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitResult {
    pub found: bool,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KillResult {
    pub success: bool,
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestartResult {
    pub success: bool,
    pub old_session_id: String,
    pub new_session_id: String,
    pub command: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub running: bool,
    pub created_at: String,
    pub size: TerminalSize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsResult {
    pub sessions: Vec<SessionInfo>,
    pub active_session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResult {
    pub status: String,
    pub pid: u32,
    pub uptime_ms: u64,
    pub session_count: usize,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_usage_mb: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_details: Option<MemoryDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_stats: Option<RequestStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reasons: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryDetails {
    pub heap_used_mb: f64,
    pub heap_total_mb: f64,
    pub rss_mb: f64,
    pub external_mb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub error_rate_percent: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_request_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests_by_method: Option<std::collections::HashMap<String, u64>>,
}

// === Extended Command Results ===

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectResult {
    pub success: bool,
    pub message: Option<String>,
    pub selected_option: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiSelectResult {
    pub success: bool,
    pub message: Option<String>,
    pub selected_options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrollResult {
    pub success: bool,
    pub scrolled_amount: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FocusResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearResult {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTextResult {
    pub success: bool,
    pub text: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetValueResult {
    pub success: bool,
    pub value: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsVisibleResult {
    pub visible: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsFocusedResult {
    pub focused: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsEnabledResult {
    pub enabled: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IsCheckedResult {
    pub checked: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountResult {
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenResult {
    pub session_id: String,
    pub screen: String,
    pub size: TerminalSize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleResult {
    pub success: bool,
    pub message: Option<String>,
    pub checked: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResizeResult {
    pub success: bool,
    pub session_id: String,
    pub size: TerminalSize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachResult {
    pub success: bool,
    pub session_id: String,
    pub message: Option<String>,
}

// PTY I/O results for interactive attach
#[derive(Debug, Serialize, Deserialize)]
pub struct PtyReadResult {
    pub session_id: String,
    /// Base64-encoded data read from PTY
    pub data: String,
    /// Number of bytes read
    pub bytes_read: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PtyWriteResult {
    pub success: bool,
    pub session_id: String,
}

// === Recording and Debugging Params/Results ===

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStartParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStopParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStatusParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsoleParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorsParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStartResult {
    pub success: bool,
    pub session_id: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStopResult {
    pub success: bool,
    pub session_id: String,
    pub frame_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub output_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordStatusResult {
    pub session_id: String,
    pub is_recording: bool,
    pub frame_count: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceEntry {
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceResult {
    pub session_id: String,
    pub is_tracing: bool,
    pub entries: Vec<TraceEntry>,
    pub formatted: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsoleResult {
    pub session_id: String,
    pub lines: Vec<String>,
    pub total_lines: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleared: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorsResult {
    pub session_id: String,
    pub errors: Vec<ErrorEntry>,
    pub total_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleared: Option<bool>,
}

// === Semantic Locator ===

#[derive(Debug, Serialize, Deserialize)]
pub struct FindParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    /// Select the nth matching element (0-indexed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
    /// Use exact string matching instead of substring matching
    #[serde(default)]
    pub exact: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindResult {
    pub elements: Vec<Element>,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Test that Request serialization produces correct JSON-RPC format
    #[test]
    fn test_request_serialization() {
        let request = Request::new(1, "spawn", Some(json!({"command": "bash"})));
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["jsonrpc"], "2.0", "JSON-RPC version must be 2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "spawn");
        assert_eq!(json["params"]["command"], "bash");
    }

    /// Test that Request without params omits the params field
    #[test]
    fn test_request_without_params() {
        let request = Request::new(1, "sessions", None);
        let json_str = serde_json::to_string(&request).unwrap();

        // params should not appear in serialized output
        assert!(!json_str.contains("params"), "Empty params should not be serialized");
    }

    /// Test Response deserialization with result
    #[test]
    fn test_response_deserialization_success() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"session_id":"abc123","pid":12345}}"#;
        let response: Response = serde_json::from_str(json).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    /// Test Response deserialization with error
    #[test]
    fn test_response_deserialization_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let response: Response = serde_json::from_str(json).unwrap();

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
    }

    /// Test SpawnParams serialization - verify field names match daemon expectations
    #[test]
    fn test_spawn_params_serialization() {
        let params = SpawnParams {
            command: "bash".to_string(),
            args: Some(vec!["--login".to_string()]),
            cwd: Some("/home/user".to_string()),
            env: None,
            session: None,
            cols: Some(120),
            rows: Some(40),
        };
        let json = serde_json::to_value(&params).unwrap();

        // Verify exact field names the daemon expects
        assert_eq!(json["command"], "bash");
        assert_eq!(json["args"][0], "--login");
        assert_eq!(json["cwd"], "/home/user");
        assert_eq!(json["cols"], 120);
        assert_eq!(json["rows"], 40);
    }

    /// Test ClickParams serialization - verify "ref" field name (not "element_ref")
    #[test]
    fn test_click_params_field_name() {
        let params = ClickParams {
            element_ref: "@btn1".to_string(),
            session: None,
        };
        let json = serde_json::to_value(&params).unwrap();

        // CRITICAL: The daemon expects "ref", not "element_ref"
        assert!(json.get("ref").is_some(), "Field must be named 'ref' for daemon");
        assert!(json.get("element_ref").is_none(), "Field must NOT be named 'element_ref'");
        assert_eq!(json["ref"], "@btn1");
    }

    /// Test FillParams serialization - verify "ref" field name
    #[test]
    fn test_fill_params_field_name() {
        let params = FillParams {
            element_ref: "@inp1".to_string(),
            value: "test value".to_string(),
            session: None,
        };
        let json = serde_json::to_value(&params).unwrap();

        assert!(json.get("ref").is_some(), "Field must be named 'ref' for daemon");
        assert_eq!(json["ref"], "@inp1");
        assert_eq!(json["value"], "test value");
    }

    /// Test all element-ref params use "ref" field name
    #[test]
    fn test_all_ref_params_use_correct_field_name() {
        // SelectParams
        let params = SelectParams {
            element_ref: "@sel1".to_string(),
            option: "opt1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // FocusParams
        let params = FocusParams {
            element_ref: "@inp1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // ClearParams
        let params = ClearParams {
            element_ref: "@inp1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // GetTextParams
        let params = GetTextParams {
            element_ref: "@btn1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // GetValueParams
        let params = GetValueParams {
            element_ref: "@inp1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // IsVisibleParams
        let params = IsVisibleParams {
            element_ref: "@btn1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // IsFocusedParams
        let params = IsFocusedParams {
            element_ref: "@inp1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());

        // ToggleParams
        let params = ToggleParams {
            element_ref: "@cb1".to_string(),
            session: None,
        };
        assert!(serde_json::to_value(&params).unwrap().get("ref").is_some());
    }

    /// Test SpawnResult deserialization
    #[test]
    fn test_spawn_result_deserialization() {
        let json = r#"{"session_id":"abc123","pid":12345}"#;
        let result: SpawnResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.session_id, "abc123");
        assert_eq!(result.pid, 12345);
    }

    /// Test SnapshotResult deserialization with elements
    #[test]
    fn test_snapshot_result_with_elements() {
        let json = r#"{
            "session_id": "abc123",
            "screen": "Hello World\n",
            "elements": [
                {
                    "ref": "@btn1",
                    "type": "button",
                    "label": "Submit",
                    "value": null,
                    "position": {"row": 5, "col": 10, "width": 8, "height": 1},
                    "focused": true,
                    "selected": false
                }
            ],
            "cursor": {"row": 0, "col": 0, "visible": true},
            "size": {"cols": 120, "rows": 40}
        }"#;

        let result: SnapshotResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.session_id, "abc123");
        assert_eq!(result.screen, "Hello World\n");
        assert!(result.elements.is_some());

        let elements = result.elements.unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_ref, "@btn1");
        assert!(matches!(elements[0].element_type, ElementType::Button));
        assert_eq!(elements[0].label, Some("Submit".to_string()));
        assert!(elements[0].focused);
    }

    /// Test Element deserialization - verify "ref" field is read correctly
    #[test]
    fn test_element_ref_field_deserialization() {
        let json = r#"{
            "ref": "@inp1",
            "type": "input",
            "label": "Name",
            "value": "John",
            "position": {"row": 1, "col": 1, "width": 20, "height": 1},
            "focused": false,
            "selected": false
        }"#;

        let element: Element = serde_json::from_str(json).unwrap();
        assert_eq!(element.element_ref, "@inp1");
        assert_eq!(element.value, Some("John".to_string()));
    }

    /// Test Element type deserialization for all types
    #[test]
    fn test_element_type_deserialization() {
        let test_cases = [
            ("button", ElementType::Button),
            ("input", ElementType::Input),
            ("checkbox", ElementType::Checkbox),
            ("radio", ElementType::Radio),
            ("select", ElementType::Select),
            ("menuitem", ElementType::MenuItem),
            ("listitem", ElementType::ListItem),
            ("link", ElementType::Link),
            ("spinner", ElementType::Spinner),
            ("progress", ElementType::Progress),
            ("text", ElementType::Text),
            ("container", ElementType::Container),
            ("unknown", ElementType::Unknown),
        ];

        for (type_str, expected_type) in test_cases {
            let json = format!(r#"{{
                "ref": "@el1",
                "type": "{}",
                "label": null,
                "value": null,
                "position": {{"row": 0, "col": 0}},
                "focused": false,
                "selected": false
            }}"#, type_str);

            let element: Element = serde_json::from_str(&json).unwrap();
            assert_eq!(element.element_type, expected_type);
        }
    }

    /// Test WaitResult deserialization with all optional fields
    #[test]
    fn test_wait_result_full() {
        let json = r#"{
            "found": true,
            "elapsed_ms": 1500,
            "screen_context": "Current screen...",
            "suggestion": "Try waiting longer",
            "matched_text": "Success",
            "element_ref": "@btn1"
        }"#;

        let result: WaitResult = serde_json::from_str(json).unwrap();
        assert!(result.found);
        assert_eq!(result.elapsed_ms, 1500);
        assert_eq!(result.screen_context, Some("Current screen...".to_string()));
        assert_eq!(result.matched_text, Some("Success".to_string()));
        assert_eq!(result.element_ref, Some("@btn1".to_string()));
    }

    /// Test WaitResult deserialization with minimal fields
    #[test]
    fn test_wait_result_minimal() {
        let json = r#"{"found": false, "elapsed_ms": 30000}"#;
        let result: WaitResult = serde_json::from_str(json).unwrap();

        assert!(!result.found);
        assert_eq!(result.elapsed_ms, 30000);
        assert!(result.screen_context.is_none());
    }

    /// Test HealthResult deserialization
    #[test]
    fn test_health_result_deserialization() {
        let json = r#"{
            "status": "healthy",
            "pid": 12345,
            "uptime_ms": 60000,
            "session_count": 2,
            "version": "1.0.0",
            "memory_usage_mb": 45.5
        }"#;

        let result: HealthResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.status, "healthy");
        assert_eq!(result.pid, 12345);
        assert_eq!(result.uptime_ms, 60000);
        assert_eq!(result.session_count, 2);
        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.memory_usage_mb, Some(45.5));
    }

    /// Test SessionInfo deserialization
    #[test]
    fn test_session_info_deserialization() {
        let json = r#"{
            "id": "abc123",
            "command": "bash",
            "pid": 12345,
            "running": true,
            "created_at": "2024-01-01T00:00:00Z",
            "size": {"cols": 120, "rows": 40}
        }"#;

        let session: SessionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "abc123");
        assert_eq!(session.command, "bash");
        assert_eq!(session.pid, 12345);
        assert!(session.running);
        assert_eq!(session.size.cols, 120);
        assert_eq!(session.size.rows, 40);
    }

    /// Test WaitCondition serialization
    #[test]
    fn test_wait_condition_serialization() {
        let test_cases = [
            (WaitCondition::Text, "text"),
            (WaitCondition::Element, "element"),
            (WaitCondition::Focused, "focused"),
            (WaitCondition::NotVisible, "not_visible"),
            (WaitCondition::Stable, "stable"),
            (WaitCondition::TextGone, "text_gone"),
            (WaitCondition::Value, "value"),
        ];

        for (condition, expected) in test_cases {
            let params = WaitParams {
                text: None,
                timeout_ms: Some(5000),
                session: None,
                condition: Some(condition),
                target: Some("@btn1".to_string()),
            };
            let json = serde_json::to_value(&params).unwrap();
            assert_eq!(json["condition"], expected);
        }
    }

    /// Test SnapshotFormat serialization
    #[test]
    fn test_snapshot_format_serialization() {
        let params = SnapshotParams {
            session: None,
            include_elements: true,
            format: SnapshotFormat::Json,
            interactive_only: false,
            compact: false,
            region: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["format"], "json");

        let params = SnapshotParams {
            session: None,
            include_elements: true,
            format: SnapshotFormat::Text,
            interactive_only: false,
            compact: false,
            region: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["format"], "text");
    }

    /// Test IsVisibleResult uses "ref" field name
    #[test]
    fn test_is_visible_result_ref_field() {
        let json = r#"{"visible": true, "ref": "@btn1"}"#;
        let result: IsVisibleResult = serde_json::from_str(json).unwrap();

        assert!(result.visible);
        assert_eq!(result.element_ref, "@btn1");
    }

    /// Test IsFocusedResult uses "ref" field name
    #[test]
    fn test_is_focused_result_ref_field() {
        let json = r#"{"focused": true, "ref": "@inp1"}"#;
        let result: IsFocusedResult = serde_json::from_str(json).unwrap();

        assert!(result.focused);
        assert_eq!(result.element_ref, "@inp1");
    }

    /// Test round-trip serialization for all param types
    #[test]
    fn test_params_round_trip() {
        // SpawnParams
        let original = SpawnParams {
            command: "htop".to_string(),
            args: Some(vec!["-d".to_string(), "1".to_string()]),
            cwd: Some("/tmp".to_string()),
            env: None,
            session: Some("test".to_string()),
            cols: Some(80),
            rows: Some(24),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: SpawnParams = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.command, original.command);
        assert_eq!(decoded.args, original.args);
        assert_eq!(decoded.cwd, original.cwd);
        assert_eq!(decoded.cols, original.cols);
        assert_eq!(decoded.rows, original.rows);

        // KeystrokeParams
        let original = KeystrokeParams {
            key: "Ctrl+C".to_string(),
            session: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: KeystrokeParams = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.key, original.key);

        // TypeParams
        let original = TypeParams {
            text: "Hello World".to_string(),
            session: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: TypeParams = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.text, original.text);
    }

    /// Test method constants match daemon expectations
    #[test]
    fn test_method_constants() {
        assert_eq!(METHOD_SPAWN, "spawn");
        assert_eq!(METHOD_SNAPSHOT, "snapshot");
        assert_eq!(METHOD_CLICK, "click");
        assert_eq!(METHOD_FILL, "fill");
        assert_eq!(METHOD_KEYSTROKE, "keystroke");
        assert_eq!(METHOD_TYPE, "type");
        assert_eq!(METHOD_WAIT, "wait");
        assert_eq!(METHOD_KILL, "kill");
        assert_eq!(METHOD_SESSIONS, "sessions");
        assert_eq!(METHOD_PING, "ping");
        assert_eq!(METHOD_HEALTH, "health");
        assert_eq!(METHOD_SELECT, "select");
        assert_eq!(METHOD_SCROLL, "scroll");
        assert_eq!(METHOD_FOCUS, "focus");
        assert_eq!(METHOD_CLEAR, "clear");
        assert_eq!(METHOD_GET_TEXT, "get_text");
        assert_eq!(METHOD_GET_VALUE, "get_value");
        assert_eq!(METHOD_IS_VISIBLE, "is_visible");
        assert_eq!(METHOD_IS_FOCUSED, "is_focused");
        assert_eq!(METHOD_SCREEN, "screen");
        assert_eq!(METHOD_TOGGLE, "toggle");
        assert_eq!(METHOD_RESIZE, "resize");
        assert_eq!(METHOD_ATTACH, "attach");
        assert_eq!(METHOD_RECORD_START, "record_start");
        assert_eq!(METHOD_RECORD_STOP, "record_stop");
        assert_eq!(METHOD_RECORD_STATUS, "record_status");
        assert_eq!(METHOD_TRACE, "trace");
        assert_eq!(METHOD_CONSOLE, "console");
        assert_eq!(METHOD_FIND, "find");
        assert_eq!(METHOD_IS_ENABLED, "is_enabled");
        assert_eq!(METHOD_IS_CHECKED, "is_checked");
        assert_eq!(METHOD_COUNT, "count");
        assert_eq!(METHOD_ERRORS, "errors");
    }

    /// Test ScrollParams optional element_ref field
    #[test]
    fn test_scroll_params_optional_element() {
        // Without element
        let params = ScrollParams {
            element_ref: None,
            direction: "down".to_string(),
            amount: 5,
            session: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert!(!json.as_object().unwrap().contains_key("ref"));

        // With element
        let params = ScrollParams {
            element_ref: Some("@list1".to_string()),
            direction: "down".to_string(),
            amount: 10,
            session: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["ref"], "@list1");
    }

    /// Test TraceEntry deserialization
    #[test]
    fn test_trace_entry_deserialization() {
        let json = r#"{
            "timestamp": 1234567890,
            "type": "request",
            "method": "click",
            "params": {"ref": "@btn1"},
            "result": {"success": true},
            "error": null,
            "duration": 50
        }"#;

        let entry: TraceEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.timestamp, 1234567890);
        assert_eq!(entry.entry_type, "request");
        assert_eq!(entry.method, Some("click".to_string()));
        assert_eq!(entry.duration, Some(50));
    }

    /// Test FindParams serialization
    #[test]
    fn test_find_params_serialization() {
        let params = FindParams {
            session: None,
            role: Some("button".to_string()),
            name: Some("Submit".to_string()),
            text: None,
            placeholder: None,
            focused: Some(true),
            nth: None,
            exact: false,
        };
        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["role"], "button");
        assert_eq!(json["name"], "Submit");
        assert_eq!(json["focused"], true);
    }

    /// Test FindParams serialization with nth and exact options
    #[test]
    fn test_find_params_with_nth_and_exact() {
        let params = FindParams {
            session: None,
            role: Some("button".to_string()),
            name: None,
            text: Some("Submit".to_string()),
            placeholder: None,
            focused: None,
            nth: Some(2),
            exact: true,
        };
        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["role"], "button");
        assert_eq!(json["text"], "Submit");
        assert_eq!(json["nth"], 2);
        assert_eq!(json["exact"], true);
    }

    /// Test FindParams serialization with placeholder
    #[test]
    fn test_find_params_with_placeholder() {
        let params = FindParams {
            session: None,
            role: Some("input".to_string()),
            name: None,
            text: None,
            placeholder: Some("Search...".to_string()),
            focused: None,
            nth: None,
            exact: false,
        };
        let json = serde_json::to_value(&params).unwrap();

        assert_eq!(json["role"], "input");
        assert_eq!(json["placeholder"], "Search...");
    }
}
