#![expect(clippy::print_stdout, reason = "xtask is a CLI orchestrator")]
#![expect(clippy::print_stderr, reason = "xtask is a CLI orchestrator")]

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use cargo_metadata::Metadata;
use cargo_metadata::MetadataCommand;
use cargo_metadata::Package;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::IsTerminal;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "xtask")]
#[command(about = "Rust workspace task runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Version {
        #[command(subcommand)]
        command: VersionCommands,
    },
    Release(ReleaseArgs),
    Ci,
    Architecture {
        #[command(subcommand)]
        command: ArchitectureCommands,
    },
    Dist {
        #[command(subcommand)]
        command: DistCommands,
    },
}

#[derive(Subcommand, Debug)]
enum VersionCommands {
    Check {
        #[arg(long)]
        quiet: bool,
    },
    Current,
    AssertTag {
        tag: String,
    },
    AssertInput {
        version: String,
    },
    Set {
        version: String,
    },
}

#[derive(Args, Debug)]
struct ReleaseArgs {
    version_or_bump: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Subcommand, Debug)]
enum ArchitectureCommands {
    Check {
        #[arg(long)]
        verbose: bool,
    },
    Graph {
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
enum DistCommands {
    Release {
        #[arg(long, default_value = "artifacts")]
        input: String,
        #[arg(long, default_value = "release")]
        output: String,
    },
    Verify {
        #[arg(long, default_value = "artifacts")]
        input: String,
        #[arg(long, default_value = "release")]
        kind: DistKind,
    },
    Npm {
        #[arg(long, default_value = "artifacts")]
        input: String,
        #[arg(long, default_value = "npm")]
        output: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DistKind {
    Release,
    Npm,
}

#[derive(Debug, Clone, Copy)]
enum Bump {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Clone, Copy)]
struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("xtask: error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = workspace_root()?;

    match cli.command {
        Commands::Version { command } => match command {
            VersionCommands::Check { quiet } => version_check(&root, quiet),
            VersionCommands::Current => {
                println!("{}", read_package_version(&package_json_path(&root))?);
                Ok(())
            }
            VersionCommands::AssertTag { tag } => assert_tag(&root, &tag),
            VersionCommands::AssertInput { version } => assert_input(&root, &version),
            VersionCommands::Set { version } => set_version(&root, &version),
        },
        Commands::Release(args) => release(&root, &args.version_or_bump, !args.yes),
        Commands::Ci => ci(&root),
        Commands::Architecture { command } => match command {
            ArchitectureCommands::Check { verbose } => architecture_check(&root, verbose),
            ArchitectureCommands::Graph { format } => architecture_graph(&root, &format),
        },
        Commands::Dist { command } => match command {
            DistCommands::Release { input, output } => dist_release(
                &root,
                &path_from_root(&root, &input),
                &path_from_root(&root, &output),
            ),
            DistCommands::Verify { input, kind } => {
                dist_verify(&path_from_root(&root, &input), kind)
            }
            DistCommands::Npm { input, output } => dist_npm(
                &path_from_root(&root, &input),
                &path_from_root(&root, &output),
            ),
        },
    }
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(crates_dir) = manifest_dir.parent() else {
        bail!("invalid xtask location: missing crates dir");
    };
    let Some(root) = crates_dir.parent() else {
        bail!("invalid xtask location: missing workspace root");
    };
    Ok(root.to_path_buf())
}

fn path_from_root(root: &Path, raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn cargo_toml_path(root: &Path) -> PathBuf {
    root.join("Cargo.toml")
}

fn package_json_path(root: &Path) -> PathBuf {
    root.join("package.json")
}

fn npm_platform_package_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let npm_root = root.join("npm");
    if !npm_root.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in
        fs::read_dir(&npm_root).with_context(|| format!("failed to list {}", npm_root.display()))?
    {
        let entry = entry.with_context(|| "failed to read npm entry")?;
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let package_json = dir_path.join("package.json");
        if package_json.exists() {
            paths.push(package_json);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_versions(root: &Path) -> Result<(String, String)> {
    let cargo_version = read_cargo_version(&cargo_toml_path(root))?;
    let package_version = read_package_version(&package_json_path(root))?;
    Ok((cargo_version, package_version))
}

fn read_cargo_version(path: &Path) -> Result<String> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    if let Some(version) = read_toml_version_in_section(&contents, "workspace.package") {
        return Ok(version);
    }
    if let Some(version) = read_toml_version_in_section(&contents, "package") {
        return Ok(version);
    }

    bail!("could not find version in {}", path.display())
}

fn read_toml_version_in_section(contents: &str, section: &str) -> Option<String> {
    let mut in_section = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == format!("[{section}]");
            continue;
        }
        if !in_section {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("version") {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix('=') {
                let value = value.trim();
                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    return Some(value[1..value.len() - 1].to_string());
                }
            }
        }
    }

    None
}

fn write_cargo_version(path: &Path, version: &str) -> Result<()> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    if let Some(updated) = update_toml_version_in_section(&contents, "workspace.package", version) {
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
        return Ok(());
    }

    if let Some(updated) = update_toml_version_in_section(&contents, "package", version) {
        fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
        return Ok(());
    }

    bail!("could not update version in {}", path.display())
}

fn update_toml_version_in_section(contents: &str, section: &str, version: &str) -> Option<String> {
    let trailing_newline = contents.ends_with('\n');
    let mut lines = contents
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>();

    let mut in_section = false;
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == format!("[{section}]");
            continue;
        }
        if !in_section {
            continue;
        }

        let line_trimmed = line.trim_start();
        if line_trimmed.starts_with("version") {
            let indent_len = line.len().saturating_sub(line_trimmed.len());
            let indent = &line[..indent_len];
            *line = format!("{indent}version = \"{version}\"");
            let mut joined = lines.join("\n");
            if trailing_newline {
                joined.push('\n');
            }
            return Some(joined);
        }
    }

