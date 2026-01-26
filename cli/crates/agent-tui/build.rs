use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_VERSION");
    println!("cargo:rerun-if-env-changed=AGENT_TUI_GIT_SHA");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "".to_string());

    let version = env::var("AGENT_TUI_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("CARGO_PKG_VERSION").ok())
        .unwrap_or_else(|| "unknown".to_string());

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
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=AGENT_TUI_VERSION={}", version);
    println!("cargo:rustc-env=AGENT_TUI_GIT_SHA={}", git_sha);
}
