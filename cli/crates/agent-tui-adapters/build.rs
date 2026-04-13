use std::env;
use std::process::Command;

fn strict_build_metadata() -> bool {
    matches!(env::var("PROFILE").ok().as_deref(), Some("release"))
        || env::var("AGENT_TUI_STRICT_BUILD_METADATA")
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes"
                )
            })
            .unwrap_or(false)
}

fn unknown_metadata(message: &str) -> String {
    if strict_build_metadata() {
        panic!("{message}; release builds must embed verifiable build metadata");
    }

    println!("cargo:warning={message}; embedding 'unknown' build metadata");
    "unknown".to_string()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_VERSION");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_GIT_SHA");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_STRICT_BUILD_METADATA");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| String::new());

    let version = env::var("AGENT_TUI_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("CARGO_PKG_VERSION").ok())
        .unwrap_or_else(|| unknown_metadata("AGENT_TUI_VERSION and CARGO_PKG_VERSION unavailable"));

    let git_sha = env::var("AGENT_TUI_GIT_SHA")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=12", "HEAD"])
                .current_dir(&manifest_dir)
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| {
            unknown_metadata("AGENT_TUI_GIT_SHA unavailable and git rev-parse failed")
        });

    println!("cargo:rustc-env=AGENT_TUI_VERSION={version}");
    println!("cargo:rustc-env=AGENT_TUI_GIT_SHA={git_sha}");
}
