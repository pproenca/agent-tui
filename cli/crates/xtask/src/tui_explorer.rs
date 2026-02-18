use chrono::Utc;
use clap::Args;
use clap::Subcommand;
use serde::Serialize;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

const EXIT_SUCCESS: i32 = 0;
const EXIT_SCENARIO_FAILED: i32 = 1;
const EXIT_SPEC_ERROR: i32 = 2;
const EXIT_UNAVAILABLE: i32 = 69;

const DEFAULT_SAFE_ACTIONS: [&str; 8] = [
    "Enter",
    "Tab",
    "ArrowDown",
    "ArrowUp",
    "ArrowRight",
    "ArrowLeft",
    "Esc",
    "Space",
];

const DEFAULT_RISKY_ACTIONS: [&str; 4] = ["q", "Ctrl+C", "Ctrl+D", "F10"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplorerErrorKind {
    Spec,
    Scenario,
    Unavailable,
}

#[derive(Debug, Clone)]
struct ExplorerError {
    kind: ExplorerErrorKind,
    message: String,
}

impl ExplorerError {
    fn spec(message: impl Into<String>) -> Self {
        Self {
            kind: ExplorerErrorKind::Spec,
            message: message.into(),
        }
    }

    fn scenario(message: impl Into<String>) -> Self {
        Self {
            kind: ExplorerErrorKind::Scenario,
            message: message.into(),
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            kind: ExplorerErrorKind::Unavailable,
            message: message.into(),
        }
    }
}

impl fmt::Display for ExplorerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExplorerError {}

#[derive(Subcommand, Debug, Clone)]
pub enum TuiExplorerCommands {
    #[command(about = "Explore a TUI app and generate acceptance artifacts")]
    Discover(DiscoverArgs),
    #[command(about = "Replay acceptance scenarios and fail on scenario errors")]
    Verify(VerifyArgs),
}

#[derive(Args, Debug, Clone)]
pub struct DiscoverArgs {
    /// Shell command that starts the target TUI app.
    #[arg(long)]
    command: String,
    /// Optional working directory for the spawned app.
    #[arg(long)]
    cwd: Option<String>,
    /// Terminal width in columns.
    #[arg(long, default_value_t = 120)]
    cols: u16,
    /// Terminal height in rows.
    #[arg(long, default_value_t = 40)]
    rows: u16,
    /// Maximum BFS action-path depth.
    #[arg(long, default_value_t = 5)]
    max_depth: usize,
    /// Maximum number of explored states.
    #[arg(long, default_value_t = 40)]
    max_states: usize,
    /// Maximum number of actions expanded per state.
    #[arg(long, default_value_t = 4)]
    branch_limit: usize,
    /// Hard wall-clock budget for discovery.
    #[arg(long, default_value_t = 180)]
    time_budget_sec: u64,
    /// Optional output directory; defaults to .agent-tui/discover/<timestamp>/.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Allow risky actions (q, Ctrl+C, Ctrl+D, F10).
    #[arg(long)]
    allow_risky: bool,
}

#[derive(Args, Debug, Clone)]
pub struct VerifyArgs {
    /// Path to the markdown acceptance spec.
    #[arg(long)]
    spec: PathBuf,
    /// Run only one scenario by exact name.
    #[arg(long)]
    scenario: Option<String>,
    /// Optional output directory for reports and failures.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Stop replay on first failing scenario.
    #[arg(long, default_value_t = true)]
    fail_fast: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Expect(String),
    Press(String),
    Type(String),
    WaitStable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scenario {
    name: String,
    steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
struct Spec {
    frontmatter: BTreeMap<String, Value>,
    scenarios: Vec<Scenario>,
}

#[derive(Clone, Debug)]
struct DiscoverConfig {
    command: String,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    max_depth: usize,
    max_states: usize,
    branch_limit: usize,
    time_budget_sec: u64,
    out_dir: PathBuf,
    allow_risky: bool,
    default_timeout_ms: u64,
}

#[derive(Serialize, Debug)]
struct DiscoverReport {
    command: String,
    cwd: Option<String>,
    states_explored: usize,
    scenarios_generated: usize,
    unique_hashes: usize,
    out_dir: String,
    acceptance_spec: String,
    trace_file: String,
    risky_actions_blocked: Vec<String>,
}

#[derive(Serialize, Debug)]
struct VerifyScenarioResult {
    name: String,
    passed: bool,
    failed_step: Option<usize>,
    message: Option<String>,
}

#[derive(Serialize, Debug)]
struct VerifyReport {
    spec_path: String,
    out_dir: String,
    total_scenarios: usize,
    passed_scenarios: usize,
    failed_scenarios: usize,
    results: Vec<VerifyScenarioResult>,
}

#[derive(Serialize, Debug)]
struct TraceRecord {
    timestamp: String,
    path: Vec<String>,
    depth: usize,
    state_hash: String,
    anchor: Option<String>,
    error: Option<String>,
}

trait Runner {
    fn spawn(
        &mut self,
        command: &str,
        cwd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<String, ExplorerError>;
    fn press(&mut self, session_id: &str, key: &str) -> Result<(), ExplorerError>;
    fn type_text(&mut self, session_id: &str, text: &str) -> Result<(), ExplorerError>;
    fn wait_stable(&mut self, session_id: &str, timeout_ms: u64) -> Result<(), ExplorerError>;
    fn wait_for_text(
        &mut self,
        session_id: &str,
        text: &str,
        timeout_ms: u64,
    ) -> Result<bool, ExplorerError>;
    fn screenshot(&mut self, session_id: &str) -> Result<(String, Cursor), ExplorerError>;
    fn kill(&mut self, session_id: &str) -> Result<(), ExplorerError>;
}

#[derive(Clone, Debug, Default)]
struct Cursor {
    row: i64,
    col: i64,
    visible: bool,
}

#[derive(Debug)]
struct AgentTuiRunner {
    executable: String,
    base_args: Vec<String>,
    command_cwd: Option<PathBuf>,
}

#[derive(Debug)]
struct RawCommandOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

impl AgentTuiRunner {
    fn new(root: &Path) -> Self {
        if let Ok(bin) = env::var("AGENT_TUI_BIN")
            && !bin.trim().is_empty()
        {
            return Self {
                executable: bin,
                base_args: Vec::new(),
                command_cwd: None,
            };
        }

        let cli_manifest = root.join("Cargo.toml");
        if cli_manifest.exists() {
            return Self {
                executable: "cargo".to_string(),
                base_args: vec![
                    "run".to_string(),
                    "-q".to_string(),
                    "-p".to_string(),
                    "agent-tui".to_string(),
                    "--bin".to_string(),
                    "agent-tui".to_string(),
                    "--".to_string(),
                ],
                command_cwd: Some(root.to_path_buf()),
            };
        }

        Self {
            executable: "agent-tui".to_string(),
            base_args: Vec::new(),
            command_cwd: None,
        }
    }

    fn run_raw(
        &self,
        args: &[String],
        allow_failure: bool,
    ) -> Result<RawCommandOutput, ExplorerError> {
        let mut command = Command::new(&self.executable);
        for arg in &self.base_args {
            command.arg(arg);
        }
        command.arg("--json");
        for arg in args {
            command.arg(arg);
        }
        if let Some(cwd) = &self.command_cwd {
            command.current_dir(cwd);
        }
        if env::var("NO_COLOR").ok().as_deref() == Some("1") {
            command.env("NO_COLOR", "true");
        }

        command.stdin(Stdio::null());

        let output = command.output().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ExplorerError::unavailable("agent-tui not found in PATH")
            } else {
                ExplorerError::scenario(format!("failed to execute agent-tui: {error}"))
            }
        })?;

        let status = output.status.code().unwrap_or(1);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !allow_failure && status != 0 {
            if is_unavailable_text(&stdout, &stderr) {
                return Err(ExplorerError::unavailable("daemon unavailable"));
            }
            return Err(ExplorerError::scenario(format!(
                "agent-tui command failed (status {status}): {}",
                join_args(args)
            )));
        }

        Ok(RawCommandOutput {
            status,
            stdout,
            stderr,
        })
    }

    fn run_json(&self, args: &[String]) -> Result<Value, ExplorerError> {
        let output = self.run_raw(args, false)?;
        parse_json_payload(&output.stdout)
    }
}

impl Runner for AgentTuiRunner {
    fn spawn(
        &mut self,
        command: &str,
        cwd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<String, ExplorerError> {
        let mut args = vec![
            "run".to_string(),
            "--cols".to_string(),
            cols.to_string(),
            "--rows".to_string(),
            rows.to_string(),
        ];
        if let Some(path) = cwd {
            args.push("-d".to_string());
            args.push(path.to_string());
        }
        args.push("sh".to_string());
        args.push("--".to_string());
        args.push("-lc".to_string());
        args.push(command.to_string());

        let payload = self.run_json(&args)?;
        let session_id = payload
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ExplorerError::scenario("spawn response did not include a session_id")
            })?;
        if session_id.trim().is_empty() {
            return Err(ExplorerError::scenario(
                "spawn response included an empty session_id",
            ));
        }
        Ok(session_id.to_string())
    }

    fn press(&mut self, session_id: &str, key: &str) -> Result<(), ExplorerError> {
        let args = vec![
            "-s".to_string(),
            session_id.to_string(),
            "press".to_string(),
            key.to_string(),
        ];
        let _ = self.run_json(&args)?;
        Ok(())
    }

    fn type_text(&mut self, session_id: &str, text: &str) -> Result<(), ExplorerError> {
        let args = vec![
            "-s".to_string(),
            session_id.to_string(),
            "type".to_string(),
            text.to_string(),
        ];
        let _ = self.run_json(&args)?;
        Ok(())
    }

    fn wait_stable(&mut self, session_id: &str, timeout_ms: u64) -> Result<(), ExplorerError> {
        let args = vec![
            "-s".to_string(),
            session_id.to_string(),
            "wait".to_string(),
            "--stable".to_string(),
            "-t".to_string(),
            timeout_ms.to_string(),
        ];
        let _ = self.run_json(&args)?;
        Ok(())
    }

    fn wait_for_text(
        &mut self,
        session_id: &str,
        text: &str,
        timeout_ms: u64,
    ) -> Result<bool, ExplorerError> {
        let args = vec![
            "-s".to_string(),
            session_id.to_string(),
            "wait".to_string(),
            "--assert".to_string(),
            text.to_string(),
            "-t".to_string(),
            timeout_ms.to_string(),
        ];

        let output = self.run_raw(&args, true)?;
        if output.status == 0 {
            let payload = parse_json_payload(&output.stdout)?;
            return Ok(payload
                .get("found")
                .and_then(Value::as_bool)
                .unwrap_or(true));
        }

        if is_unavailable_text(&output.stdout, &output.stderr) {
            return Err(ExplorerError::unavailable("daemon unavailable"));
        }

        Ok(false)
    }

    fn screenshot(&mut self, session_id: &str) -> Result<(String, Cursor), ExplorerError> {
        let args = vec![
            "-s".to_string(),
            session_id.to_string(),
            "screenshot".to_string(),
            "--strip-ansi".to_string(),
            "--include-cursor".to_string(),
        ];

        let payload = self.run_json(&args)?;
        let screenshot = payload
            .get("screenshot")
            .and_then(Value::as_str)
            .ok_or_else(|| ExplorerError::scenario("screenshot response missing screenshot field"))?
            .to_string();

        let cursor = payload
            .get("cursor")
            .and_then(Value::as_object)
            .map(|cursor| Cursor {
                row: cursor.get("row").and_then(Value::as_i64).unwrap_or(0),
                col: cursor.get("col").and_then(Value::as_i64).unwrap_or(0),
                visible: cursor
                    .get("visible")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
            .unwrap_or_default();

        Ok((screenshot, cursor))
    }

    fn kill(&mut self, session_id: &str) -> Result<(), ExplorerError> {
        let args = vec!["-s".to_string(), session_id.to_string(), "kill".to_string()];
        let _ = self.run_raw(&args, true)?;
        Ok(())
    }
}

pub fn run(root: &Path, command: TuiExplorerCommands) -> i32 {
    let result = match command {
        TuiExplorerCommands::Discover(args) => run_discover(root, args),
        TuiExplorerCommands::Verify(args) => run_verify(root, args),
    };

    match result {
        Ok(()) => EXIT_SUCCESS,
        Err(error) => {
            eprintln!("{}", error.message);
            match error.kind {
                ExplorerErrorKind::Spec => EXIT_SPEC_ERROR,
                ExplorerErrorKind::Scenario => EXIT_SCENARIO_FAILED,
                ExplorerErrorKind::Unavailable => EXIT_UNAVAILABLE,
            }
        }
    }
}

fn run_discover(root: &Path, args: DiscoverArgs) -> Result<(), ExplorerError> {
    let out_dir = ensure_out_dir(args.out.unwrap_or_else(default_out_dir))?;
    let config = DiscoverConfig {
        command: args.command,
        cwd: args.cwd,
        cols: args.cols,
        rows: args.rows,
        max_depth: args.max_depth,
        max_states: args.max_states,
        branch_limit: args.branch_limit,
        time_budget_sec: args.time_budget_sec,
        out_dir,
        allow_risky: args.allow_risky,
        default_timeout_ms: 3000,
    };

    let mut runner = AgentTuiRunner::new(root);
    let (report, _spec, _traces) = discover_with_runner(&config, &mut runner)?;
    let report_path = write_discover_report(&config.out_dir, &report)?;

    print_json(&report)?;
    println!("discover report: {}", report_path.display());

    Ok(())
}

fn run_verify(root: &Path, args: VerifyArgs) -> Result<(), ExplorerError> {
    let spec = parse_spec_file(&args.spec)?;
    let out_dir = ensure_out_dir(args.out.unwrap_or_else(default_out_dir))?;

    let mut runner = AgentTuiRunner::new(root);
    let report = verify_with_runner(
        &spec,
        &args.spec,
        &mut runner,
        &out_dir,
        args.scenario.as_deref(),
        args.fail_fast,
    )?;

    let report_path = write_verify_report(&out_dir, &report)?;
    print_json(&report)?;
    println!("verify report: {}", report_path.display());

    if report.failed_scenarios > 0 {
        return Err(ExplorerError::scenario(
            "verify completed with failing scenarios",
        ));
    }

    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), ExplorerError> {
    let output = serde_json::to_string_pretty(value).map_err(|error| {
        ExplorerError::scenario(format!("failed to serialize JSON output: {error}"))
    })?;
    println!("{output}");
    Ok(())
}

fn default_out_dir() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    PathBuf::from(".agent-tui").join("discover").join(stamp)
}

fn ensure_out_dir(path: PathBuf) -> Result<PathBuf, ExplorerError> {
    fs::create_dir_all(&path).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to create output directory {}: {error}",
            path.display()
        ))
    })?;
    Ok(path)
}

