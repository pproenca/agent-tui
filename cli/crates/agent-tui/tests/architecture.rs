//! Workspace architecture rule tests.
#![allow(clippy::expect_used)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn metadata_json() -> serde_json::Value {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");

    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("valid cargo metadata JSON")
}

#[test]
fn expected_layer_crates_exist_and_legacy_tree_is_gone() {
    let root = workspace_root();

    for path in [
        "crates/agent-tui-common/src/common",
        "crates/agent-tui-domain/src/domain",
        "crates/agent-tui-usecases/src/usecases",
        "crates/agent-tui-adapters/src/adapters",
        "crates/agent-tui-infra/src/infra",
        "crates/agent-tui-app/src/app",
        "crates/xtask/src",
    ] {
        assert!(root.join(path).exists(), "missing expected path: {path}");
    }

    for legacy_path in [
        "crates/agent-tui/src/common",
        "crates/agent-tui/src/domain",
        "crates/agent-tui/src/usecases",
        "crates/agent-tui/src/adapters",
        "crates/agent-tui/src/infra",
        "crates/agent-tui/src/app",
        "rules",
        "sgconfig.yml",
        "scripts/xtask.ts",
        "scripts/check-architecture.sh",
    ] {
        assert!(
            !root.join(legacy_path).exists(),
            "legacy path should not exist: {legacy_path}"
        );
    }
}

#[test]
fn internal_crate_dependencies_follow_allowed_matrix() {
    let metadata = metadata_json();
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .expect("packages array");

    let mut id_to_name = HashMap::new();
    let mut names = HashSet::new();

    for package in packages {
        let id = package
            .get("id")
            .and_then(serde_json::Value::as_str)
            .expect("package id")
            .to_string();
        let name = package
            .get("name")
            .and_then(serde_json::Value::as_str)
            .expect("package name")
            .to_string();
        id_to_name.insert(id, name.clone());
        names.insert(name);
    }

    let internal = HashSet::from([
        "agent-tui-common",
        "agent-tui-domain",
        "agent-tui-usecases",
        "agent-tui-adapters",
        "agent-tui-infra",
        "agent-tui-app",
        "agent-tui",
    ]);

    for expected in &internal {
        assert!(names.contains(*expected), "missing crate: {expected}");
    }

    let allowed: HashMap<&str, HashSet<&str>> = HashMap::from([
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
    ]);

    let resolve_nodes = metadata
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(serde_json::Value::as_array)
        .expect("resolve.nodes array");

    for node in resolve_nodes {
        let source_id = node
            .get("id")
            .and_then(serde_json::Value::as_str)
            .expect("node id");
        let source = id_to_name
            .get(source_id)
            .expect("source name available")
            .as_str();

        if !internal.contains(source) {
            continue;
        }

        let allowed_targets = allowed
            .get(source)
            .expect("allowed targets configured for internal crate");

        let deps = node
            .get("deps")
            .and_then(serde_json::Value::as_array)
            .expect("node deps array");

        for dep in deps {
            let target_id = dep
                .get("pkg")
                .and_then(serde_json::Value::as_str)
                .expect("dep pkg id");
            let target = id_to_name
                .get(target_id)
                .expect("target name available")
                .as_str();

            if !internal.contains(target) {
                continue;
            }

            assert!(
                allowed_targets.contains(target),
                "forbidden internal dependency: {source} -> {target}"
            );
        }
    }
}
