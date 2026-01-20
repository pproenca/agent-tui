//! E2E tests for CLI spawn and session commands
//!
//! These tests invoke the compiled binary as a subprocess and verify
//! exact output formats, exit codes, and behavior.
//!
//! ## Test Categories
//!
//! 1. **Argument parsing tests** - Test CLI argument handling without daemon
//! 2. **Help/version tests** - Test CLI info commands without daemon
//!
//! ## Running Integration Tests
//!
//! For full E2E tests against a real daemon:
//! ```
//! cargo test -- --ignored
//! ```

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Test --help shows usage information
#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-tui"))
        .stdout(predicate::str::contains("spawn"))
        .stdout(predicate::str::contains("snapshot"))
        .stdout(predicate::str::contains("click"))
        .stdout(predicate::str::contains("fill"))
        .stdout(predicate::str::contains("keystroke"));
}

/// Test --version shows version number
#[test]
fn test_version_output() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("agent-tui"));
}

/// Test spawn command help
#[test]
fn test_spawn_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["spawn", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Spawn a new TUI application"))
        .stdout(predicate::str::contains("--cols"))
        .stdout(predicate::str::contains("--rows"))
        .stdout(predicate::str::contains("--cwd"));
}

/// Test snapshot command help
#[test]
fn test_snapshot_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["snapshot", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot"))
        .stdout(predicate::str::contains("--elements"))
        .stdout(predicate::str::contains("--compact"));
}

/// Test click command help
#[test]
fn test_click_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["click", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Click"))
        .stdout(predicate::str::contains("<ref>"));
}

/// Test fill command help
#[test]
fn test_fill_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["fill", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fill"))
        .stdout(predicate::str::contains("<ref>"))
        .stdout(predicate::str::contains("<VALUE>"));
}

/// Test wait command help
#[test]
fn test_wait_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["wait", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Wait"))
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--stable"))
        .stdout(predicate::str::contains("--element"));
}

/// Test health command help
#[test]
fn test_health_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["health", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("daemon"));
}

/// Test invalid command shows error
#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("not-a-real-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test missing required argument for click
#[test]
fn test_click_missing_arg() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("click")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test missing required argument for fill
#[test]
fn test_fill_missing_args() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("fill")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test missing required argument for spawn
#[test]
fn test_spawn_missing_command() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.arg("spawn")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

/// Test invalid format option
#[test]
fn test_invalid_format() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["-f", "yaml", "health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'yaml'"));
}

/// Test scroll command with invalid direction
#[test]
fn test_invalid_scroll_direction() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["scroll", "diagonal"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

/// Test env command help
#[test]
fn test_env_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["env", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("environment"));
}

/// Test assert command help
#[test]
fn test_assert_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["assert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Assert"))
        .stdout(predicate::str::contains("CONDITION"));
}

/// Test find command help
#[test]
fn test_find_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["find", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Find"))
        .stdout(predicate::str::contains("--role"))
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--focused"));
}

/// Test trace command help
#[test]
fn test_trace_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["trace", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("trace"))
        .stdout(predicate::str::contains("--count"));
}

/// Test console command help
#[test]
fn test_console_help() {
    let mut cmd = Command::cargo_bin("agent-tui").unwrap();
    cmd.args(["console", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("console"))
        .stdout(predicate::str::contains("--lines"));
}
