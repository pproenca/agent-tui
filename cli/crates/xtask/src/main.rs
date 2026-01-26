use clap::{Args, Parser, Subcommand};
use regex::Regex;
use semver::Version;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use toml_edit::{DocumentMut, value};
use walkdir::WalkDir;

static LEGACY_SHIM_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"crate::daemon::|crate::ipc::|crate::terminal::|crate::core::|crate::commands::|crate::handlers::|crate::presenter::|crate::error::|crate::attach::",
    )
    .expect("Invalid legacy shim regex")
});

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Version and release metadata helpers.
    Version(VersionArgs),
    /// Prepare a release commit and tag.
    Release(ReleaseArgs),
    /// Install or manage git hooks.
    Hooks(HooksArgs),
    /// Run local pre-push checks.
    PrePush,
    /// Architecture checks.
    Architecture(ArchitectureArgs),
    /// Prepare distribution artifacts.
    Dist(DistArgs),
}

#[derive(Args)]
struct VersionArgs {
    #[command(subcommand)]
    command: VersionCommand,
}

#[derive(Subcommand)]
enum VersionCommand {
    /// Verify Cargo.toml and package.json versions match.
    Check {
        /// Suppress success output.
        #[arg(long)]
        quiet: bool,
    },
    /// Print the current version.
    Current,
    /// Verify versions match the provided tag (e.g. v1.2.3).
    AssertTag {
        /// Tag name (e.g. v1.2.3 or refs/tags/v1.2.3).
        tag: String,
    },
    /// Verify versions match the provided input version.
    AssertInput {
        /// Input version (e.g. 1.2.3).
        version: String,
    },
}

#[derive(Args)]
struct ReleaseArgs {
    /// Version to set (semver) or bump type (major|minor|patch).
    version_or_bump: String,
}

#[derive(Args)]
struct HooksArgs {
    #[command(subcommand)]
    command: HooksCommand,
}

#[derive(Subcommand)]
enum HooksCommand {
    /// Install the git pre-push hook.
    Install,
}

#[derive(Args)]
struct ArchitectureArgs {
    #[command(subcommand)]
    command: ArchitectureCommand,
}

#[derive(Subcommand)]
enum ArchitectureCommand {
    /// Run architecture checks.
    Check,
}

#[derive(Args)]
struct DistArgs {
    #[command(subcommand)]
    command: DistCommand,
}

#[derive(Subcommand)]
enum DistCommand {
    /// Prepare release assets and checksums from build artifacts.
    Release {
        /// Input artifacts directory.
        #[arg(long, default_value = "artifacts")]
        input: PathBuf,
        /// Output directory for release assets.
        #[arg(long, default_value = "release")]
        output: PathBuf,
    },
    /// Verify required artifacts exist.
    Verify {
        /// Input artifacts directory.
        #[arg(long, default_value = "artifacts")]
        input: PathBuf,
        /// Artifact set to validate.
        #[arg(long, value_enum, default_value = "release")]
        kind: DistKind,
    },
    /// Prepare npm bin/ folder from build artifacts.
    Npm {
        /// Input artifacts directory.
        #[arg(long, default_value = "artifacts")]
        input: PathBuf,
        /// Output directory for npm binaries.
        #[arg(long, default_value = "npm")]
        output: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum DistKind {
    Release,
    Npm,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}: error: {err}", env!("CARGO_PKG_NAME"));
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version(args) => match args.command {
            VersionCommand::Check { quiet } => version_check(quiet),
            VersionCommand::Current => {
                let root = repo_root();
                let version = read_package_version(&package_json_path(&root))?;
                println!("{version}");
                Ok(())
            }
            VersionCommand::AssertTag { tag } => assert_tag(&tag),
            VersionCommand::AssertInput { version } => assert_input(&version),
        },
        Commands::Release(args) => release(&args.version_or_bump),
        Commands::Hooks(args) => match args.command {
            HooksCommand::Install => install_hooks(),
        },
        Commands::PrePush => pre_push(),
        Commands::Architecture(args) => match args.command {
            ArchitectureCommand::Check => architecture_check(),
        },
        Commands::Dist(args) => match args.command {
            DistCommand::Release { input, output } => dist_release(&input, &output),
            DistCommand::Verify { input, kind } => dist_verify(&input, kind),
            DistCommand::Npm { input, output } => dist_npm(&input, &output),
        },
    }
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("xtask should live under crates/")
        .to_path_buf()
}

fn cargo_toml_path(root: &Path) -> PathBuf {
    root.join("Cargo.toml")
}

fn package_json_path(root: &Path) -> PathBuf {
    root.join("package.json")
}

fn npm_platform_package_paths(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let npm_root = root.join("npm");
    if !npm_root.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(npm_root)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let package_json = path.join("package.json");
        if package_json.exists() {
            paths.push(package_json);
        }
    }

    paths.sort();
    Ok(paths)
}

