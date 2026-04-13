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

    fn wait_stable(&mut self, _session_id: &str, _timeout_ms: u64) -> Result<(), ExplorerError> {
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
fn parse_allows_openspec_expectation_block() {
    let spec_text = "---\nschema_version: \"v1\"\ncommand: \"printf app\"\ncols: 120\nrows: 40\ndefault_timeout_ms: 3000\ngenerated_at: \"2026-02-18T00:00:00Z\"\ngenerator: \"tui-explorer/1\"\n---\n\n## Scenario: Basic\n### Expectation\n- **WHEN** the operator replays: press \"Enter\", wait_stable\n- **THEN** the screen contains \"OK\"\n- **SHOULD** execute machine checks: expect \"OK\"\n- press: \"Enter\"\n- wait_stable: true\n- expect: \"OK\"\n";
    let spec = assert_ok(parse_spec_text(spec_text));
    assert_eq!(spec.scenarios.len(), 1);
    assert!(matches!(spec.scenarios[0].steps[0], Step::Press(_)));
    assert!(matches!(spec.scenarios[0].steps[1], Step::WaitStable));
    assert!(matches!(spec.scenarios[0].steps[2], Step::Expect(_)));
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
fn render_markdown_includes_openspec_expectation_block() {
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
            ("cwd".to_string(), Value::Null),
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
            name: "Basic".to_string(),
            steps: vec![
                Step::Press("Enter".to_string()),
                Step::WaitStable,
                Step::Expect("OK".to_string()),
            ],
        }],
    };

    let text = assert_ok(render_markdown(&spec));
    assert!(text.contains("### Expectation"));
    assert!(text.contains("- **WHEN**"));
    assert!(text.contains("- **THEN**"));
    assert!(text.contains("- **SHOULD**"));
    assert!(text.contains("- expect: \"OK\""));
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
fn verify_happy_path_with_openspec_expectation_lines() {
    let spec_text = "---\nschema_version: \"v1\"\ncommand: \"printf app\"\ncols: 120\nrows: 40\ndefault_timeout_ms: 3000\ngenerated_at: \"2026-02-18T00:00:00Z\"\ngenerator: \"tui-explorer/1\"\n---\n\n## Scenario: Basic\n### Expectation\n- **WHEN** replaying the path: press \"Enter\", wait_stable\n- **THEN** the screen contains \"OK\"\n- **SHOULD** execute machine checks: expect \"OK\"\n- press: \"Enter\"\n- wait_stable: true\n- expect: \"OK\"\n";
    let spec = assert_ok(parse_spec_text(spec_text));
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
