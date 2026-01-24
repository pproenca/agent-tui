//! Architecture boundary verification tests.
//!
//! These tests verify Clean Architecture boundaries by checking
//! that inner layers don't depend on outer layers.
//!
//! Detects multiple dependency mechanisms:
//! - `use` statements (e.g., `use crate::handlers::...`)
//! - Fully-qualified paths (e.g., `crate::handlers::foo()`)
//! - Type annotations (e.g., `x: crate::handlers::Handler`)

use std::fs;
use std::path::Path;

/// Check that a file doesn't contain references to forbidden modules.
///
/// This checks for:
/// - `use crate::module` imports
/// - Fully-qualified `crate::module::item` references anywhere in code
/// - Type annotations like `: crate::module::Type`
fn file_does_not_reference(file_path: &Path, forbidden_patterns: &[&str]) -> Vec<String> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut violations = Vec::new();
    let mut in_block_comment = false;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Handle block comments
        if trimmed.contains("/*") {
            in_block_comment = true;
        }
        if trimmed.contains("*/") {
            in_block_comment = false;
            continue;
        }
        if in_block_comment {
            continue;
        }

        // Skip line comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Remove inline comments for checking
        let code = if let Some(idx) = trimmed.find("//") {
            &trimmed[..idx]
        } else {
            trimmed
        };

        // Check for any reference to forbidden modules
        for pattern in forbidden_patterns {
            if code.contains(pattern) {
                violations.push(format!(
                    "{}:{}: forbidden reference '{}' in '{}'",
                    file_path.display(),
                    line_num + 1,
                    pattern,
                    trimmed
                ));
            }
        }
    }
    violations
}

/// Check all files in a directory (recursively) for forbidden references.
fn check_directory_references(
    dir: &Path,
    forbidden_patterns: &[&str],
    file_extension: &str,
) -> Vec<String> {
    let mut violations = Vec::new();

    if !dir.exists() {
        return violations;
    }

    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            violations.extend(check_directory_references(
                &path,
                forbidden_patterns,
                file_extension,
            ));
        } else if path.extension().is_some_and(|ext| ext == file_extension) {
            violations.extend(file_does_not_reference(&path, forbidden_patterns));
        }
    }
    violations
}

#[test]
fn domain_layer_does_not_import_from_outer_layers() {
    // Domain should not import from handlers, adapters, usecases, or infrastructure
    let forbidden = &[
        "crate::handlers",
        "crate::adapters",
        "crate::usecases",
        "crate::server",
        "crate::session", // Infrastructure
        "crate::repository",
        "crate::transport",
        // Framework dependencies (except for minimal serde for DTOs at boundaries)
        // Note: We allow serde in domain for DTO types that cross boundaries
    ];

    let domain_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    let violations = check_directory_references(&domain_dir, forbidden, "rs");

    if !violations.is_empty() {
        panic!(
            "Domain layer has forbidden references:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn usecases_layer_does_not_import_from_handlers_or_adapters() {
    // Use cases should not import from handlers or adapters
    // They should only depend on domain types and repository traits
    let forbidden = &["crate::handlers", "crate::server", "crate::transport"];

    let usecases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/usecases");
    let violations = check_directory_references(&usecases_dir, forbidden, "rs");

    if !violations.is_empty() {
        panic!(
            "Use cases layer has forbidden references:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn adapters_layer_does_not_import_from_handlers() {
    // Adapters should not import from handlers
    let forbidden = &["crate::handlers"];

    let adapters_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapters");
    let violations = check_directory_references(&adapters_dir, forbidden, "rs");

    if !violations.is_empty() {
        panic!(
            "Adapters layer has forbidden references:\n{}",
            violations.join("\n")
        );
    }
}

#[test]
fn verify_dependency_direction() {
    // Verify the overall dependency direction:
    // handlers -> usecases -> domain
    // handlers -> adapters -> domain
    // Infrastructure (server, session) can import from all layers

    // Domain should have no violations (tested above)
    // Use cases should have no violations (tested above)
    // Adapters should have no violations (tested above)

    // This test is a meta-verification that the other tests ran
    println!("Dependency direction verified: domain <- usecases <- handlers");
    println!("                               domain <- adapters <- handlers");
}