fn parse_json_payload(stdout: &str) -> Result<Value, ExplorerError> {
    let payload = stdout.trim();
    if payload.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str::<Value>(payload).map_err(|error| {
        ExplorerError::scenario(format!("invalid JSON response from agent-tui: {error}"))
    })
}

fn join_args(args: &[String]) -> String {
    args.join(" ")
}

fn is_unavailable_text(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{}\n{}", stdout.to_lowercase(), stderr.to_lowercase());
    combined.contains("daemon not running") || combined.contains("connection refused")
}

fn strip_ansi(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        if chars.peek() == Some(&'[') {
            let _ = chars.next();
            for code in chars.by_ref() {
                let byte = code as u32;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
            continue;
        }

        let _ = chars.next();
    }

    out
}

fn normalize_screenshot(value: &str) -> String {
    strip_ansi(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn compute_state_hash(screenshot: &str, cursor: &Cursor, cols: u16, rows: u16) -> String {
    let normalized = normalize_screenshot(screenshot);
    let visible = if cursor.visible { 1 } else { 0 };
    let payload = format!(
        "{normalized}|{}:{}:{visible}|{cols}:{rows}",
        cursor.row, cursor.col
    );

    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    let digest = hasher.finalize();
    format!("{digest:x}")
}

fn contains_clock_like_token(line: &str) -> bool {
    for token in line.split_whitespace() {
        let trimmed = token.trim_matches(|ch: char| !ch.is_ascii_digit() && ch != ':');
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split(':');
        let hour = parts.next();
        let minute = parts.next();
        let extra = parts.next();

        if let (Some(hour), Some(minute), None) = (hour, minute, extra) {
            let hour_ok =
                (hour.len() == 1 || hour.len() == 2) && hour.chars().all(|ch| ch.is_ascii_digit());
            let minute_ok = minute.len() == 2 && minute.chars().all(|ch| ch.is_ascii_digit());
            if hour_ok && minute_ok {
                return true;
            }
        }
    }

    false
}

fn pick_anchor(screenshot: &str) -> Option<String> {
    for raw in screenshot.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() < 3 || line.len() > 120 {
            continue;
        }
        if contains_clock_like_token(line) {
            continue;
        }

        let digits = line.chars().filter(|ch| ch.is_ascii_digit()).count();
        let ratio = digits as f64 / line.len() as f64;
        if ratio > 0.45 {
            continue;
        }

        if line.chars().any(|ch| ch.is_ascii_alphabetic()) {
            return Some(line.to_string());
        }
    }

    for raw in screenshot.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let anchor = if line.len() > 80 { &line[..80] } else { line };
        return Some(anchor.to_string());
    }

    None
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn parse_scalar(raw_value: &str) -> Result<Value, ExplorerError> {
    let value = raw_value.trim();
    if value.starts_with('"') && value.ends_with('"') {
        let parsed = serde_json::from_str::<String>(value).map_err(|error| {
            ExplorerError::spec(format!("invalid quoted scalar '{value}': {error}"))
        })?;
        return Ok(Value::String(parsed));
    }

    if value == "true" {
        return Ok(Value::Bool(true));
    }
    if value == "false" {
        return Ok(Value::Bool(false));
    }
    if value == "null" {
        return Ok(Value::Null);
    }

    if let Ok(parsed) = value.parse::<i64>() {
        return Ok(Value::Number(parsed.into()));
    }

    Ok(Value::String(value.to_string()))
}

fn parse_frontmatter_and_body(
    text: &str,
) -> Result<(BTreeMap<String, Value>, Vec<String>), ExplorerError> {
    let lines = text
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();
    if lines.is_empty() || lines[0].trim() != "---" {
        return Err(ExplorerError::spec(
            "spec must start with frontmatter delimiter '---'",
        ));
    }

    let mut end_idx: Option<usize> = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(index);
            break;
        }
    }

    let Some(end_idx) = end_idx else {
        return Err(ExplorerError::spec("frontmatter is not closed with '---'"));
    };

    let mut frontmatter = BTreeMap::new();
    for line in lines.iter().take(end_idx).skip(1) {
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        let Some((key, value)) = stripped.split_once(':') else {
            return Err(ExplorerError::spec(format!(
                "invalid frontmatter line: {line}"
            )));
        };
        frontmatter.insert(key.trim().to_string(), parse_scalar(value)?);
    }

    Ok((frontmatter, lines[(end_idx + 1)..].to_vec()))
}