fn read_versions(root: &Path) -> Result<(String, String), Box<dyn Error>> {
    let cargo_version = read_cargo_version(&cargo_toml_path(root))?;
    let package_version = read_package_version(&package_json_path(root))?;
    Ok((cargo_version, package_version))
}

fn read_cargo_version(path: &Path) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let doc = contents.parse::<DocumentMut>()?;

    if let Some(version) = doc["workspace"]["package"]["version"].as_str() {
        return Ok(version.to_string());
    }

    if let Some(version) = doc["package"]["version"].as_str() {
        return Ok(version.to_string());
    }

    Err("Could not find version in Cargo.toml".into())
}

fn write_cargo_version(path: &Path, version: &str) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut doc = contents.parse::<DocumentMut>()?;

    if doc["workspace"]["package"]["version"].is_none() && doc["package"]["version"].is_none() {
        return Err("Could not find version in Cargo.toml".into());
    }

    if doc["workspace"]["package"]["version"].is_none() {
        doc["package"]["version"] = value(version);
    } else {
        doc["workspace"]["package"]["version"] = value(version);
    }

    fs::write(path, doc.to_string())?;
    Ok(())
}

fn read_package_version(path: &Path) -> Result<String, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;
    let version = json
        .get("version")
        .and_then(|value| value.as_str())
        .ok_or("Could not find version in package.json")?;
    Ok(version.to_string())
}

fn write_package_version(path: &Path, version: &str) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut json: serde_json::Value = serde_json::from_str(&contents)?;
    let object = json
        .as_object_mut()
        .ok_or("Expected package.json to be an object")?;

    object.insert(
        "version".to_string(),
        serde_json::Value::String(version.to_string()),
    );
    let output = serde_json::to_string_pretty(&json)?;
    fs::write(path, format!("{output}\n"))?;
    Ok(())
}

fn version_check(quiet: bool) -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let (cargo_version, package_version) = read_versions(&root)?;
    let npm_paths = npm_platform_package_paths(&root)?;

    if cargo_version == package_version {
        for path in npm_paths {
            let npm_version = read_package_version(&path)?;
            if npm_version != package_version {
                return Err(format!(
                    "Version mismatch detected!\n  package.json: {package_version}\n  {}: {npm_version}",
                    path.display()
                )
                .into());
            }
        }
        if !quiet {
            println!("Version check passed: {cargo_version}");
        }
        Ok(())
    } else {
        Err(format!(
            "Version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        )
        .into())
    }
}

fn normalize_tag(tag: &str) -> &str {
    let tag = tag.strip_prefix("refs/tags/").unwrap_or(tag);
    tag.strip_prefix('v').unwrap_or(tag)
}

fn assert_tag(tag: &str) -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let (cargo_version, package_version) = read_versions(&root)?;
    let tag_version = normalize_tag(tag);

    if cargo_version != package_version {
        return Err(format!(
            "Version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        )
        .into());
    }

    if cargo_version != tag_version {
        return Err(format!(
            "Version mismatch between Cargo.toml ({cargo_version}) and tag ({tag})"
        )
        .into());
    }

    Ok(())
}

fn assert_input(version: &str) -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let (cargo_version, package_version) = read_versions(&root)?;

    if cargo_version != package_version {
        return Err(format!(
            "Version mismatch detected!\n  Cargo.toml:   {cargo_version}\n  package.json: {package_version}"
        )
        .into());
    }

    if cargo_version != version {
        return Err(format!(
            "Version mismatch between Cargo.toml ({cargo_version}) and input ({version})"
        )
        .into());
    }

    Ok(())
}

fn release(version_or_bump: &str) -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let cargo_path = cargo_toml_path(&root);
    let package_path = package_json_path(&root);
    let npm_paths = npm_platform_package_paths(&root)?;

    let current_version = read_cargo_version(&cargo_path)?;
    let target_version = if matches!(version_or_bump, "major" | "minor" | "patch") {
        let current = Version::parse(&current_version)?;
        bump_version(&current, version_or_bump).to_string()
    } else {
        Version::parse(version_or_bump)?;
        version_or_bump.to_string()
    };

    let tag = format!("v{target_version}");

    ensure_git_clean(&root)?;
    ensure_tag_absent(&root, &tag)?;

    println!("Releasing version {target_version}...");

    println!("Updating package.json...");
    write_package_version(&package_path, &target_version)?;
    for path in &npm_paths {
        write_package_version(path, &target_version)?;
    }

    println!("Updating Cargo.toml...");
    write_cargo_version(&cargo_path, &target_version)?;

    println!("Staging changes...");
    let mut git_add = command_in_root(&root, "git");
    git_add.arg("add").arg(&package_path).arg(&cargo_path);
    for path in npm_paths {
        git_add.arg(path);
    }
    run_command(git_add)?;

    println!("Committing...");
    let mut git_commit = command_in_root(&root, "git");
    git_commit
        .arg("commit")
        .arg("-m")
        .arg(format!("chore: bump version to {target_version}"));
    run_command(git_commit)?;

    println!("Creating tag {tag}...");
    let mut git_tag = command_in_root(&root, "git");
    git_tag
        .arg("tag")
        .arg("-a")
        .arg(&tag)
        .arg("-m")
        .arg(format!("Release {target_version}"));
    run_command(git_tag)?;

    println!("Done! Release {target_version} prepared.");
    println!("To publish, run:");
    println!("  git push && git push --tags");

    Ok(())
}