    None
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse JSON in {}", path.display()))?;
    Ok(value)
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut serialized = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    serialized.push('\n');
    fs::write(path, serialized).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn read_package_version(path: &Path) -> Result<String> {
    let json = read_json(path)?;
    let Some(version) = json.get("version").and_then(serde_json::Value::as_str) else {
        bail!("could not find version in {}", path.display());
    };
    Ok(version.to_string())
}

fn write_package_version(path: &Path, version: &str) -> Result<()> {
    let mut json = read_json(path)?;
    let Some(obj) = json.as_object_mut() else {
        bail!("{} is not a JSON object", path.display());
    };
    obj.insert(
        "version".to_string(),
        serde_json::Value::String(version.to_string()),
    );

    if let Some(optional_deps) = obj
        .get_mut("optionalDependencies")
        .and_then(serde_json::Value::as_object_mut)
    {
        for (name, dep_version) in optional_deps.iter_mut() {
            if name.starts_with("agent-tui-") {
                *dep_version = serde_json::Value::String(version.to_string());
            }
        }
    }

    write_json(path, &json)
}

fn version_check(root: &Path, quiet: bool) -> Result<()> {
    let (cargo_version, package_version) = read_versions(root)?;
    if cargo_version != package_version {
        bail!(
            "version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        );
    }

    let package_json = read_json(&package_json_path(root))?;
    let optional_deps = package_json
        .get("optionalDependencies")
        .and_then(serde_json::Value::as_object)
        .cloned();

    let mut npm_package_names = HashSet::new();
    for npm_path in npm_platform_package_paths(root)? {
        let npm_json = read_json(&npm_path)?;
        let Some(npm_name) = npm_json.get("name").and_then(serde_json::Value::as_str) else {
            bail!("could not find name in {}", npm_path.display());
        };
        let Some(npm_version) = npm_json.get("version").and_then(serde_json::Value::as_str) else {
            bail!("could not find version in {}", npm_path.display());
        };

        npm_package_names.insert(npm_name.to_string());

        if npm_version != package_version {
            bail!(
                "version mismatch detected!\n  package.json: {package_version}\n  {}: {npm_version}",
                npm_path.display()
            );
        }
    }

    if !npm_package_names.is_empty() && optional_deps.is_none() {
        bail!("missing optionalDependencies in package.json");
    }

    if let Some(optional_deps) = optional_deps {
        for (name, dep_version_value) in &optional_deps {
            if !name.starts_with("agent-tui-") {
                continue;
            }
            let Some(dep_version) = dep_version_value.as_str() else {
                bail!("optionalDependencies.{name} must be a string");
            };
            if dep_version != package_version {
                bail!(
                    "version mismatch detected!\n  package.json: {package_version}\n  optionalDependencies.{name}: {dep_version}"
                );
            }
            if !npm_package_names.contains(name) {
                bail!("optional dependency {name} has no matching npm package under npm/");
            }
        }

        for npm_name in &npm_package_names {
            if !optional_deps.contains_key(npm_name) {
                bail!("missing optionalDependencies entry for {npm_name} in package.json");
            }
        }
    }

    if !quiet {
        println!("Version check passed: {cargo_version}");
    }
    Ok(())
}

fn normalize_tag(tag: &str) -> String {
    let mut normalized = tag.to_string();
    if let Some(stripped) = normalized.strip_prefix("refs/tags/") {
        normalized = stripped.to_string();
    }
    if let Some(stripped) = normalized.strip_prefix('v') {
        normalized = stripped.to_string();
    }
    normalized
}

fn assert_tag(root: &Path, tag: &str) -> Result<()> {
    let (cargo_version, package_version) = read_versions(root)?;
    let tag_version = normalize_tag(tag);

    if cargo_version != package_version {
        bail!(
            "version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        );
    }

    if cargo_version != tag_version {
        bail!("version mismatch between Cargo.toml ({cargo_version}) and tag ({tag})");
    }

    Ok(())
}

fn assert_input(root: &Path, version: &str) -> Result<()> {
    let (cargo_version, package_version) = read_versions(root)?;

    if cargo_version != package_version {
        bail!(
            "version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        );
    }

    if cargo_version != version {
        bail!("version mismatch between Cargo.toml ({cargo_version}) and input ({version})");
    }

    Ok(())
}

fn set_version(root: &Path, version: &str) -> Result<()> {
    let target_version = ensure_semver(version)?;
    println!("Setting version to {target_version}...");

    println!("Updating package.json...");
    write_package_version(&package_json_path(root), &target_version)?;
    for npm_path in npm_platform_package_paths(root)? {
        write_package_version(&npm_path, &target_version)?;
    }

    println!("Updating Cargo.toml...");
    write_cargo_version(&cargo_toml_path(root), &target_version)?;

    println!("Done.");
    Ok(())
}

fn parse_semver(value: &str) -> Result<Semver> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("invalid version: {value}");
    }

    let core = normalized
        .split(['-', '+'])
        .next()
        .context("invalid version")?;
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        bail!("invalid version: {value}");
    }

    let major = parts[0]
        .parse::<u64>()
        .with_context(|| format!("invalid major version in {value}"))?;
    let minor = parts[1]
        .parse::<u64>()
        .with_context(|| format!("invalid minor version in {value}"))?;
    let patch = parts[2]
        .parse::<u64>()
        .with_context(|| format!("invalid patch version in {value}"))?;

    Ok(Semver {
        major,
        minor,
        patch,
    })
}

fn ensure_semver(value: &str) -> Result<String> {
    let _ = parse_semver(value)?;
    Ok(value.to_string())
}

fn parse_bump(value: &str) -> Option<Bump> {
    match value {
        "major" => Some(Bump::Major),
        "minor" => Some(Bump::Minor),
        "patch" => Some(Bump::Patch),
        _ => None,
    }
}

fn bump_version(current: &str, bump: Bump) -> Result<String> {
    let parsed = parse_semver(current)?;
    let next = match bump {
        Bump::Major => Semver {
            major: parsed.major + 1,
            minor: 0,
            patch: 0,
        },
        Bump::Minor => Semver {
            major: parsed.major,
            minor: parsed.minor + 1,
            patch: 0,
        },
        Bump::Patch => Semver {
            major: parsed.major,
            minor: parsed.minor,
            patch: parsed.patch + 1,
        },
    };
    Ok(format!("{}.{}.{}", next.major, next.minor, next.patch))
}

fn latest_tag_version(root: &Path) -> Result<Option<String>> {
    let output = run_output(
        "git",
        &["tag", "--list", "v*", "--sort=-v:refname"],
        Some(root),
    )?;

    let first = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(normalize_tag);

    match first {
        Some(tag) => Ok(Some(ensure_semver(&tag)?)),
        None => Ok(None),
    }
}

fn ensure_git_clean(root: &Path) -> Result<()> {
    let output = run_output("git", &["status", "--porcelain"], Some(root))?;
    if !output.trim().is_empty() {
        bail!("you have uncommitted or untracked changes. commit or stash them first");
    }
    Ok(())
}

fn ensure_tag_absent(root: &Path, tag: &str) -> Result<()> {
    let status = Command::new("git")
        .arg("rev-parse")
        .arg(tag)
        .current_dir(root)
        .status()
        .with_context(|| "failed to run git rev-parse")?;

    if status.success() {
        bail!("tag {tag} already exists");
    }
    Ok(())
}

fn confirm_proceed(message: &str) -> Result<bool> {
    if !io::stdin().is_terminal() {
        bail!("confirmation required but stdin is not a TTY. Re-run with --yes");
    }

    print!("{message} [y/N] ");
    io::stdout()
        .flush()
        .with_context(|| "failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .with_context(|| "failed to read confirmation")?;

    let answer = input.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn release(root: &Path, version_or_bump: &str, confirm: bool) -> Result<()> {
    let current_version = match latest_tag_version(root)? {
        Some(version) => version,
        None => read_cargo_version(&cargo_toml_path(root))?,
    };

    let target_version = if let Some(bump) = parse_bump(version_or_bump) {
        bump_version(&current_version, bump)?
    } else {
        ensure_semver(version_or_bump)?
    };

    let tag = format!("v{target_version}");

    ensure_git_clean(root)?;
    ensure_tag_absent(root, &tag)?;

    println!("Releasing version {target_version}...");
    if confirm && !confirm_proceed(&format!("Create and push tag {tag}?"))? {
        println!("Release aborted before tagging.");
        return Ok(());
    }

    println!("Creating tag {tag}...");
    run_command(
        "git",
        &[
            "tag",
            "-a",
            &tag,
            "-m",
            &format!("Release {target_version}"),
        ],
        Some(root),
    )?;

    println!("Pushing tag {tag}...");
    run_command("git", &["push", "origin", &tag], Some(root))?;

    println!("Done! Release tag {tag} pushed.");
    Ok(())
}

fn ci(root: &Path) -> Result<()> {
    println!("Running CI checks...");

    run_step("Checking formatting", || {
        run_command("cargo", &["fmt", "--all", "--", "--check"], Some(root))
    })?;

    run_step("Running clippy", || {
        run_command(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
            Some(root),
        )
    })?;

    run_step("Running architecture checks", || {
        architecture_check(root, false)
    })?;

    run_step("Running tests", || {
        run_command("cargo", &["test", "--workspace"], Some(root))
    })?;

    if has_command("cargo-machete") {
        run_step("Checking for unused dependencies", || {
            run_command("cargo-machete", &["--skip-target-dir", "."], Some(root))
        })?;
    } else {
        println!("cargo-machete not installed, skipping...");
    }

    run_step("Checking version consistency", || version_check(root, true))?;

    println!("All checks passed!");
    Ok(())
}

fn run_step<F>(label: &str, action: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    println!("\n-> {label}...");
    action()
}

fn has_command(name: &str) -> bool {
    let Some(path_os) = env::var_os("PATH") else {
        return false;
    };

    let exts = if cfg!(windows) {
        env::var("PATHEXT")
            .ok()
            .map(|value| {
                value
                    .split(';')
                    .filter(|item| !item.is_empty())
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![".EXE".to_string(), ".CMD".to_string(), ".BAT".to_string()])
    } else {
        vec![String::new()]
    };

    env::split_paths(&path_os).any(|dir| {
        exts.iter().any(|ext| {
            let candidate = dir.join(format!("{name}{ext}"));
            candidate.is_file()
        })
    })
}

fn run_command(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to run command: {program} {}", args.join(" ")))?;

    if !status.success() {
        bail!("command failed: {program} {}", args.join(" "));
    }

    Ok(())
}

fn run_output(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to run command: {program} {}", args.join(" ")))?;

    if !output.status.success() {
        bail!("command failed: {program} {}", args.join(" "));
    }

    String::from_utf8(output.stdout).with_context(|| "command output was not valid UTF-8")
}

#[derive(Debug, Clone, Serialize)]
struct ArchitectureEdge {
    source: String,
    target: String,
}

#[derive(Debug, Serialize)]
struct ArchitectureGraphPayload {
    edges: Vec<ArchitectureEdge>,
    summary: Vec<String>,
    violations: Vec<String>,
}

fn architecture_check(root: &Path, verbose: bool) -> Result<()> {
    let report = architecture_report(root)?;

    if !report.violations.is_empty() {
        let mut message = String::from("Architecture check failed:\n");
        for violation in &report.violations {
            message.push_str("- ");
            message.push_str(violation);
            message.push('\n');
        }
        bail!(message.trim_end().to_string());
    }

    if verbose {
        println!("Layer dependency summary:");
        for line in &report.summary {
            println!("  {line}");
        }
    }

    println!("Architecture checks passed.");
    Ok(())
}

fn architecture_graph(root: &Path, format: &str) -> Result<()> {
    if format != "json" {
        bail!("unsupported format: {format}");
    }

    let report = architecture_report(root)?;
    let payload = ArchitectureGraphPayload {
        edges: report.edges,
        summary: report.summary,
        violations: report.violations,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).with_context(|| "failed to serialize graph")?
    );

    Ok(())
}

struct ArchitectureReport {
    edges: Vec<ArchitectureEdge>,
    summary: Vec<String>,
    violations: Vec<String>,
}

fn architecture_report(root: &Path) -> Result<ArchitectureReport> {
    let metadata = workspace_metadata(root)?;

    let known = HashSet::from([
        "agent-tui-common",
        "agent-tui-domain",
        "agent-tui-usecases",
        "agent-tui-adapters",
        "agent-tui-infra",
        "agent-tui-app",
        "agent-tui",
        "xtask",
    ]);

    let mut violations = Vec::new();

    for package in &metadata.packages {
        if package.name.starts_with("agent-tui-") && !known.contains(package.name.as_str()) {
            violations.push(format!(
                "unknown internal crate in workspace: {}",
                package.name
            ));
        }
    }

    for expected in [
        "agent-tui-common",
        "agent-tui-domain",
        "agent-tui-usecases",
        "agent-tui-adapters",
        "agent-tui-infra",
        "agent-tui-app",
        "agent-tui",
    ] {
        if !metadata.packages.iter().any(|pkg| pkg.name == expected) {
            violations.push(format!("missing required workspace crate: {expected}"));
        }
    }

    violations.extend(check_src_layout(&metadata)?);

    let allowed = allowed_dependency_matrix();
    let package_name_by_id = metadata
        .packages
        .iter()
        .map(|pkg| (pkg.id.clone(), pkg.name.clone()))
        .collect::<HashMap<_, _>>();

    let mut edges = Vec::new();
    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            let Some(source) = package_name_by_id.get(&node.id) else {
                continue;
            };
            if !allowed.contains_key(source.as_str()) {
                continue;
            }

            for dep in &node.deps {
                let Some(target) = package_name_by_id.get(&dep.pkg) else {
                    continue;
                };
                if !allowed.contains_key(target.as_str()) {
                    continue;
                }

                edges.push(ArchitectureEdge {
                    source: source.to_string(),
                    target: target.to_string(),
                });
            }
        }
    }

    edges.sort_by(|a, b| {
        (a.source.as_str(), a.target.as_str()).cmp(&(b.source.as_str(), b.target.as_str()))
    });
    edges.dedup_by(|a, b| a.source == b.source && a.target == b.target);

    for edge in &edges {
        let Some(allowed_targets) = allowed.get(edge.source.as_str()) else {
            continue;
        };
        if !allowed_targets.contains(edge.target.as_str()) {
            violations.push(format!(
                "forbidden crate dependency: {} -> {}",
                edge.source, edge.target
            ));
        }
    }

    let mut summary_counts = BTreeMap::new();
    for edge in &edges {
        let key = format!("{} -> {}", edge.source, edge.target);
        *summary_counts.entry(key).or_insert(0usize) += 1;
    }
    let summary = summary_counts
        .into_iter()
        .map(|(edge, count)| format!("{edge} ({count})"))
        .collect::<Vec<_>>();

    Ok(ArchitectureReport {
        edges,
        summary,
        violations,
    })
}

fn workspace_metadata(root: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .exec()
        .with_context(|| "failed to run cargo metadata")
}

fn check_src_layout(metadata: &Metadata) -> Result<Vec<String>> {
    let mut violations = Vec::new();

    let mut rules = HashMap::new();
    rules.insert("agent-tui-common", (BTreeSet::from(["common"]), false));
    rules.insert("agent-tui-domain", (BTreeSet::from(["domain"]), false));
    rules.insert("agent-tui-usecases", (BTreeSet::from(["usecases"]), false));
    rules.insert("agent-tui-adapters", (BTreeSet::from(["adapters"]), false));
    rules.insert("agent-tui-infra", (BTreeSet::from(["infra"]), false));
    rules.insert(
        "agent-tui-app",
        (BTreeSet::from(["app", "test_support"]), false),
    );
    rules.insert("agent-tui", (BTreeSet::new(), true));

    for package in &metadata.packages {
        let Some((allowed_dirs, allow_bin)) = rules.get(package.name.as_str()) else {
            continue;
        };

        let crate_root = package_dir(package)?;
        let src_dir = crate_root.join("src");
        if !src_dir.is_dir() {
            violations.push(format!(
                "{} is missing src/ directory ({})",
                package.name,
                src_dir.display()
            ));
            continue;
        }

        let mut present_dirs = BTreeSet::new();
        for entry in fs::read_dir(&src_dir)
            .with_context(|| format!("failed to read {}", src_dir.display()))?
        {
            let entry = entry.with_context(|| "failed to read src entry")?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            present_dirs.insert(name.to_string());
        }

        for present in &present_dirs {
            if *allow_bin && present == "bin" {
                continue;
            }
            if !allowed_dirs.contains(present.as_str()) {
                violations.push(format!(
                    "{} has unknown top-level production dir: src/{}",
                    package.name, present
                ));
            }
        }

        for expected in allowed_dirs {
            if !present_dirs.contains(*expected) {
                violations.push(format!(
                    "{} is missing expected top-level dir: src/{}",
                    package.name, expected
                ));
            }
        }
    }

    Ok(violations)
}

fn package_dir(package: &Package) -> Result<PathBuf> {
    let manifest = PathBuf::from(package.manifest_path.as_str());
    let Some(parent) = manifest.parent() else {
        bail!("invalid manifest path for {}", package.name);
    };
    Ok(parent.to_path_buf())
}

fn allowed_dependency_matrix() -> HashMap<&'static str, HashSet<&'static str>> {
    HashMap::from([
        ("agent-tui-common", HashSet::from([])),
        ("agent-tui-domain", HashSet::from(["agent-tui-common"])),
        (
            "agent-tui-usecases",
            HashSet::from(["agent-tui-domain", "agent-tui-common"]),
        ),
        (
            "agent-tui-adapters",
            HashSet::from(["agent-tui-usecases", "agent-tui-domain", "agent-tui-common"]),
        ),
        (
            "agent-tui-infra",
            HashSet::from(["agent-tui-usecases", "agent-tui-domain", "agent-tui-common"]),
        ),
        (
            "agent-tui-app",
            HashSet::from([
                "agent-tui-adapters",
                "agent-tui-infra",
                "agent-tui-usecases",
                "agent-tui-domain",
                "agent-tui-common",
            ]),
        ),
        ("agent-tui", HashSet::from(["agent-tui-app"])),
    ])
}

fn dist_verify(input: &Path, kind: DistKind) -> Result<()> {
    if !input.exists() {
        bail!("artifacts directory not found: {}", input.display());
    }

    let mut missing = Vec::new();
    for name in required_artifacts(kind) {
        let path = artifact_path(input, name);
        if !path.exists() {
            missing.push(path);
        }
    }

    if !missing.is_empty() {
        let mut message = String::from("missing required artifacts:\n");
        for artifact in missing {
            message.push_str(&artifact.display().to_string());
            message.push('\n');
        }
        bail!(message.trim_end().to_string());
    }

    println!("All required artifacts present for {:?}.", kind);
    Ok(())
}

fn dist_release(_root: &Path, input: &Path, output: &Path) -> Result<()> {
    dist_verify(input, DistKind::Release)?;

    fs::create_dir_all(output).with_context(|| format!("failed to create {}", output.display()))?;

    let mut seen = HashSet::new();
    for entry in WalkDir::new(input).into_iter().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        if !seen.insert(file_name.to_string()) {
            bail!("duplicate artifact filename: {file_name}");
        }

        let dest = output.join(file_name);
        fs::copy(path, &dest)
            .with_context(|| format!("failed to copy {} to {}", path.display(), dest.display()))?;
        make_executable(&dest)?;
    }

