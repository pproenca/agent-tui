use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use super::session_types::ErrorEntry;
use super::session_types::RecordingFrame;
use super::session_types::RecordingStatus;
use super::session_types::SessionId;
use super::session_types::SessionInfo;
use super::session_types::TraceEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollDirectionError {
    pub invalid_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitConditionTypeError {
    pub invalid_value: String,
}

impl fmt::Display for WaitConditionTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid wait condition type '{}'. Must be one of: text, element, focused, not_visible, stable, text_gone, value",
            self.invalid_value
        )
    }
}

impl std::error::Error for WaitConditionTypeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitConditionType {
    Text,
    Element,
    Focused,
    NotVisible,
    Stable,
    TextGone,
    Value,
}

impl WaitConditionType {
    pub fn parse(s: &str) -> Result<Self, WaitConditionTypeError> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "element" => Ok(Self::Element),
            "focused" => Ok(Self::Focused),
            "not_visible" => Ok(Self::NotVisible),
            "stable" => Ok(Self::Stable),
            "text_gone" => Ok(Self::TextGone),
            "value" => Ok(Self::Value),
            _ => Err(WaitConditionTypeError {
                invalid_value: s.to_string(),
            }),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Element => "element",
            Self::Focused => "focused",
            Self::NotVisible => "not_visible",
            Self::Stable => "stable",
            Self::TextGone => "text_gone",
            Self::Value => "value",
        }
    }

    pub fn requires_target(&self) -> bool {
        matches!(
            self,
            Self::Element | Self::Focused | Self::NotVisible | Self::Value
        )
    }

    pub fn requires_text(&self) -> bool {
        matches!(self, Self::Text | Self::TextGone | Self::Value)
    }
}

impl fmt::Display for WaitConditionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for WaitConditionType {
    type Err = WaitConditionTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for ScrollDirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid scroll direction '{}'. Must be one of: up, down, left, right",
            self.invalid_value
        )
    }
}

impl std::error::Error for ScrollDirectionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn parse(s: &str) -> Result<Self, ScrollDirectionError> {
        match s.to_lowercase().as_str() {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(ScrollDirectionError {
                invalid_value: s.to_string(),
            }),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

impl fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ScrollDirection {
    type Err = ScrollDirectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DomainCursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct DomainPosition {
    pub row: u16,
    pub col: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainElementType {
    Button,
    Input,
    Checkbox,
    Radio,
    Select,
    MenuItem,
    ListItem,
    Spinner,
    Progress,
    Link,
}

impl DomainElementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainElementType::Button => "button",
            DomainElementType::Input => "input",
            DomainElementType::Checkbox => "checkbox",
            DomainElementType::Radio => "radio",
            DomainElementType::Select => "select",
            DomainElementType::MenuItem => "menuitem",
            DomainElementType::ListItem => "listitem",
            DomainElementType::Spinner => "spinner",
            DomainElementType::Progress => "progress",
            DomainElementType::Link => "link",
        }
    }

    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            DomainElementType::Button
                | DomainElementType::Input
                | DomainElementType::Checkbox
                | DomainElementType::Radio
                | DomainElementType::Select
                | DomainElementType::MenuItem
                | DomainElementType::Link
        )
    }

    pub fn accepts_input(&self) -> bool {
        matches!(self, DomainElementType::Input)
    }

    pub fn is_toggleable(&self) -> bool {
        matches!(self, DomainElementType::Checkbox | DomainElementType::Radio)
    }
}

#[derive(Debug, Clone)]
pub struct DomainElement {
    pub element_ref: String,
    pub element_type: DomainElementType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub position: DomainPosition,
    pub focused: bool,
    pub selected: bool,
    pub checked: Option<bool>,
    pub disabled: Option<bool>,
    pub hint: Option<String>,
}

impl DomainElement {
    pub fn is_interactive(&self) -> bool {
        self.element_type.is_interactive()
    }

    pub fn can_click(&self) -> bool {
        self.is_interactive() && !self.is_disabled()
    }

