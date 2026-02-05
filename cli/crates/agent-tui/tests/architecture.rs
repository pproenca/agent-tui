//! Architecture rule tests.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_INNER_IMPORTS: &[&str] = &[
    "crate::infra",
    "crate::app",
    "crate::adapters",
    "tokio::",
    "axum::",
    "crossterm::",
    "tower_http::",
    "portable_pty::",
    "tracing::",
];

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn assert_no_forbidden_imports(layer: &str, root: &Path, forbidden: &[&str]) {
    let mut files = Vec::new();
    if let Err(err) = collect_rs_files(root, &mut files) {
        panic!("collect source files: {err}");
    }

    let mut violations = Vec::new();
    for file in files {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        for token in forbidden {
            if contents.contains(token) {
                violations.push(format!("{}: {}", file.display(), token));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Found forbidden dependencies in {} layer:\n{}",
        layer,
        violations.join("\n")
    );
}

#[test]
fn domain_layer_has_no_outward_dependencies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    assert_no_forbidden_imports("domain", &root, FORBIDDEN_INNER_IMPORTS);
}

#[test]
fn usecase_layer_has_no_outward_dependencies() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/usecases");
    assert_no_forbidden_imports("usecases", &root, FORBIDDEN_INNER_IMPORTS);
}

#[test]
fn adapter_layer_does_not_depend_on_app() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/adapters");
    assert_no_forbidden_imports("adapters", &root, &["crate::app"]);
}