fn parse_quoted_step(raw_value: &str, kind: &str, line: &str) -> Result<String, ExplorerError> {
    let value = raw_value.trim();
    if !value.starts_with('"') || !value.ends_with('"') {
        return Err(ExplorerError::spec(format!(
            "{kind} step must use a double-quoted string: {line}"
        )));
    }

    serde_json::from_str::<String>(value).map_err(|error| {
        ExplorerError::spec(format!("invalid quoted value in step '{line}': {error}"))
    })
}

fn validate_frontmatter(frontmatter: &BTreeMap<String, Value>) -> Result<(), ExplorerError> {
    for required in [
        "schema_version",
        "command",
        "cols",
        "rows",
        "default_timeout_ms",
        "generated_at",
        "generator",
    ] {
        if !frontmatter.contains_key(required) {
            return Err(ExplorerError::spec(format!(
                "missing required frontmatter key: {required}"
            )));
        }
    }

    if frontmatter.get("schema_version").and_then(Value::as_str) != Some("v1") {
        return Err(ExplorerError::spec(
            "unsupported schema_version, expected 'v1'",
        ));
    }

    let command = frontmatter
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if command.trim().is_empty() {
        return Err(ExplorerError::spec(
            "frontmatter 'command' must be a non-empty string",
        ));
    }

    for field in ["cols", "rows", "default_timeout_ms"] {
        let value = frontmatter.get(field).and_then(Value::as_i64).unwrap_or(0);
        if value <= 0 {
            return Err(ExplorerError::spec(format!(
                "frontmatter '{field}' must be a positive integer"
            )));
        }
    }

    if let Some(cwd) = frontmatter.get("cwd")
        && !cwd.is_null()
        && cwd.as_str().is_none()
    {
        return Err(ExplorerError::spec(
            "frontmatter 'cwd' must be a string when present",
        ));
    }

    Ok(())
}