    pub fn can_type(&self) -> bool {
        self.element_type.accepts_input() && !self.is_disabled()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.unwrap_or(false)
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_checked(&self) -> Option<bool> {
        self.checked
    }

    pub fn display_text(&self) -> Option<&str> {
        self.label.as_deref().or(self.value.as_deref())
    }
}

#[derive(Debug, Clone)]
pub struct SpawnInput {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub session_id: Option<SessionId>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct SpawnOutput {
    pub session_id: SessionId,
    pub pid: u32,
}

#[derive(Debug, Clone)]
pub struct RestartOutput {
    pub old_session_id: SessionId,
    pub new_session_id: SessionId,
    pub command: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotInput {
    pub session_id: Option<SessionId>,
    pub include_elements: bool,
    pub region: Option<String>,
    pub strip_ansi: bool,
    pub include_cursor: bool,
}

#[derive(Debug, Clone)]
pub struct SnapshotOutput {
    pub session_id: SessionId,
    pub screen: String,
    pub elements: Option<Vec<DomainElement>>,
    pub cursor: Option<DomainCursorPosition>,
}

#[derive(Debug, Clone, Default)]
pub struct AccessibilitySnapshotInput {
    pub session_id: Option<SessionId>,
    pub interactive_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainBoundsError {
    pub message: String,
}

impl fmt::Display for DomainBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DomainBoundsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainRole {
    Button,
    Tab,
    Input,
    StaticText,
    Panel,
    Checkbox,
    MenuItem,
    Status,
    ToolBlock,
    PromptMarker,
    ProgressBar,
    Link,
    ErrorMessage,
    DiffLine,
    CodeBlock,
}

impl DomainRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainRole::Button => "button",
            DomainRole::Tab => "tab",
            DomainRole::Input => "input",
            DomainRole::StaticText => "text",
            DomainRole::Panel => "panel",
            DomainRole::Checkbox => "checkbox",
            DomainRole::MenuItem => "menuitem",
            DomainRole::Status => "status",
            DomainRole::ToolBlock => "toolblock",
            DomainRole::PromptMarker => "prompt",
            DomainRole::ProgressBar => "progressbar",
            DomainRole::Link => "link",
            DomainRole::ErrorMessage => "error",
            DomainRole::DiffLine => "diff",
            DomainRole::CodeBlock => "codeblock",
        }
    }

    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            DomainRole::Button
                | DomainRole::Tab
                | DomainRole::Input
                | DomainRole::Checkbox
                | DomainRole::MenuItem
                | DomainRole::PromptMarker
                | DomainRole::Link
        )
    }
}

