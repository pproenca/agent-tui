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