fn bump_version(current: &Version, bump: &str) -> Version {
    match bump {
        "major" => Version::new(current.major + 1, 0, 0),
        "minor" => Version::new(current.major, current.minor + 1, 0),
        "patch" => Version::new(current.major, current.minor, current.patch + 1),
        _ => current.clone(),
    }
}

fn ensure_git_clean(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut diff = Command::new("git");
    let diff_status = diff.arg("diff").arg("--quiet").current_dir(root).status()?;
    if !diff_status.success() {
        return Err("You have uncommitted changes. Please commit or stash them first.".into());
    }

    let mut diff_cached = Command::new("git");
    let diff_cached_status = diff_cached
        .arg("diff")
        .arg("--cached")
        .arg("--quiet")
        .current_dir(root)
        .status()?;
    if !diff_cached_status.success() {
        return Err("You have staged changes. Please commit or stash them first.".into());
    }

    Ok(())
}

fn ensure_tag_absent(root: &Path, tag: &str) -> Result<(), Box<dyn Error>> {
    let status = Command::new("git")
        .arg("rev-parse")
        .arg(tag)
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        return Err(format!("Tag {tag} already exists").into());
    }
    Ok(())
}

fn install_hooks() -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let hooks_dir = root.join(".git/hooks");
    if !hooks_dir.exists() {
        return Err(".git/hooks directory not found. Are you in a git repository?".into());
    }

    let hook_path = hooks_dir.join("pre-push");
    let contents = "#!/bin/sh\nset -e\n\ncargo xtask pre-push\n";
    fs::write(&hook_path, contents)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    println!("Installed pre-push hook at {}", hook_path.display());
    Ok(())
}

fn pre_push() -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    println!("Running pre-push checks...");

    run_step("Checking formatting", || {
        let mut cmd = command_in_root(&root, "cargo");
        cmd.arg("fmt").arg("--all").arg("--").arg("--check");
        run_command(cmd)
    })?;

    run_step("Running clippy", || {
        let mut cmd = command_in_root(&root, "cargo");
        cmd.arg("clippy")
            .arg("--workspace")
            .arg("--all-targets")
            .arg("--")
            .arg("-D")
            .arg("warnings");
        run_command(cmd)
    })?;

    if has_command("ast-grep") {
        run_step("Running ast-grep Clean Architecture checks", || {
            let mut cmd = command_in_root(&root, "ast-grep");
            cmd.arg("scan").arg("--config").arg("sgconfig.yml");
            run_command(cmd)
        })?;
    } else {
        println!("ast-grep not installed, skipping...");
    }

    run_step("Running architecture checks", architecture_check)?;

    run_step("Running tests", || {
        let mut cmd = command_in_root(&root, "cargo");
        cmd.arg("test").arg("--workspace");
        run_command(cmd)
    })?;

    if has_command("cargo-machete") {
        run_step("Checking for unused dependencies", || {
            let mut cmd = command_in_root(&root, "cargo-machete");
            cmd.arg("--skip-target-dir").arg(".");
            if let Err(err) = run_command(cmd) {
                println!("cargo-machete failed, skipping: {err}");
            }
            Ok(())
        })?;
    } else {
        println!("cargo-machete not installed, skipping...");
    }

    run_step("Checking version consistency", || version_check(true))?;

    println!("All pre-push checks passed!");
    Ok(())
}

fn run_step<F>(label: &str, action: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    println!("\n→ {label}...");
    action()
}

fn command_in_root(root: &Path, program: &str) -> Command {
    let mut command = Command::new(program);
    command.current_dir(root);
    command
}

fn run_command(mut command: Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command failed: {:?}", command).into())
    }
}