fn parse_spec_text(text: &str) -> Result<Spec, ExplorerError> {
    let (frontmatter, body) = parse_frontmatter_and_body(text)?;
    validate_frontmatter(&frontmatter)?;

    let mut scenarios: Vec<Scenario> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_steps: Vec<Step> = Vec::new();

    let flush_current = |scenarios: &mut Vec<Scenario>,
                         current_name: &mut Option<String>,
                         current_steps: &mut Vec<Step>|
     -> Result<(), ExplorerError> {
        let Some(name) = current_name.clone() else {
            return Ok(());
        };
        if current_steps.is_empty() {
            return Err(ExplorerError::spec(format!(
                "scenario '{name}' has no steps"
            )));
        }
        scenarios.push(Scenario {
            name,
            steps: current_steps.clone(),
        });
        *current_name = None;
        current_steps.clear();
        Ok(())
    };

    for raw in body {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("## Scenario:") {
            flush_current(&mut scenarios, &mut current_name, &mut current_steps)?;
            let name = rest.trim();
            if name.is_empty() {
                return Err(ExplorerError::spec("scenario header has empty name"));
            }
            current_name = Some(name.to_string());
            continue;
        }

        if current_name.is_none() {
            return Err(ExplorerError::spec(format!(
                "unexpected content before first scenario: {line}"
            )));
        }

        if line == "- wait_stable: true" {
            current_steps.push(Step::WaitStable);
            continue;
        }

        if let Some(rest) = line.strip_prefix("- expect:") {
            let value = parse_quoted_step(rest, "expect", line)?;
            current_steps.push(Step::Expect(value));
            continue;
        }

        if let Some(rest) = line.strip_prefix("- press:") {
            let value = parse_quoted_step(rest, "press", line)?;
            current_steps.push(Step::Press(value));
            continue;
        }

        if let Some(rest) = line.strip_prefix("- type:") {
            let value = parse_quoted_step(rest, "type", line)?;
            current_steps.push(Step::Type(value));
            continue;
        }

        return Err(ExplorerError::spec(format!("invalid step syntax: {line}")));
    }

    flush_current(&mut scenarios, &mut current_name, &mut current_steps)?;

    if scenarios.is_empty() {
        return Err(ExplorerError::spec("spec contains no scenarios"));
    }

    Ok(Spec {
        frontmatter,
        scenarios,
    })
}

fn parse_spec_file(path: &Path) -> Result<Spec, ExplorerError> {
    let text = fs::read_to_string(path).map_err(|error| {
        ExplorerError::spec(format!(
            "failed to read spec file {}: {error}",
            path.display()
        ))
    })?;
    parse_spec_text(&text)
}

