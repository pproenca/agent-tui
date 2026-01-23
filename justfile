# agent-tui justfile
# Run `just` for help, `just <recipe>` to execute

set shell := ["bash", "-uc"]

# Default recipe - show help
default:
    @just --list

# Run all checks (pre-commit)
ready: format-check lint test
    @echo "All checks passed!"

# Format code
format:
    cargo fmt --all

# Check formatting without modifying
format-check:
    cargo fmt --all -- --check

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run tests
test:
    cargo test --workspace

# Run unit tests only (~2s)
test-unit:
    cargo test --lib --workspace

# Run integration tests with mock daemon (~8s)
test-integration:
    cargo test --test concurrent_tests --test connection_failure_tests \
        --test dbl_click_tests --test e2e_daemon_tests \
        --test error_propagation_tests --test lock_timeout_tests \
        --test parameter_validation_tests --test pty_operations_tests \
        --test response_edge_cases_tests --test retry_mechanism_tests \
        --test session_state_tests

# Run E2E tests with real daemon (~31s)
test-e2e:
    cargo test --test e2e_workflow_tests

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Build debug
build:
    cargo build --workspace

# Build release
build-release:
    cargo build --workspace --release

# Clean build artifacts
clean:
    cargo clean

# Run the daemon in foreground
daemon:
    cargo run -p agent-tui -- daemon

# Run health check
health:
    cargo run -p agent-tui -- health

# Watch and rebuild on changes (requires cargo-watch)
watch:
    cargo watch -x "build --workspace"

# Check for unused dependencies (requires cargo-udeps and nightly)
udeps:
    cargo +nightly udeps --workspace --all-targets

# Update dependencies
update:
    cargo update

# Generate documentation
doc:
    cargo doc --workspace --no-deps --open

# Build a specific crate
build-crate crate:
    cargo build -p {{crate}}

# Test a specific crate
test-crate crate:
    cargo test -p {{crate}}

# Lint a specific crate
lint-crate crate:
    cargo clippy -p {{crate}} --all-targets -- -D warnings

# Release with patch version bump
release-patch:
    ./scripts/release.sh patch

# Release with minor version bump
release-minor:
    ./scripts/release.sh minor

# Release with major version bump
release-major:
    ./scripts/release.sh major

# Release with explicit version
release version:
    ./scripts/release.sh {{version}}

# Install git hooks (pre-push checks)
setup-hooks:
    ./scripts/setup-hooks.sh
