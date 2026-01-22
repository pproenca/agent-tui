//! E2E CLI command tests using trycmd
//!
//! These tests verify the CLI interface contract independent of implementation.
//! They test the actual binary against the real daemon (auto-started).
//!
//! ## How it works
//!
//! trycmd uses markdown files in `tests/cmd/*.md` to define test cases.
//! Each file can contain multiple shell commands and their expected outputs.
//!
//! ## Fuzzy matching
//!
//! - `[..]` - matches any characters (for session IDs, PIDs, timestamps)
//! - `...` - matches multiple lines
//! - `[ROOT]` - matches absolute paths
//!
//! ## Running tests
//!
//! ```bash
//! cargo test --test cmd_e2e_tests
//! ```
//!
//! ## Updating snapshots
//!
//! ```bash
//! TRYCMD=dump cargo test --test cmd_e2e_tests
//! ```

#[test]
fn cli_e2e_tests() {
    // trycmd will find test cases in tests/cmd/*.md and tests/cmd/*.toml
    trycmd::TestCases::new()
        .case("tests/cmd/*.md")
        .case("tests/cmd/*.toml");
}