fn scalar_to_markdown(value: &Value) -> Result<String, ExplorerError> {
    match value {
        Value::String(text) => serde_json::to_string(text).map_err(|error| {
            ExplorerError::spec(format!("failed to render string scalar: {error}"))
        }),
        Value::Null => Ok("null".to_string()),
        Value::Bool(boolean) => Ok(if *boolean {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        Value::Number(number) => Ok(number.to_string()),
        _ => Err(ExplorerError::spec("unsupported frontmatter value type")),
    }
}

fn render_markdown(spec: &Spec) -> Result<String, ExplorerError> {
    let mut lines = vec!["---".to_string()];

    for key in [
        "schema_version",
        "command",
        "cwd",
        "cols",
        "rows",
        "default_timeout_ms",
        "generated_at",
        "generator",
    ] {
        if let Some(value) = spec.frontmatter.get(key) {
            lines.push(format!("{key}: {}", scalar_to_markdown(value)?));
        }
    }

    lines.push("---".to_string());
    lines.push(String::new());

    for scenario in &spec.scenarios {
        lines.push(format!("## Scenario: {}", scenario.name));
        for step in &scenario.steps {
            match step {
                Step::WaitStable => lines.push("- wait_stable: true".to_string()),
                Step::Expect(value) => lines.push(format!(
                    "- expect: {}",
                    serde_json::to_string(value).map_err(|error| ExplorerError::spec(format!(
                        "failed to render expect step: {error}"
                    )))?
                )),
                Step::Press(value) => lines.push(format!(
                    "- press: {}",
                    serde_json::to_string(value).map_err(|error| ExplorerError::spec(format!(
                        "failed to render press step: {error}"
                    )))?
                )),
                Step::Type(value) => lines.push(format!(
                    "- type: {}",
                    serde_json::to_string(value).map_err(|error| ExplorerError::spec(format!(
                        "failed to render type step: {error}"
                    )))?
                )),
            }
        }
        lines.push(String::new());
    }

    Ok(format!("{}\n", lines.join("\n").trim_end()))
}

fn scenario_name(path: &[String], index: usize) -> String {
    if path.is_empty() {
        return format!("Path {index}: root");
    }
    let compact = path
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(" -> ");
    format!("Path {index}: {compact}")
}

fn steps_for_action(action: &str) -> Vec<Step> {
    vec![Step::Press(action.to_string()), Step::WaitStable]
}

fn discover_with_runner<R: Runner>(
    config: &DiscoverConfig,
    runner: &mut R,
) -> Result<(DiscoverReport, Spec, Vec<TraceRecord>), ExplorerError> {
    let mut actions = DEFAULT_SAFE_ACTIONS
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();

    let blocked_risky_actions = if config.allow_risky {
        actions.extend(
            DEFAULT_RISKY_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<_>>(),
        );
        Vec::new()
    } else {
        DEFAULT_RISKY_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect::<Vec<_>>()
    };

    let timeout = Duration::from_secs(config.time_budget_sec);
    let start = Instant::now();

    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    queue.push_back(Vec::new());

    let mut visited_hashes = HashSet::new();
    let mut traces = Vec::new();
    let mut scenarios = Vec::new();
    let mut states_explored = 0usize;

    while let Some(path) = queue.pop_front() {
        if states_explored >= config.max_states {
            break;
        }
        if start.elapsed() >= timeout {
            break;
        }

        states_explored += 1;

        let mut session_id: Option<String> = None;
        let mut state_hash = String::new();
        let mut anchor: Option<String> = None;
        let mut error_message: Option<String> = None;

        let execution = (|| -> Result<(), ExplorerError> {
            let id = runner.spawn(
                &config.command,
                config.cwd.as_deref(),
                config.cols,
                config.rows,
            )?;
            session_id = Some(id.clone());

            for action in &path {
                runner.press(&id, action)?;
                runner.wait_stable(&id, config.default_timeout_ms.min(1000))?;
            }

            runner.wait_stable(&id, config.default_timeout_ms.min(1000))?;
            let (screenshot, cursor) = runner.screenshot(&id)?;
            state_hash = compute_state_hash(&screenshot, &cursor, config.cols, config.rows);
            anchor = pick_anchor(&screenshot);

            Ok(())
        })();

        if let Some(id) = session_id.as_ref() {
            let _ = runner.kill(id);
        }

        if let Err(error) = execution {
            if error.kind == ExplorerErrorKind::Unavailable {
                return Err(error);
            }
            error_message = Some(error.to_string());
        }

        traces.push(TraceRecord {
            timestamp: now_rfc3339(),
            path: path.clone(),
            depth: path.len(),
            state_hash: state_hash.clone(),
            anchor: anchor.clone(),
            error: error_message.clone(),
        });

        if error_message.is_some() {
            continue;
        }

        if visited_hashes.contains(&state_hash) {
            continue;
        }
        visited_hashes.insert(state_hash);

        if !path.is_empty()
            && let Some(anchor) = anchor
        {
            let mut steps = Vec::new();
            for action in &path {
                steps.extend(steps_for_action(action));
            }
            steps.push(Step::Expect(anchor));
            scenarios.push(Scenario {
                name: scenario_name(&path, scenarios.len() + 1),
                steps,
            });
        }

        if path.len() >= config.max_depth {
            continue;
        }

        for action in actions.iter().take(config.branch_limit) {
            let mut next = path.clone();
            next.push(action.clone());
            queue.push_back(next);
        }
    }

    let generated_at = now_rfc3339();
    let spec = Spec {
        frontmatter: BTreeMap::from([
            (
                "schema_version".to_string(),
                Value::String("v1".to_string()),
            ),
            ("command".to_string(), Value::String(config.command.clone())),
            (
                "cwd".to_string(),
                config
                    .cwd
                    .as_ref()
                    .map(|cwd| Value::String(cwd.clone()))
                    .unwrap_or(Value::Null),
            ),
            (
                "cols".to_string(),
                Value::Number(i64::from(config.cols).into()),
            ),
            (
                "rows".to_string(),
                Value::Number(i64::from(config.rows).into()),
            ),
            (
                "default_timeout_ms".to_string(),
                Value::Number((config.default_timeout_ms as i64).into()),
            ),
            ("generated_at".to_string(), Value::String(generated_at)),
            (
                "generator".to_string(),
                Value::String("tui-explorer/1".to_string()),
            ),
        ]),
        scenarios,
    };

    let acceptance_path = config.out_dir.join("acceptance.md");
    let trace_path = config.out_dir.join("trace.jsonl");

    let markdown = render_markdown(&spec)?;
    fs::write(&acceptance_path, markdown).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to write acceptance spec {}: {error}",
            acceptance_path.display()
        ))
    })?;

    let mut trace_lines = String::new();
    for trace in &traces {
        let line = serde_json::to_string(trace).map_err(|error| {
            ExplorerError::scenario(format!("failed to serialize trace: {error}"))
        })?;
        trace_lines.push_str(&line);
        trace_lines.push('\n');
    }

    fs::write(&trace_path, trace_lines).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to write trace file {}: {error}",
            trace_path.display()
        ))
    })?;

    let report = DiscoverReport {
        command: config.command.clone(),
        cwd: config.cwd.clone(),
        states_explored,
        scenarios_generated: spec.scenarios.len(),
        unique_hashes: visited_hashes.len(),
        out_dir: config.out_dir.display().to_string(),
        acceptance_spec: acceptance_path.display().to_string(),
        trace_file: trace_path.display().to_string(),
        risky_actions_blocked: blocked_risky_actions,
    };

    Ok((report, spec, traces))
}

fn value_string(frontmatter: &BTreeMap<String, Value>, key: &str) -> Result<String, ExplorerError> {
    let Some(value) = frontmatter.get(key) else {
        return Err(ExplorerError::spec(format!(
            "missing frontmatter key: {key}"
        )));
    };
    let Some(parsed) = value.as_str() else {
        return Err(ExplorerError::spec(format!(
            "frontmatter '{key}' must be a string"
        )));
    };
    Ok(parsed.to_string())
}

fn value_optional_string(
    frontmatter: &BTreeMap<String, Value>,
    key: &str,
) -> Result<Option<String>, ExplorerError> {
    let Some(value) = frontmatter.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(parsed) = value.as_str() else {
        return Err(ExplorerError::spec(format!(
            "frontmatter '{key}' must be a string"
        )));
    };
    Ok(Some(parsed.to_string()))
}

