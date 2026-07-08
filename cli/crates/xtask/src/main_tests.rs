use super::*;

use serde_json::json;
use tempfile::TempDir;

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut serialized = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    serialized.push('\n');
    write_file(path, &serialized)
}

fn seed_release_root(version: &str) -> Result<TempDir> {
    let tempdir = TempDir::new().with_context(|| "failed to create tempdir")?;
    let root = tempdir.path();

    write_file(
        &cargo_toml_path(root),
        &format!(
            r#"[workspace]
members = []

[workspace.package]
version = "{version}"
"#
        ),
    )?;

    write_json_file(
        &package_json_path(root),
        &json!({
            "name": "agent-tui",
            "version": version,
            "optionalDependencies": {
                "agent-tui-darwin-arm64": version,
                "agent-tui-darwin-x64": version,
                "agent-tui-linux-arm64": version,
                "agent-tui-linux-x64": version,
            }
        }),
    )?;

    for package_name in [
        "agent-tui-darwin-arm64",
        "agent-tui-darwin-x64",
        "agent-tui-linux-arm64",
        "agent-tui-linux-x64",
    ] {
        write_json_file(
            &root.join("npm").join(package_name).join("package.json"),
            &json!({
                "name": package_name,
                "version": version,
            }),
        )?;
    }

    Ok(tempdir)
}

fn seed_artifacts(input: &Path, kind: DistKind) -> Result<()> {
    for name in required_artifacts(kind) {
        write_file(
            &artifact_path(input, name),
            &format!("binary payload for {name}\n"),
        )?;
    }
    Ok(())
}

fn current_install_script_asset_name() -> Result<String> {
    let platform = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        other => bail!("unsupported test OS for install script: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" | "arm64" => "arm64",
        other => bail!("unsupported test arch for install script: {other}"),
    };
    Ok(format!("agent-tui-{platform}-{arch}"))
}

fn write_fake_curl(bin_dir: &Path) -> Result<()> {
    let script = r#"#!/bin/sh
set -eu

url=""
dest=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o)
      shift
      dest="$1"
      ;;
    http://* | https://*)
      url="$1"
      ;;
  esac
  shift
done

if [ -z "$url" ] || [ -z "$dest" ]; then
  echo "fake curl requires a URL and -o destination" >&2
  exit 2
fi

printf '%s\n' "$url" >> "$AGENT_TUI_TEST_CURL_LOG"
case "$url" in
  */checksums-sha256.txt)
    cp "$AGENT_TUI_TEST_FIXTURES/checksums-sha256.txt" "$dest"
    ;;
  *)
    name=${url##*/}
    cp "$AGENT_TUI_TEST_FIXTURES/$name" "$dest"
    ;;
esac
"#;
    let path = bin_dir.join("curl");
    write_file(&path, script)?;
    make_executable(&path)?;
    Ok(())
}

