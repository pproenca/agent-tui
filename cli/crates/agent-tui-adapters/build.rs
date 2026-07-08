use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type BuildResult<T> = Result<T, Box<dyn Error>>;

fn strict_build_metadata(manifest_dir: &Path) -> bool {
    env::var("AGENT_TUI_STRICT_BUILD_METADATA")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(false)
        || (matches!(env::var("PROFILE").ok().as_deref(), Some("release"))
            && has_git_metadata(manifest_dir))
}

fn has_git_metadata(manifest_dir: &Path) -> bool {
    manifest_dir
        .ancestors()
        .any(|ancestor| ancestor.join(".git").exists())
}

fn unknown_metadata(message: &str, manifest_dir: &Path) -> BuildResult<String> {
    if strict_build_metadata(manifest_dir) {
        return Err(format!(
            "{message}; release builds from git checkouts must embed verifiable build metadata"
        )
        .into());
    }

    println!("cargo:warning={message}; embedding 'packaged-source' build metadata");
    Ok("packaged-source".to_string())
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn git_rev_parse(manifest_dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(manifest_dir)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty())
}

fn packaged_vcs_sha(manifest_dir: &Path) -> Option<String> {
    let contents = fs::read_to_string(manifest_dir.join(".cargo_vcs_info.json")).ok()?;
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix("\"sha1\":")?;
        let value = rest.trim().trim_end_matches(',').trim();
        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            Some(value[1..value.len() - 1].to_string())
        } else {
            None
        }
    })
}

fn main() -> BuildResult<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=.cargo_vcs_info.json");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_VERSION");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_GIT_SHA");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_STRICT_BUILD_METADATA");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| String::new());
    let manifest_dir = Path::new(&manifest_dir);

    let version = non_empty_env("AGENT_TUI_VERSION")
        .or_else(|| non_empty_env("CARGO_PKG_VERSION"))
        .map(Ok)
        .unwrap_or_else(|| {
            unknown_metadata(
                "AGENT_TUI_VERSION and CARGO_PKG_VERSION unavailable",
                manifest_dir,
            )
        })?;

    let git_sha = non_empty_env("AGENT_TUI_GIT_SHA")
        .or_else(|| git_rev_parse(manifest_dir))
        .or_else(|| packaged_vcs_sha(manifest_dir))
        .map(Ok)
        .unwrap_or_else(|| {
            unknown_metadata(
                "AGENT_TUI_GIT_SHA unavailable and git/package metadata lookup failed",
                manifest_dir,
            )
        })?;

    println!("cargo:rustc-env=AGENT_TUI_VERSION={version}");
    println!("cargo:rustc-env=AGENT_TUI_GIT_SHA={git_sha}");
    Ok(())
}