fn value_u64(frontmatter: &BTreeMap<String, Value>, key: &str) -> Result<u64, ExplorerError> {
    let Some(value) = frontmatter.get(key) else {
        return Err(ExplorerError::spec(format!(
            "missing frontmatter key: {key}"
        )));
    };
    let Some(parsed) = value.as_u64() else {
        return Err(ExplorerError::spec(format!(
            "frontmatter '{key}' must be a positive integer"
        )));
    };
    if parsed == 0 {
        return Err(ExplorerError::spec(format!(
            "frontmatter '{key}' must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn verify_with_runner<R: Runner>(
    spec: &Spec,
    spec_path: &Path,
    runner: &mut R,
    out_dir: &Path,
    scenario_filter: Option<&str>,
    fail_fast: bool,
) -> Result<VerifyReport, ExplorerError> {
    let target_scenarios = if let Some(name) = scenario_filter {
        let filtered = spec
            .scenarios
            .iter()
            .filter(|scenario| scenario.name == name)
            .cloned()
            .collect::<Vec<_>>();
        if filtered.is_empty() {
            return Err(ExplorerError::spec(format!(
                "scenario '{name}' not found in spec"
            )));
        }
        filtered
    } else {
        spec.scenarios.clone()
    };

    let failure_dir = out_dir.join("failures");
    fs::create_dir_all(&failure_dir).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to create failure directory {}: {error}",
            failure_dir.display()
        ))
    })?;

    let timeout_ms = value_u64(&spec.frontmatter, "default_timeout_ms")?;
    let command = value_string(&spec.frontmatter, "command")?;
    let cwd = value_optional_string(&spec.frontmatter, "cwd")?;
    let cols = value_u64(&spec.frontmatter, "cols")? as u16;
    let rows = value_u64(&spec.frontmatter, "rows")? as u16;

    let mut results = Vec::new();

    for scenario in target_scenarios {
        let mut scenario_result = VerifyScenarioResult {
            name: scenario.name.clone(),
            passed: true,
            failed_step: None,
            message: None,
        };
        let mut session_id: Option<String> = None;
        let mut failing_step_index: Option<usize> = None;

        let execution = (|| -> Result<(), ExplorerError> {
            let id = runner.spawn(&command, cwd.as_deref(), cols, rows)?;
            session_id = Some(id.clone());

            for (index, step) in scenario.steps.iter().enumerate() {
                let step_number = index + 1;
                failing_step_index = Some(step_number);
                match step {
                    Step::Press(key) => runner.press(&id, key)?,
                    Step::Type(text) => runner.type_text(&id, text)?,
                    Step::WaitStable => {
                        runner.wait_stable(&id, timeout_ms)?;
                        continue;
                    }
                    Step::Expect(text) => {
                        let found = runner.wait_for_text(&id, text, timeout_ms)?;
                        if !found {
                            return Err(ExplorerError::scenario(format!(
                                "expectation not met: {text}"
                            )));
                        }
                    }
                }

                runner.wait_stable(&id, timeout_ms).map_err(|error| {
                    ExplorerError::scenario(format!(
                        "post-step stabilization failed at step {step_number}: {error}"
                    ))
                })?;
            }

            Ok(())
        })();

        if let Some(id) = session_id.as_ref() {
            let _ = runner.kill(id);
        }

        if let Err(error) = execution {
            if error.kind == ExplorerErrorKind::Unavailable {
                return Err(error);
            }
            scenario_result.passed = false;
            let failed_step = failing_step_index.unwrap_or(0);
            scenario_result.failed_step = Some(failed_step);
            scenario_result.message = Some(error.to_string());

            let failure_path = failure_dir.join(format!(
                "{}-step-{}.txt",
                slugify(&scenario.name),
                scenario_result.failed_step.unwrap_or(0)
            ));
            let failure_text = format!(
                "scenario: {}\nstep: {}\nerror: {}\n",
                scenario.name,
                scenario_result.failed_step.unwrap_or(0),
                scenario_result.message.clone().unwrap_or_default()
            );
            fs::write(&failure_path, failure_text).map_err(|write_error| {
                ExplorerError::scenario(format!(
                    "failed to write failure artifact {}: {write_error}",
                    failure_path.display()
                ))
            })?;
        }

        let stop_on_failure = fail_fast && !scenario_result.passed;
        results.push(scenario_result);
        if stop_on_failure {
            break;
        }
    }

    let passed = results.iter().filter(|result| result.passed).count();
    let failed = results.len().saturating_sub(passed);

    Ok(VerifyReport {
        spec_path: spec_path.display().to_string(),
        out_dir: out_dir.display().to_string(),
        total_scenarios: results.len(),
        passed_scenarios: passed,
        failed_scenarios: failed,
        results,
    })
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
            continue;
        }

        if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "scenario".to_string()
    } else {
        slug
    }
}

fn write_discover_report(
    out_dir: &Path,
    report: &DiscoverReport,
) -> Result<PathBuf, ExplorerError> {
    let path = out_dir.join("discover-report.json");
    let mut serialized = serde_json::to_string_pretty(report).map_err(|error| {
        ExplorerError::scenario(format!("failed to serialize discover report: {error}"))
    })?;
    serialized.push('\n');

    fs::write(&path, serialized).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to write discover report {}: {error}",
            path.display()
        ))
    })?;

    Ok(path)
}