impl fmt::Display for DomainRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainBounds {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl DomainBounds {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Result<Self, DomainBoundsError> {
        if width == 0 {
            return Err(DomainBoundsError {
                message: "Width must be at least 1".to_string(),
            });
        }
        if height == 0 {
            return Err(DomainBoundsError {
                message: "Height must be at least 1".to_string(),
            });
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn new_unchecked(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn x(&self) -> u16 {
        self.x
    }

    pub fn y(&self) -> u16 {
        self.y
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone)]
pub struct DomainElementRef {
    pub role: DomainRole,
    pub name: Option<String>,
    pub bounds: DomainBounds,
    pub visual_hash: u64,
    pub nth: Option<usize>,
    pub selected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DomainRefMap {
    pub refs: HashMap<String, DomainElementRef>,
}

impl DomainRefMap {
    pub fn get(&self, ref_id: &str) -> Option<&DomainElementRef> {
        self.refs.get(ref_id)
    }
}

#[derive(Debug, Clone)]
pub struct DomainSnapshotStats {
    pub total: usize,
    pub interactive: usize,
    pub lines: usize,
}

#[derive(Debug, Clone)]
pub struct DomainAccessibilitySnapshot {
    pub tree: String,
    pub refs: DomainRefMap,
    pub stats: DomainSnapshotStats,
}

#[derive(Debug, Clone)]
pub struct AccessibilitySnapshotOutput {
    pub session_id: SessionId,
    pub snapshot: DomainAccessibilitySnapshot,
}

#[derive(Debug, Clone)]
pub struct ClickInput {
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeystrokeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInput {
    pub session_id: Option<SessionId>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TypeOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeydownInput {
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeydownOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct KeyupInput {
    pub session_id: Option<SessionId>,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct KeyupOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct WaitInput {
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub elements: Vec<DomainElement>,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct ScrollInput {
    pub session_id: Option<SessionId>,
    pub direction: String,
    pub amount: u16,
}

#[derive(Debug, Clone)]
pub struct ScrollOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ResizeInput {
    pub session_id: Option<SessionId>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct ResizeOutput {
    pub session_id: SessionId,
    pub success: bool,
    pub cols: u16,
    pub rows: u16,
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
    pub session_id: Option<SessionId>,
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
    pub element: Option<DomainElement>,
}

#[derive(Debug, Clone)]
pub struct DoubleClickInput {
    pub session_id: Option<SessionId>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct DoubleClickOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct FocusInput {
    pub session_id: Option<SessionId>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct FocusOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ClearInput {
    pub session_id: Option<SessionId>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct ClearOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SelectAllInput {
    pub session_id: Option<SessionId>,
    pub element_ref: String,
}

#[derive(Debug, Clone)]
pub struct SelectAllOutput {
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ToggleInput {
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct RecordStopInput {
    pub session_id: Option<SessionId>,
    pub format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RecordStatusInput {
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
    pub count: usize,
    pub clear: bool,
}

#[derive(Debug, Clone)]
pub struct ConsoleOutput {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ErrorsInput {
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
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
    pub session_id: Option<SessionId>,
    pub max_bytes: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PtyReadOutput {
    pub session_id: SessionId,
    pub data: Vec<u8>,
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct PtyWriteInput {
    pub session_id: Option<SessionId>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PtyWriteOutput {
    pub session_id: SessionId,
    pub bytes_written: usize,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SessionInput {
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct AttachInput {
    pub session_id: SessionId,
}

#[derive(Debug, Clone)]
pub struct AttachOutput {
    pub session_id: SessionId,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStartInput {
    pub session_id: Option<SessionId>,
    pub listen_addr: Option<String>,
    pub allow_remote: bool,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStartOutput {
    pub session_id: SessionId,
    pub listen_addr: String,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStopOutput {
    pub stopped: bool,
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct LivePreviewStatusOutput {
    pub running: bool,
    pub session_id: Option<SessionId>,
    pub listen_addr: Option<String>,
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
    pub commit: String,
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

#[derive(Debug, Clone)]
pub struct CleanupInput {
    pub all: bool,
}

#[derive(Debug, Clone)]
pub struct CleanupFailure {
    pub session_id: SessionId,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct CleanupOutput {
    pub cleaned: usize,
    pub failures: Vec<CleanupFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertConditionType {
    Text,
    Element,
    Session,
}

impl AssertConditionType {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "element" => Ok(Self::Element),
            "session" => Ok(Self::Session),
            _ => Err(format!(
                "Unknown condition type: {}. Use: text, element, or session",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Element => "element",
            Self::Session => "session",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssertInput {
    pub session_id: Option<SessionId>,
    pub condition_type: AssertConditionType,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct AssertOutput {
    pub passed: bool,
    pub condition: String,
}

#[derive(Debug, Clone, Default)]
pub struct ShutdownInput;

#[derive(Debug, Clone)]
pub struct ShutdownOutput {
    pub acknowledged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod scroll_direction_tests {
        use super::*;

        #[test]
        fn test_scroll_direction_from_str_up() {
            let dir = ScrollDirection::parse("up").expect("Should parse 'up'");
            assert_eq!(dir, ScrollDirection::Up);
        }

        #[test]
        fn test_scroll_direction_from_str_down() {
            let dir = ScrollDirection::parse("down").expect("Should parse 'down'");
            assert_eq!(dir, ScrollDirection::Down);
        }

        #[test]
        fn test_scroll_direction_from_str_left() {
            let dir = ScrollDirection::parse("left").expect("Should parse 'left'");
            assert_eq!(dir, ScrollDirection::Left);
        }

        #[test]
        fn test_scroll_direction_from_str_right() {
            let dir = ScrollDirection::parse("right").expect("Should parse 'right'");
            assert_eq!(dir, ScrollDirection::Right);
        }

        #[test]
        fn test_scroll_direction_from_str_invalid() {
            let result = ScrollDirection::parse("diagonal");
            assert!(result.is_err(), "Invalid direction should be rejected");
        }

        #[test]
        fn test_scroll_direction_from_str_empty() {
            let result = ScrollDirection::parse("");
            assert!(result.is_err(), "Empty string should be rejected");
        }

        #[test]
        fn test_scroll_direction_case_insensitive() {
            assert_eq!(ScrollDirection::parse("UP").unwrap(), ScrollDirection::Up);
            assert_eq!(
                ScrollDirection::parse("Down").unwrap(),
                ScrollDirection::Down
            );
            assert_eq!(
                ScrollDirection::parse("LEFT").unwrap(),
                ScrollDirection::Left
            );
            assert_eq!(
                ScrollDirection::parse("RIGHT").unwrap(),
                ScrollDirection::Right
            );
        }

        #[test]
        fn test_scroll_direction_as_str() {
            assert_eq!(ScrollDirection::Up.as_str(), "up");
            assert_eq!(ScrollDirection::Down.as_str(), "down");
            assert_eq!(ScrollDirection::Left.as_str(), "left");
            assert_eq!(ScrollDirection::Right.as_str(), "right");
        }

        #[test]
        fn test_scroll_direction_display() {
            assert_eq!(format!("{}", ScrollDirection::Up), "up");
            assert_eq!(format!("{}", ScrollDirection::Down), "down");
        }
    }

    mod wait_condition_type_tests {
        use super::*;

        #[test]
        fn test_wait_condition_type_from_str_text() {
            let cond = WaitConditionType::parse("text").expect("Should parse 'text'");
            assert_eq!(cond, WaitConditionType::Text);
        }

        #[test]
        fn test_wait_condition_type_from_str_element() {
            let cond = WaitConditionType::parse("element").expect("Should parse 'element'");
            assert_eq!(cond, WaitConditionType::Element);
        }

        #[test]
        fn test_wait_condition_type_from_str_focused() {
            let cond = WaitConditionType::parse("focused").expect("Should parse 'focused'");
            assert_eq!(cond, WaitConditionType::Focused);
        }

        #[test]
        fn test_wait_condition_type_from_str_not_visible() {
            let cond = WaitConditionType::parse("not_visible").expect("Should parse 'not_visible'");
            assert_eq!(cond, WaitConditionType::NotVisible);
        }

        #[test]
        fn test_wait_condition_type_from_str_stable() {
            let cond = WaitConditionType::parse("stable").expect("Should parse 'stable'");
            assert_eq!(cond, WaitConditionType::Stable);
        }

        #[test]
        fn test_wait_condition_type_from_str_text_gone() {
            let cond = WaitConditionType::parse("text_gone").expect("Should parse 'text_gone'");
            assert_eq!(cond, WaitConditionType::TextGone);
        }

        #[test]
        fn test_wait_condition_type_from_str_value() {
            let cond = WaitConditionType::parse("value").expect("Should parse 'value'");
            assert_eq!(cond, WaitConditionType::Value);
        }

        #[test]
        fn test_wait_condition_type_from_str_invalid() {
            let result = WaitConditionType::parse("invalid");
            assert!(result.is_err(), "Invalid condition should be rejected");
        }

        #[test]
        fn test_wait_condition_type_from_str_empty() {
            let result = WaitConditionType::parse("");
            assert!(result.is_err(), "Empty string should be rejected");
        }

        #[test]
        fn test_wait_condition_type_case_insensitive() {
            assert_eq!(
                WaitConditionType::parse("TEXT").unwrap(),
                WaitConditionType::Text
            );
            assert_eq!(
                WaitConditionType::parse("Element").unwrap(),
                WaitConditionType::Element
            );
            assert_eq!(
                WaitConditionType::parse("STABLE").unwrap(),
                WaitConditionType::Stable
            );
        }

        #[test]
        fn test_wait_condition_type_as_str() {
            assert_eq!(WaitConditionType::Text.as_str(), "text");
            assert_eq!(WaitConditionType::Element.as_str(), "element");
            assert_eq!(WaitConditionType::Focused.as_str(), "focused");
            assert_eq!(WaitConditionType::NotVisible.as_str(), "not_visible");
            assert_eq!(WaitConditionType::Stable.as_str(), "stable");
            assert_eq!(WaitConditionType::TextGone.as_str(), "text_gone");
            assert_eq!(WaitConditionType::Value.as_str(), "value");
        }

        #[test]
        fn test_wait_condition_type_display() {
            assert_eq!(format!("{}", WaitConditionType::Text), "text");
            assert_eq!(format!("{}", WaitConditionType::Stable), "stable");
        }

        #[test]
        fn test_wait_condition_type_requires_target() {
            assert!(!WaitConditionType::Text.requires_target());
            assert!(WaitConditionType::Element.requires_target());
            assert!(WaitConditionType::Focused.requires_target());
            assert!(WaitConditionType::NotVisible.requires_target());
            assert!(!WaitConditionType::Stable.requires_target());
            assert!(!WaitConditionType::TextGone.requires_target());
            assert!(WaitConditionType::Value.requires_target());
        }

        #[test]
        fn test_wait_condition_type_requires_text() {
            assert!(WaitConditionType::Text.requires_text());
            assert!(!WaitConditionType::Element.requires_text());
            assert!(!WaitConditionType::Focused.requires_text());
            assert!(!WaitConditionType::NotVisible.requires_text());
            assert!(!WaitConditionType::Stable.requires_text());
            assert!(WaitConditionType::TextGone.requires_text());
            assert!(WaitConditionType::Value.requires_text());
        }

        #[test]
        fn test_wait_condition_type_error_message() {
            let err = WaitConditionType::parse("invalid").unwrap_err();
            assert!(err.to_string().contains("invalid"));
            assert!(err.to_string().contains("text"));
        }
    }

    mod domain_bounds_tests {
        use super::*;

        #[test]
        fn test_domain_bounds_valid() {
            let bounds = DomainBounds::new(10, 20, 100, 50).expect("Valid bounds");
            assert_eq!(bounds.x(), 10);
            assert_eq!(bounds.y(), 20);
            assert_eq!(bounds.width(), 100);
            assert_eq!(bounds.height(), 50);
        }

        #[test]
        fn test_domain_bounds_rejects_zero_width() {
            let result = DomainBounds::new(0, 0, 0, 10);
            assert!(result.is_err(), "Zero width should be rejected");
            assert!(result.unwrap_err().message.contains("Width"));
        }

        #[test]
        fn test_domain_bounds_rejects_zero_height() {
            let result = DomainBounds::new(0, 0, 10, 0);
            assert!(result.is_err(), "Zero height should be rejected");
            assert!(result.unwrap_err().message.contains("Height"));
        }

        #[test]
        fn test_domain_bounds_accepts_minimum() {
            let bounds = DomainBounds::new(0, 0, 1, 1).expect("Minimum valid bounds");
            assert_eq!(bounds.width(), 1);
            assert_eq!(bounds.height(), 1);
        }

        #[test]
        fn test_domain_bounds_at_origin() {
            let bounds = DomainBounds::new(0, 0, 10, 10).expect("Bounds at origin");
            assert_eq!(bounds.x(), 0);
            assert_eq!(bounds.y(), 0);
        }

        #[test]
        fn test_domain_bounds_contains() {
            let bounds = DomainBounds::new(10, 10, 20, 10).expect("Valid bounds");
            assert!(bounds.contains(10, 10));
            assert!(bounds.contains(15, 15));
            assert!(bounds.contains(29, 19));
            assert!(!bounds.contains(30, 10));
            assert!(!bounds.contains(10, 20));
            assert!(!bounds.contains(9, 10));
            assert!(!bounds.contains(10, 9));
        }

        #[test]
        fn test_domain_bounds_unchecked() {
            let bounds = DomainBounds::new_unchecked(0, 0, 0, 0);
            assert_eq!(bounds.width(), 0);
            assert_eq!(bounds.height(), 0);
        }

        #[test]
        fn test_domain_bounds_error_display() {
            let err = DomainBounds::new(0, 0, 0, 10).unwrap_err();
            assert!(!err.to_string().is_empty());
        }
    }

    mod domain_element_type_tests {
        use super::*;

        #[test]
        fn test_element_type_is_interactive() {
            assert!(DomainElementType::Button.is_interactive());
            assert!(DomainElementType::Input.is_interactive());
            assert!(DomainElementType::Checkbox.is_interactive());
            assert!(DomainElementType::Radio.is_interactive());
            assert!(DomainElementType::Select.is_interactive());
            assert!(DomainElementType::MenuItem.is_interactive());
            assert!(DomainElementType::Link.is_interactive());

            assert!(!DomainElementType::ListItem.is_interactive());
            assert!(!DomainElementType::Spinner.is_interactive());
            assert!(!DomainElementType::Progress.is_interactive());
        }

        #[test]
        fn test_element_type_accepts_input() {
            assert!(DomainElementType::Input.accepts_input());
            assert!(!DomainElementType::Button.accepts_input());
            assert!(!DomainElementType::Checkbox.accepts_input());
        }

        #[test]
        fn test_element_type_is_toggleable() {
            assert!(DomainElementType::Checkbox.is_toggleable());
            assert!(DomainElementType::Radio.is_toggleable());
            assert!(!DomainElementType::Button.is_toggleable());
            assert!(!DomainElementType::Input.is_toggleable());
        }
    }

    mod domain_element_tests {
        use super::*;

        fn make_element(element_type: DomainElementType, disabled: Option<bool>) -> DomainElement {
            DomainElement {
                element_ref: "test".to_string(),
                element_type,
                label: Some("Test".to_string()),
                value: None,
                position: DomainPosition {
                    row: 0,
                    col: 0,
                    width: Some(10),
                    height: Some(1),
                },
                focused: false,
                selected: false,
                checked: None,
                disabled,
                hint: None,
            }
        }

        #[test]
        fn test_element_is_interactive() {
            let button = make_element(DomainElementType::Button, None);
            assert!(button.is_interactive());

            let progress = make_element(DomainElementType::Progress, None);
            assert!(!progress.is_interactive());
        }

        #[test]
        fn test_element_can_click_enabled() {
            let button = make_element(DomainElementType::Button, None);
            assert!(button.can_click());

            let disabled_button = make_element(DomainElementType::Button, Some(true));
            assert!(!disabled_button.can_click());
        }

        #[test]
        fn test_element_can_click_non_interactive() {
            let progress = make_element(DomainElementType::Progress, None);
            assert!(!progress.can_click());
        }

        #[test]
        fn test_element_can_type() {
            let input = make_element(DomainElementType::Input, None);
            assert!(input.can_type());

            let disabled_input = make_element(DomainElementType::Input, Some(true));
            assert!(!disabled_input.can_type());

            let button = make_element(DomainElementType::Button, None);
            assert!(!button.can_type());
        }

        #[test]
        fn test_element_is_disabled() {
            let enabled = make_element(DomainElementType::Button, None);
            assert!(!enabled.is_disabled());
            assert!(enabled.is_enabled());

            let explicit_enabled = make_element(DomainElementType::Button, Some(false));
            assert!(!explicit_enabled.is_disabled());
            assert!(explicit_enabled.is_enabled());

            let disabled = make_element(DomainElementType::Button, Some(true));
            assert!(disabled.is_disabled());
            assert!(!disabled.is_enabled());
        }

        #[test]
        fn test_element_display_text() {
            let with_label = DomainElement {
                element_ref: "test".to_string(),
                element_type: DomainElementType::Button,
                label: Some("Click Me".to_string()),
                value: Some("ignored".to_string()),
                position: DomainPosition {
                    row: 0,
                    col: 0,
                    width: None,
                    height: None,
                },
                focused: false,
                selected: false,
                checked: None,
                disabled: None,
                hint: None,
            };
            assert_eq!(with_label.display_text(), Some("Click Me"));

            let with_value_only = DomainElement {
                element_ref: "test".to_string(),
                element_type: DomainElementType::Input,
                label: None,
                value: Some("typed text".to_string()),
                position: DomainPosition {
                    row: 0,
                    col: 0,
                    width: None,
                    height: None,
                },
                focused: false,
                selected: false,
                checked: None,
                disabled: None,
                hint: None,
            };
            assert_eq!(with_value_only.display_text(), Some("typed text"));

            let no_text = DomainElement {
                element_ref: "test".to_string(),
                element_type: DomainElementType::Button,
                label: None,
                value: None,
                position: DomainPosition {
                    row: 0,
                    col: 0,
                    width: None,
                    height: None,
                },
                focused: false,
                selected: false,
                checked: None,
                disabled: None,
                hint: None,
            };
            assert_eq!(no_text.display_text(), None);
        }

        #[test]
        fn test_element_state_methods() {
            let focused_selected = DomainElement {
                element_ref: "test".to_string(),
                element_type: DomainElementType::MenuItem,
                label: None,
                value: None,
                position: DomainPosition {
                    row: 0,
                    col: 0,
                    width: None,
                    height: None,
                },
                focused: true,
                selected: true,
                checked: Some(true),
                disabled: None,
                hint: None,
            };
            assert!(focused_selected.is_focused());
            assert!(focused_selected.is_selected());
            assert_eq!(focused_selected.is_checked(), Some(true));
        }
    }
}