    let mut output_files = fs::read_dir(output)
        .with_context(|| format!("failed to list {}", output.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|name| name != "checksums-sha256.txt")
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    output_files.sort();

    let mut checksums = String::new();
    for file in output_files {
        let digest = sha256_file(&file)?;
        let Some(name) = file.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        checksums.push_str(&format!("{digest}  {name}\n"));
    }

    fs::write(output.join("checksums-sha256.txt"), checksums)
        .with_context(|| "failed to write checksums-sha256.txt")?;

    println!("Prepared release assets in {}", output.display());
    Ok(())
}

fn dist_npm(input: &Path, output: &Path) -> Result<()> {
    if !input.exists() {
        bail!("artifacts directory not found: {}", input.display());
    }

    for name in required_artifacts(DistKind::Npm) {
        let artifact = artifact_path(input, name);
        if !artifact.exists() {
            bail!("missing artifact: {}", artifact.display());
        }

        let package_dir = output.join(name);
        let package_json = package_dir.join("package.json");
        if !package_json.exists() {
            bail!("missing npm package: {}", package_json.display());
        }

        let bin_dir = package_dir.join("bin");
        fs::create_dir_all(&bin_dir)
            .with_context(|| format!("failed to create {}", bin_dir.display()))?;

        let dest = bin_dir.join("agent-tui");
        fs::copy(&artifact, &dest).with_context(|| {
            format!(
                "failed to copy {} to {}",
                artifact.display(),
                dest.display()
            )
        })?;
        make_executable(&dest)?;
    }

    println!("Prepared npm platform packages in {}", output.display());
    Ok(())
}

fn required_artifacts(kind: DistKind) -> &'static [&'static str] {
    match kind {
        DistKind::Release => &[
            "agent-tui-linux-x64",
            "agent-tui-linux-arm64",
            "agent-tui-linux-x64-musl",
            "agent-tui-linux-arm64-musl",
            "agent-tui-darwin-x64",
            "agent-tui-darwin-arm64",
        ],
        DistKind::Npm => &[
            "agent-tui-linux-x64",
            "agent-tui-linux-arm64",
            "agent-tui-darwin-x64",
            "agent-tui-darwin-arm64",
        ],
    }
}

fn artifact_path(input: &Path, name: &str) -> PathBuf {
    input.join(name).join(name)
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 16 * 1024];

    loop {
        let bytes_read = file
            .read(&mut buf)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buf[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .with_context(|| format!("failed to read metadata {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("failed to chmod {}", path.display()))?;
    }
    Ok(())
}