fn write_verify_report(out_dir: &Path, report: &VerifyReport) -> Result<PathBuf, ExplorerError> {
    let path = out_dir.join("verify-report.json");
    let mut serialized = serde_json::to_string_pretty(report).map_err(|error| {
        ExplorerError::scenario(format!("failed to serialize verify report: {error}"))
    })?;
    serialized.push('\n');

    fs::write(&path, serialized).map_err(|error| {
        ExplorerError::scenario(format!(
            "failed to write verify report {}: {error}",
            path.display()
        ))
    })?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    const VALID_SPEC: &str = "---\nschema_version: \"v1\"\ncommand: \"printf app\"\ncwd: \"/tmp\"\ncols: 120\nrows: 40\ndefault_timeout_ms: 3000\ngenerated_at: \"2026-02-18T00:00:00Z\"\ngenerator: \"tui-explorer/1\"\n---\n\n## Scenario: Basic\n- wait_stable: true\n- press: \"Enter\"\n- expect: \"OK\"\n";

    #[derive(Default)]
    struct FakeRunner {
        screen_by_path: HashMap<Vec<String>, String>,
        current_path: Vec<String>,
        spawn_count: usize,
        kill_count: usize,
    }

    impl FakeRunner {
        fn new(screen_by_path: HashMap<Vec<String>, String>) -> Self {
            Self {
                screen_by_path,
                current_path: Vec::new(),
                spawn_count: 0,
                kill_count: 0,
            }
        }

        fn screen(&self) -> String {
            self.screen_by_path
                .get(&self.current_path)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl Runner for FakeRunner {
        fn spawn(
            &mut self,
            _command: &str,
            _cwd: Option<&str>,
            _cols: u16,
            _rows: u16,
        ) -> Result<String, ExplorerError> {
            self.spawn_count += 1;
            self.current_path.clear();
            Ok(format!("session-{}", self.spawn_count))
        }

        fn press(&mut self, _session_id: &str, key: &str) -> Result<(), ExplorerError> {
            self.current_path.push(key.to_string());
            Ok(())
        }

        fn type_text(&mut self, _session_id: &str, text: &str) -> Result<(), ExplorerError> {
            self.current_path.push(format!("TYPE:{text}"));
            Ok(())
        }

        fn wait_stable(
            &mut self,
            _session_id: &str,
            _timeout_ms: u64,
        ) -> Result<(), ExplorerError> {
            Ok(())
        }

        fn wait_for_text(
            &mut self,
            _session_id: &str,
            text: &str,
            _timeout_ms: u64,
        ) -> Result<bool, ExplorerError> {
            Ok(self.screen().contains(text))
        }

        fn screenshot(&mut self, _session_id: &str) -> Result<(String, Cursor), ExplorerError> {
            Ok((self.screen(), Cursor::default()))
        }

        fn kill(&mut self, _session_id: &str) -> Result<(), ExplorerError> {
            self.kill_count += 1;
            Ok(())
        }
    }

    fn assert_ok<T>(result: Result<T, ExplorerError>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("expected Ok(_), got error: {error}"),
        }
    }

    fn as_map(entries: Vec<(Vec<String>, &str)>) -> HashMap<Vec<String>, String> {
        entries
            .into_iter()
            .map(|(path, screen)| (path, screen.to_string()))
            .collect()
    }

    #[test]
    fn parse_valid_v1_spec() {
        let spec = assert_ok(parse_spec_text(VALID_SPEC));
        assert_eq!(
            spec.frontmatter
                .get("schema_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert_eq!(spec.scenarios.len(), 1);
        assert!(matches!(spec.scenarios[0].steps[1], Step::Press(_)));
    }

    #[test]
    fn parse_missing_required_frontmatter() {
        let bad = VALID_SPEC.replace("rows: 40\n", "");
        let result = parse_spec_text(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_step_syntax() {
        let bad = VALID_SPEC.replace("- press: \"Enter\"", "- press: Enter");
        let result = parse_spec_text(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn parse_unsupported_schema_version() {
        let bad = VALID_SPEC.replace("schema_version: \"v1\"", "schema_version: \"v2\"");
        let result = parse_spec_text(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn discovery_dedupes_hashes_and_respects_limits() {
        let screens = as_map(vec![
            (Vec::new(), "Main Menu"),
            (vec!["Enter".to_string()], "Main Menu"),
            (vec!["Tab".to_string()], "Settings"),
        ]);
        let mut runner = FakeRunner::new(screens);
        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let config = DiscoverConfig {
            command: "printf app".to_string(),
            cwd: None,
            cols: 120,
            rows: 40,
            max_depth: 1,
            max_states: 10,
            branch_limit: 2,
            time_budget_sec: 30,
            out_dir: temp.path().to_path_buf(),
            allow_risky: false,
            default_timeout_ms: 3000,
        };

        let (report, spec, traces) = assert_ok(discover_with_runner(&config, &mut runner));
        assert!(report.states_explored >= 1);
        assert_eq!(report.unique_hashes, 2);
        assert!(traces.len() >= spec.scenarios.len());
    }

    #[test]
    fn discovery_safe_allowlist_only() {
        let screens = as_map(vec![
            (Vec::new(), "Menu"),
            (vec!["Enter".to_string()], "Menu Enter"),
            (vec!["Tab".to_string()], "Menu Tab"),
        ]);
        let mut runner = FakeRunner::new(screens);
        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let config = DiscoverConfig {
            command: "printf app".to_string(),
            cwd: None,
            cols: 120,
            rows: 40,
            max_depth: 1,
            max_states: 5,
            branch_limit: 2,
            time_budget_sec: 30,
            out_dir: temp.path().to_path_buf(),
            allow_risky: false,
            default_timeout_ms: 3000,
        };

        let (_report, spec, _traces) = assert_ok(discover_with_runner(&config, &mut runner));
        for scenario in spec.scenarios {
            for step in scenario.steps {
                if let Step::Press(key) = step {
                    assert!(DEFAULT_SAFE_ACTIONS.contains(&key.as_str()));
                }
            }
        }
    }

    #[test]
    fn discovery_is_deterministic_for_same_input() {
        let screens = as_map(vec![
            (Vec::new(), "Menu"),
            (vec!["Enter".to_string()], "Item A"),
            (vec!["Tab".to_string()], "Item B"),
        ]);

        let temp1 = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };
        let temp2 = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let config1 = DiscoverConfig {
            command: "printf app".to_string(),
            cwd: None,
            cols: 120,
            rows: 40,
            max_depth: 1,
            max_states: 5,
            branch_limit: 2,
            time_budget_sec: 30,
            out_dir: temp1.path().to_path_buf(),
            allow_risky: false,
            default_timeout_ms: 3000,
        };

        let config2 = DiscoverConfig {
            out_dir: temp2.path().to_path_buf(),
            ..config1.clone()
        };

        let (_report1, spec1, _traces1) = assert_ok(discover_with_runner(
            &config1,
            &mut FakeRunner::new(screens.clone()),
        ));
        let (_report2, spec2, _traces2) = assert_ok(discover_with_runner(
            &config2,
            &mut FakeRunner::new(screens),
        ));

        let signature1 = spec1
            .scenarios
            .iter()
            .map(|scenario| {
                let step_signature = scenario
                    .steps
                    .iter()
                    .map(|step| match step {
                        Step::Expect(value) => format!("expect:{value}"),
                        Step::Press(value) => format!("press:{value}"),
                        Step::Type(value) => format!("type:{value}"),
                        Step::WaitStable => "wait_stable:true".to_string(),
                    })
                    .collect::<Vec<_>>();
                (scenario.name.clone(), step_signature)
            })
            .collect::<Vec<_>>();

        let signature2 = spec2
            .scenarios
            .iter()
            .map(|scenario| {
                let step_signature = scenario
                    .steps
                    .iter()
                    .map(|step| match step {
                        Step::Expect(value) => format!("expect:{value}"),
                        Step::Press(value) => format!("press:{value}"),
                        Step::Type(value) => format!("type:{value}"),
                        Step::WaitStable => "wait_stable:true".to_string(),
                    })
                    .collect::<Vec<_>>();
                (scenario.name.clone(), step_signature)
            })
            .collect::<Vec<_>>();

        assert_eq!(signature1, signature2);
    }

    #[test]
    fn markdown_roundtrip_with_escaped_text() {
        let spec = Spec {
            frontmatter: BTreeMap::from([
                (
                    "schema_version".to_string(),
                    Value::String("v1".to_string()),
                ),
                (
                    "command".to_string(),
                    Value::String("printf app".to_string()),
                ),
                ("cwd".to_string(), Value::String("/tmp".to_string())),
                ("cols".to_string(), Value::Number(120.into())),
                ("rows".to_string(), Value::Number(40.into())),
                ("default_timeout_ms".to_string(), Value::Number(3000.into())),
                (
                    "generated_at".to_string(),
                    Value::String("2026-02-18T00:00:00Z".to_string()),
                ),
                (
                    "generator".to_string(),
                    Value::String("tui-explorer/1".to_string()),
                ),
            ]),
            scenarios: vec![Scenario {
                name: "Quoted".to_string(),
                steps: vec![
                    Step::Type("he said \"hello\" \\\\".to_string()),
                    Step::WaitStable,
                    Step::Expect("done".to_string()),
                ],
            }],
        };

        let text = assert_ok(render_markdown(&spec));
        let parsed = assert_ok(parse_spec_text(&text));

        match &parsed.scenarios[0].steps[0] {
            Step::Type(value) => assert_eq!(value, "he said \"hello\" \\\\"),
            _ => panic!("unexpected first step kind"),
        }
    }

    #[test]
    fn verify_happy_path() {
        let spec = assert_ok(parse_spec_text(VALID_SPEC));
        let mut runner = FakeRunner::new(as_map(vec![
            (Vec::new(), "OK"),
            (vec!["Enter".to_string()], "OK"),
        ]));

        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let report = assert_ok(verify_with_runner(
            &spec,
            Path::new("acceptance.md"),
            &mut runner,
            temp.path(),
            None,
            true,
        ));

        assert_eq!(report.failed_scenarios, 0);
        assert_eq!(report.passed_scenarios, 1);
    }

    #[test]
    fn verify_fail_fast_writes_failure_artifact() {
        let spec_text = "---\nschema_version: \"v1\"\ncommand: \"printf app\"\ncols: 120\nrows: 40\ndefault_timeout_ms: 3000\ngenerated_at: \"2026-02-18T00:00:00Z\"\ngenerator: \"tui-explorer/1\"\n---\n\n## Scenario: First\n- expect: \"NOPE\"\n\n## Scenario: Second\n- expect: \"YES\"\n";
        let spec = assert_ok(parse_spec_text(spec_text));
        let mut runner = FakeRunner::new(as_map(vec![(Vec::new(), "YES")]));

        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let report = assert_ok(verify_with_runner(
            &spec,
            Path::new("acceptance.md"),
            &mut runner,
            temp.path(),
            None,
            true,
        ));

        assert_eq!(report.failed_scenarios, 1);
        assert_eq!(report.total_scenarios, 1);

        let failures_dir = temp.path().join("failures");
        let entries = match fs::read_dir(failures_dir) {
            Ok(iter) => iter.collect::<Result<Vec<_>, _>>(),
            Err(error) => panic!("failed to read failures directory: {error}"),
        };
        let entries = match entries {
            Ok(entries) => entries,
            Err(error) => panic!("failed to collect failure entries: {error}"),
        };

        assert!(!entries.is_empty());
    }

    #[test]
    fn verify_scenario_filter() {
        let spec_text = "---\nschema_version: \"v1\"\ncommand: \"printf app\"\ncols: 120\nrows: 40\ndefault_timeout_ms: 3000\ngenerated_at: \"2026-02-18T00:00:00Z\"\ngenerator: \"tui-explorer/1\"\n---\n\n## Scenario: One\n- expect: \"NO\"\n\n## Scenario: Two\n- expect: \"YES\"\n";
        let spec = assert_ok(parse_spec_text(spec_text));
        let mut runner = FakeRunner::new(as_map(vec![(Vec::new(), "YES")]));

        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let report = assert_ok(verify_with_runner(
            &spec,
            Path::new("acceptance.md"),
            &mut runner,
            temp.path(),
            Some("Two"),
            true,
        ));

        assert_eq!(report.total_scenarios, 1);
        assert_eq!(report.failed_scenarios, 0);
    }

    #[test]
    fn risky_actions_are_blocked_without_opt_in_and_reported() {
        let screens = as_map(vec![(Vec::new(), "Menu")]);
        let mut runner = FakeRunner::new(screens);
        let temp = match tempdir() {
            Ok(path) => path,
            Err(error) => panic!("failed to create temp dir: {error}"),
        };

        let config = DiscoverConfig {
            command: "printf app".to_string(),
            cwd: None,
            cols: 120,
            rows: 40,
            max_depth: 0,
            max_states: 1,
            branch_limit: 8,
            time_budget_sec: 5,
            out_dir: temp.path().to_path_buf(),
            allow_risky: false,
            default_timeout_ms: 3000,
        };

        let (report, _spec, _traces) = assert_ok(discover_with_runner(&config, &mut runner));
        assert_eq!(
            report.risky_actions_blocked,
            DEFAULT_RISKY_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn metadata_mentions_discover_and_verify_without_python_dependency() {
        let root = workspace_root();
        let skill_path = root.join("skills/tui-explorer/SKILL.md");
        let text = match fs::read_to_string(&skill_path) {
            Ok(content) => content,
            Err(error) => panic!("failed to read {}: {error}", skill_path.display()),
        };

        assert!(text.contains("discover"));
        assert!(text.contains("verify"));
        assert!(!text.contains("python3"));
    }

    #[test]
    fn metadata_requires_live_preview_start_over_http_endpoint() {
        let root = workspace_root();
        let skill_path = root.join("skills/tui-explorer/SKILL.md");
        let text = match fs::read_to_string(&skill_path) {
            Ok(content) => content,
            Err(error) => panic!("failed to read {}: {error}", skill_path.display()),
        };

        assert!(
            text.contains("Start live preview over the HTTP endpoint"),
            "skill guidance must require starting live preview over the HTTP endpoint"
        );
        assert!(
            text.contains("Treat browser session selection as preview-local"),
            "skill guidance must state browser selection does not switch daemon active session"
        );
    }

    #[test]
    fn metadata_yaml_has_required_fields() {
        let root = workspace_root();
        let metadata_path = root.join("skills/tui-explorer/agents/openai.yaml");
        let text = match fs::read_to_string(&metadata_path) {
            Ok(content) => content,
            Err(error) => panic!("failed to read {}: {error}", metadata_path.display()),
        };

        assert!(text.contains("display_name:"));
        assert!(text.contains("short_description:"));
    }

    #[test]
    fn metadata_yaml_requires_live_preview_start_over_http_endpoint() {
        let root = workspace_root();
        let metadata_path = root.join("skills/tui-explorer/agents/openai.yaml");
        let text = match fs::read_to_string(&metadata_path) {
            Ok(content) => content,
            Err(error) => panic!("failed to read {}: {error}", metadata_path.display()),
        };

        assert!(
            text.contains("Start live preview over the HTTP endpoint"),
            "default prompt must require starting live preview over the HTTP endpoint"
        );
        assert!(
            text.contains("preview-local"),
            "default prompt must mention preview-local session selection"
        );
    }

    fn workspace_root() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let Some(crates) = manifest.parent() else {
            panic!("missing crates directory from manifest path");
        };
        let Some(cli) = crates.parent() else {
            panic!("missing cli directory from manifest path");
        };
        let Some(root) = cli.parent() else {
            panic!("missing repository root from manifest path");
        };
        root.to_path_buf()
    }
}