fn run_install_script_with_fake_downloads(
    temp_root: &Path,
    bin_dir: &Path,
    fixtures: &Path,
    log_path: &Path,
    install_dir: &Path,
    version: Option<&str>,
) -> Result<()> {
    let root = repository_root(&workspace_root()?)?;
    let install_script = root.join("install.sh");
    let path = prefixed_path(bin_dir)?;

    let mut command = Command::new("sh");
    command
        .arg(&install_script)
        .env("PATH", path)
        .env("AGENT_TUI_SKIP_PM", "1")
        .env("AGENT_TUI_INSTALL_DIR", install_dir)
        .env("AGENT_TUI_TEST_FIXTURES", fixtures)
        .env("AGENT_TUI_TEST_CURL_LOG", log_path)
        .current_dir(temp_root);
    if let Some(version) = version {
        command.env("AGENT_TUI_VERSION", version);
    }

    let output = command
        .output()
        .with_context(|| "failed to run install script")?;
    if !output.status.success() {
        bail!(
            "install script failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let installed = install_dir.join("agent-tui");
    assert!(
        installed.is_file(),
        "install script did not install {}",
        installed.display()
    );

    let version_output = Command::new(&installed)
        .output()
        .with_context(|| "failed to run installed fake binary")?;
    assert!(
        version_output.status.success(),
        "installed fake binary failed"
    );
    let stdout = String::from_utf8_lossy(&version_output.stdout);
    assert!(
        stdout.contains("agent-tui 1.2.3"),
        "unexpected installed binary output: {stdout}"
    );

    Ok(())
}

fn prefixed_path(bin_dir: &Path) -> Result<std::ffi::OsString> {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }
    env::join_paths(paths).with_context(|| "failed to build fake PATH")
}

#[test]
fn version_check_rejects_platform_package_version_mismatch() -> Result<()> {
    let tempdir = seed_release_root("1.2.3")?;
    let root = tempdir.path();

    write_json_file(
        &root.join("npm/agent-tui-linux-x64/package.json"),
        &json!({
            "name": "agent-tui-linux-x64",
            "version": "9.9.9",
        }),
    )?;

    let err = version_check(root, true).expect_err("platform package mismatch should fail");
    assert!(
        err.to_string().contains("version mismatch detected"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn set_version_updates_internal_workspace_dependency_versions() -> Result<()> {
    let tempdir = seed_release_root("1.2.3")?;
    let root = tempdir.path();
    write_file(
        &cargo_toml_path(root),
        r#"[workspace]
members = []

[workspace.package]
version = "1.2.3"

[workspace.dependencies]
agent-tui-common = { version = "1.2.3", path = "crates/agent-tui-common" }
agent-tui-app = { path = "crates/agent-tui-app" }
"#,
    )?;

    set_version(root, "1.2.4")?;

    let cargo_toml = fs::read_to_string(cargo_toml_path(root))
        .with_context(|| "failed to read updated Cargo.toml")?;
    assert!(
        cargo_toml.contains(
            r#"agent-tui-common = { version = "1.2.4", path = "crates/agent-tui-common" }"#
        ),
        "existing internal dependency version was not updated: {cargo_toml}"
    );
    assert!(
        cargo_toml
            .contains(r#"agent-tui-app = { version = "1.2.4", path = "crates/agent-tui-app" }"#),
        "missing internal dependency version was not inserted: {cargo_toml}"
    );
    Ok(())
}

#[test]
fn validate_release_inputs_rejects_missing_artifacts() -> Result<()> {
    let tempdir = seed_release_root("1.2.3")?;
    let root = tempdir.path();
    let artifacts = root.join("artifacts");

    let err = validate_release_inputs(root, "1.2.3", &artifacts)
        .expect_err("missing artifacts should fail validation");
    assert!(
        err.to_string().contains("artifacts directory not found"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn validate_release_inputs_rejects_requested_version_mismatch() -> Result<()> {
    let tempdir = seed_release_root("1.2.3")?;
    let root = tempdir.path();
    let artifacts = root.join("artifacts");
    seed_artifacts(&artifacts, DistKind::Release)?;

    let err = validate_release_inputs(root, "1.2.4", &artifacts)
        .expect_err("version mismatch should fail validation");
    assert!(
        err.to_string()
            .contains("version mismatch between Cargo.toml"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn dist_release_copies_artifacts_and_writes_checksums() -> Result<()> {
    let tempdir = TempDir::new().with_context(|| "failed to create tempdir")?;
    let input = tempdir.path().join("artifacts");
    let output = tempdir.path().join("release");
    seed_artifacts(&input, DistKind::Release)?;

    dist_release(tempdir.path(), &input, &output)?;

    for name in required_artifacts(DistKind::Release) {
        assert!(
            output.join(name).is_file(),
            "expected release asset {name} to be copied"
        );
    }

    let checksums = fs::read_to_string(output.join("checksums-sha256.txt"))
        .with_context(|| "failed to read checksums file")?;
    for name in required_artifacts(DistKind::Release) {
        assert!(
            checksums.contains(name),
            "expected checksum entry for {name}, got {checksums}"
        );
    }

    Ok(())
}

#[test]
fn dist_npm_stages_platform_binaries_into_bin_dir() -> Result<()> {
    let tempdir = TempDir::new().with_context(|| "failed to create tempdir")?;
    let input = tempdir.path().join("artifacts");
    let output = tempdir.path().join("npm");
    seed_artifacts(&input, DistKind::Npm)?;

    for name in required_artifacts(DistKind::Npm) {
        write_json_file(
            &output.join(name).join("package.json"),
            &json!({
                "name": name,
                "version": "1.2.3",
            }),
        )?;
    }

    dist_npm(&input, &output)?;

    for name in required_artifacts(DistKind::Npm) {
        let staged = output.join(name).join("bin/agent-tui");
        assert!(staged.is_file(), "expected staged binary for {name}");
        let contents =
            fs::read_to_string(&staged).with_context(|| format!("failed to read {name}"))?;
        assert!(
            contents.contains(name),
            "expected staged binary contents for {name}"
        );
    }

    Ok(())
}

fn fixture_state(version: &str) -> ReleaseChannelFixture {
    ReleaseChannelFixture {
        github_releases: Some(GitHubReleaseFixture {
            tag: format!("v{version}"),
            assets: required_artifacts(DistKind::Release)
                .iter()
                .map(|name| (*name).to_string())
                .chain(std::iter::once("checksums-sha256.txt".to_string()))
                .collect(),
        }),
        npm: Some(NpmFixture {
            meta_version: version.to_string(),
            optional_dependencies: npm_platform_package_names()
                .iter()
                .map(|name| ((*name).to_string(), version.to_string()))
                .collect(),
            platform_packages: npm_platform_package_names()
                .iter()
                .map(|name| ((*name).to_string(), version.to_string()))
                .collect(),
        }),
        crates_io: Some(CratesIoFixture {
            version: version.to_string(),
        }),
        homebrew: Some(HomebrewFixture {
            formula_present: true,
            version: Some(version.to_string()),
        }),
        install_script: Some(InstallScriptFixture {
            present: true,
            supports_pinned_version: true,
            verifies_checksums: true,
        }),
        source_install: Some(SourceInstallFixture {
            version: version.to_string(),
            package_path: "cli/crates/agent-tui".to_string(),
        }),
    }
}

#[test]
fn release_channel_inventory_lists_all_active_channels() {
    let channels = release_channel_inventory()
        .iter()
        .map(|channel| channel.name)
        .collect::<Vec<_>>();

    assert_eq!(
        channels,
        vec![
            "github-releases",
            "npm",
            "crates-io",
            "homebrew",
            "install-script",
            "source-install",
        ]
    );
}

#[test]
fn release_channel_verification_accepts_matching_fixture_state() -> Result<()> {
    let report = verify_release_channel_fixture("1.2.3", &fixture_state("1.2.3"));

    assert!(
        report.is_success(),
        "expected all channels to pass: {:?}",
        report.failures()
    );
    Ok(())
}

#[test]
fn release_channel_verification_reports_channel_specific_failures() -> Result<()> {
    let mut fixture = fixture_state("1.2.3");
    if let Some(github) = fixture.github_releases.as_mut() {
        github
            .assets
            .retain(|asset| asset != "agent-tui-darwin-arm64");
    }
    if let Some(npm) = fixture.npm.as_mut() {
        npm.platform_packages
            .insert("agent-tui-linux-x64".to_string(), "1.2.2".to_string());
    }
    fixture.homebrew = Some(HomebrewFixture {
        formula_present: false,
        version: None,
    });

    let report = verify_release_channel_fixture("1.2.3", &fixture);
    let failures = report
        .failures()
        .iter()
        .map(|status| format!("{}: {}", status.channel, status.message))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        failures.contains("github-releases: missing release asset agent-tui-darwin-arm64"),
        "unexpected failures: {failures}"
    );
    assert!(
        failures.contains("npm: agent-tui-linux-x64 is 1.2.2, expected 1.2.3"),
        "unexpected failures: {failures}"
    );
    assert!(
        failures.contains("homebrew: Homebrew formula is missing"),
        "unexpected failures: {failures}"
    );
    Ok(())
}

#[test]
fn homebrew_formula_version_can_be_inferred_from_release_url() {
    let formula = r#"
class AgentTui < Formula
  url "https://github.com/pproenca/agent-tui/releases/download/v1.2.3/agent-tui-darwin-arm64"
  sha256 "abc123"
end
"#;

    assert_eq!(
        parse_homebrew_formula_version(formula).as_deref(),
        Some("1.2.3")
    );
}

#[test]
fn release_channel_verification_can_be_scoped_to_selected_channels() -> Result<()> {
    let fixture = ReleaseChannelFixture {
        github_releases: Some(GitHubReleaseFixture {
            tag: "v1.2.3".to_string(),
            assets: required_artifacts(DistKind::Release)
                .iter()
                .map(|name| (*name).to_string())
                .chain(std::iter::once("checksums-sha256.txt".to_string()))
                .collect(),
        }),
        npm: None,
        crates_io: None,
        homebrew: None,
        install_script: None,
        source_install: None,
    };

    let report = verify_release_channel_fixture("1.2.3", &fixture);
    let selected = selected_release_channels(&[CHANNEL_GITHUB_RELEASES.to_string()])?;
    let scoped = filter_release_channel_report(report, &selected);

    assert!(
        scoped.is_success(),
        "selected GitHub channel should pass without requiring unselected channels: {:?}",
        scoped.failures()
    );
    Ok(())
}

#[test]
fn release_channel_filter_rejects_unknown_channels() {
    let err = selected_release_channels(&["mystery-channel".to_string()])
        .expect_err("unknown channels should fail before verification output");

    assert!(
        err.to_string().contains("unknown release channel"),
        "unexpected error: {err}"
    );
}

#[test]
fn live_release_channel_verification_only_runs_selected_channels() -> Result<()> {
    let selected = selected_release_channels(&[CHANNEL_INSTALL_SCRIPT.to_string()])?;
    let report = verify_release_channels_live(
        &workspace_root()?,
        "1.2.3",
        "http://127.0.0.1:9/missing",
        &selected,
    );

    assert_eq!(
        report
            .statuses
            .iter()
            .map(|status| status.channel)
            .collect::<Vec<_>>(),
        vec![CHANNEL_INSTALL_SCRIPT]
    );
    Ok(())
}

#[test]
fn install_script_uses_latest_and_pinned_github_assets_with_checksums() -> Result<()> {
    let tempdir = TempDir::new().with_context(|| "failed to create tempdir")?;
    let bin_dir = tempdir.path().join("bin");
    let fixtures = tempdir.path().join("fixtures");
    let installs = tempdir.path().join("installs");
    let logs = tempdir.path().join("logs");
    fs::create_dir_all(&bin_dir).with_context(|| "failed to create fake bin dir")?;
    fs::create_dir_all(&fixtures).with_context(|| "failed to create fixtures dir")?;
    fs::create_dir_all(&installs).with_context(|| "failed to create installs dir")?;
    fs::create_dir_all(&logs).with_context(|| "failed to create logs dir")?;

    let asset = current_install_script_asset_name()?;
    let asset_path = fixtures.join(&asset);
    write_file(&asset_path, "#!/bin/sh\nprintf 'agent-tui 1.2.3\\n'\n")?;
    make_executable(&asset_path)?;

    let digest = sha256_file(&asset_path)?;
    write_file(
        &fixtures.join("checksums-sha256.txt"),
        &format!("{digest}  {asset}\n"),
    )?;
    write_fake_curl(&bin_dir)?;

    let latest_log = logs.join("latest.log");
    let latest_install = installs.join("latest");
    run_install_script_with_fake_downloads(
        tempdir.path(),
        &bin_dir,
        &fixtures,
        &latest_log,
        &latest_install,
        None,
    )?;

    let latest_urls = fs::read_to_string(&latest_log).with_context(|| "failed to read log")?;
    assert!(
        latest_urls.contains(&format!(
            "https://github.com/pproenca/agent-tui/releases/latest/download/{asset}"
        )),
        "latest install did not download expected asset URL: {latest_urls}"
    );
    assert!(
        latest_urls.contains(
            "https://github.com/pproenca/agent-tui/releases/latest/download/checksums-sha256.txt"
        ),
        "latest install did not verify checksums: {latest_urls}"
    );

    let pinned_log = logs.join("pinned.log");
    let pinned_install = installs.join("pinned");
    run_install_script_with_fake_downloads(
        tempdir.path(),
        &bin_dir,
        &fixtures,
        &pinned_log,
        &pinned_install,
        Some("v1.2.3"),
    )?;

    let pinned_urls = fs::read_to_string(&pinned_log).with_context(|| "failed to read log")?;
    assert!(
        pinned_urls.contains(&format!(
            "https://github.com/pproenca/agent-tui/releases/download/v1.2.3/{asset}"
        )),
        "pinned install did not download expected asset URL: {pinned_urls}"
    );
    assert!(
        pinned_urls.contains(
            "https://github.com/pproenca/agent-tui/releases/download/v1.2.3/checksums-sha256.txt"
        ),
        "pinned install did not verify checksums: {pinned_urls}"
    );

    Ok(())
}

#[test]
fn release_workflow_verifies_and_smokes_github_install_script_and_npm() -> Result<()> {
    let root = repository_root(&workspace_root()?)?;
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .with_context(|| "failed to read release workflow")?;

    for needle in [
        "release-channels verify",
        "--channel github-releases",
        "--channel install-script",
        "--channel npm",
        "npm install -g \"agent-tui@$VERSION\"",
        "AGENT_TUI_SKIP_PM=1",
        "AGENT_TUI_VERSION=\"$VERSION\"",
        "\"$NPM_CONFIG_PREFIX/bin/agent-tui\" --version",
        "\"$INSTALL_DIR/agent-tui\" --version",
    ] {
        assert!(
            workflow.contains(needle),
            "release workflow missing expected smoke/verification step: {needle}"
        );
    }

    Ok(())
}

#[test]
fn crates_io_publish_plan_accepts_runtime_crate_graph() -> Result<()> {
    verify_crates_io_publish_plan(&workspace_root()?)
}

#[test]
fn release_workflow_publishes_and_smokes_crates_io_and_source_install() -> Result<()> {
    let root = repository_root(&workspace_root()?)?;
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .with_context(|| "failed to read release workflow")?;

    for needle in [
        "publish-crates:",
        "release-channels verify-crates-io-publish-plan",
        "cargo publish -p \"$crate\" --allow-dirty",
        "--channel crates-io",
        "--channel source-install",
        "cargo install agent-tui --version \"$VERSION\"",
        "cargo install --path cli/crates/agent-tui",
        "\"$CRATES_ROOT/bin/agent-tui\" --version",
        "\"$SOURCE_ROOT/bin/agent-tui\" --version",
    ] {
        assert!(
            workflow.contains(needle),
            "release workflow missing expected Rust release step: {needle}"
        );
    }

    Ok(())
}

#[test]
fn build_scripts_accept_packaged_vcs_metadata_without_git() -> Result<()> {
    let root = repository_root(&workspace_root()?)?;
    let build_script_paths = [
        root.join("cli/crates/agent-tui/build.rs"),
        root.join("cli/crates/agent-tui-adapters/build.rs"),
        root.join("cli/crates/agent-tui-app/build.rs"),
    ];
    let build_script = fs::read_to_string(&build_script_paths[0])
        .with_context(|| "failed to read build script")?;
    for path in &build_script_paths[1..] {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        assert_eq!(
            contents,
            build_script,
            "release metadata build scripts drifted: {}",
            path.display()
        );
    }

    let tempdir = TempDir::new().with_context(|| "failed to create tempdir")?;
    write_file(
        &tempdir.path().join("Cargo.toml"),
        r#"[package]
name = "agent-tui-build-script-smoke"
version = "1.2.3"
edition = "2024"
build = "build.rs"
"#,
    )?;
    write_file(&tempdir.path().join("src/lib.rs"), "")?;
    write_file(&tempdir.path().join("build.rs"), &build_script)?;
    write_file(
        &tempdir.path().join(".cargo_vcs_info.json"),
        r#"{
  "git": {
    "sha1": "1234567890abcdef1234567890abcdef12345678",
    "dirty": false
  },
  "path_in_vcs": "cli/crates/agent-tui"
}
"#,
    )?;

    let output = Command::new("cargo")
        .arg("check")
        .arg("--release")
        .env_remove("AGENT_TUI_GIT_SHA")
        .env_remove("AGENT_TUI_STRICT_BUILD_METADATA")
        .env("CARGO_TARGET_DIR", tempdir.path().join("target"))
        .current_dir(tempdir.path())
        .output()
        .with_context(|| "failed to run packaged build script smoke")?;

    assert!(
        output.status.success(),
        "packaged build script smoke failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn readme_documents_crates_io_and_source_install_paths() -> Result<()> {
    let root = repository_root(&workspace_root()?)?;
    let readme =
        fs::read_to_string(root.join("README.md")).with_context(|| "failed to read README.md")?;

    for needle in [
        "cargo install agent-tui --locked",
        "cargo install --path cli/crates/agent-tui --locked",
        "cargo install --git https://github.com/pproenca/agent-tui.git --path cli/crates/agent-tui --locked",
    ] {
        assert!(
            readme.contains(needle),
            "README missing expected Rust install command: {needle}"
        );
    }

    Ok(())
}