fn has_command(name: &str) -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join(name);
            if candidate.is_file() {
                return true;
            }
        }

        if cfg!(windows) {
            if let Some(exts) = std::env::var_os("PATHEXT") {
                for ext in std::env::split_paths(&exts) {
                    let ext = ext.to_string_lossy();
                    if ext.is_empty() {
                        continue;
                    }
                    for path in std::env::split_paths(&paths) {
                        let candidate = path.join(format!("{name}{ext}"));
                        if candidate.is_file() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn architecture_check() -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let src_root = root.join("crates/agent-tui/src");

    if let Some(hit) = find_first_match(&src_root, |_, line| LEGACY_SHIM_REGEX.is_match(line))? {
        return Err(format!(
            "Architecture check failed: legacy shim paths detected at {}",
            hit
        )
        .into());
    }

    if let Some(hit) = find_first_match(&src_root, |path, line| {
        line.contains("std::process::exit") && !path.ends_with("main.rs")
    })? {
        return Err(format!(
            "Architecture check failed: std::process::exit is only allowed in main.rs (found at {hit})"
        )
        .into());
    }

    let domain_root = src_root.join("domain");
    if let Some(hit) = find_first_match(&domain_root, |_, line| line.contains("serde_json"))? {
        return Err(format!(
            "Architecture check failed: serde_json usage detected in domain (found at {hit})"
        )
        .into());
    }

    let usecases_root = src_root.join("usecases");
    if let Some(hit) = find_first_match(&usecases_root, |_, line| line.contains("serde_json"))? {
        return Err(format!(
            "Architecture check failed: serde_json usage detected in usecases (found at {hit})"
        )
        .into());
    }

    if let Some(hit) = find_first_match(&usecases_root, |path, line| {
        if path.ends_with("ports/errors.rs") {
            return false;
        }
        if !line.contains("crate::infra::") {
            return false;
        }
        !line.contains("crate::infra::daemon::test_support")
    })? {
        return Err(format!(
            "Architecture check failed: infra dependency detected in usecases (found at {hit})"
        )
        .into());
    }

    println!("Architecture checks passed.");
    Ok(())
}

fn dist_verify(input: &Path, kind: DistKind) -> Result<(), Box<dyn Error>> {
    if !input.exists() {
        return Err(format!("Artifacts directory not found: {}", input.display()).into());
    }

    let mut missing = Vec::new();
    for name in required_artifacts(kind) {
        let path = artifact_path(input, name);
        if !path.exists() {
            missing.push(path.display().to_string());
        }
    }

    if !missing.is_empty() {
        return Err(format!("Missing required artifacts:\n{}", missing.join("\n")).into());
    }

    println!("All required artifacts present for {:?}.", kind);
    Ok(())
}

fn dist_release(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    dist_verify(input, DistKind::Release)?;

    if !input.exists() {
        return Err(format!("Artifacts directory not found: {}", input.display()).into());
    }

    fs::create_dir_all(output)?;

    let mut seen = std::collections::HashSet::new();
    for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid artifact filename")?;
        if !seen.insert(file_name.to_string()) {
            return Err(format!("Duplicate artifact filename: {file_name}").into());
        }
        let dest = output.join(file_name);
        fs::copy(path, &dest)?;
        make_executable(&dest)?;
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(output)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }
        if path.file_name() == Some(OsStr::new("checksums-sha256.txt")) {
            continue;
        }
        files.push(path);
    }
    files.sort_unstable();

    let mut checksums = String::new();
    for path in files {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Invalid release filename")?;
        let digest = sha256_file(&path)?;
        checksums.push_str(&format!("{digest}  {file_name}\n"));
    }

    let checksum_path = output.join("checksums-sha256.txt");
    fs::write(&checksum_path, checksums)?;

    println!("Prepared release assets in {}", output.display());
    Ok(())
}

fn dist_npm(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    if !input.exists() {
        return Err(format!("Artifacts directory not found: {}", input.display()).into());
    }

    for name in required_artifacts(DistKind::Npm) {
        let path = artifact_path(input, name);
        if !path.exists() {
            return Err(format!("Missing artifact: {}", path.display()).into());
        }
        let package_dir = output.join(name);
        let package_json = package_dir.join("package.json");
        if !package_json.exists() {
            return Err(format!("Missing npm package: {}", package_json.display()).into());
        }

        let bin_dir = package_dir.join("bin");
        fs::create_dir_all(&bin_dir)?;
        let dest = bin_dir.join("agent-tui");
        fs::copy(&path, &dest)?;
        make_executable(&dest)?;
    }

    println!("Prepared npm platform packages in {}", output.display());
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn make_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
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

fn find_first_match<F>(root: &Path, matcher: F) -> Result<Option<String>, Box<dyn Error>>
where
    F: Fn(&Path, &str) -> bool,
{
    if !root.exists() {
        return Ok(None);
    }

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        for (idx, line) in contents.lines().enumerate() {
            if matcher(path, line) {
                let display = path.strip_prefix(root).unwrap_or(path).display();
                return Ok(Some(format!("{display}:{}", idx + 1)));
            }
        }
    }

    Ok(None)
}
