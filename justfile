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
    cargo test --test integration_concurrent_tests --test integration_connection_failure_tests \
        --test integration_contracts_tests --test integration_daemon_no_autostart_tests \
        --test integration_dbl_click_tests --test integration_daemon_tests \
        --test integration_error_propagation_tests --test integration_lock_timeout_tests \
        --test integration_parameter_validation_tests \
        --test integration_response_edge_cases_tests --test integration_retry_mechanism_tests \
        --test integration_session_state_tests

# Prep for nextest tiers (CI hook to be wired later)
test-fast-nextest:
    cargo nextest run --workspace --filter-set 'expr(not test-type(system))'

test-system-nextest:
    cargo nextest run --workspace --filter-set 'expr(test-type(system))'

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
    cargo xtask release patch

# Release with minor version bump
release-minor:
    cargo xtask release minor

# Release with major version bump
release-major:
    cargo xtask release major

# Release with explicit version
release version:
    cargo xtask release {{version}}

# Install git hooks (pre-push checks)
setup-hooks:
    cargo xtask hooks install

# Run ast-grep rule tests
ast-grep-test:
    ast-grep test

# Scan codebase with ast-grep rules
ast-grep-scan:
    @for rule in rules/rust-codemods/**/*.yml rules/clean-architecture/*.yml; do \
        ast-grep scan --rule "$$rule"; \
    done

# Build Docker image for E2E tests (multi-stage build compiles in container)
docker-build:
    docker build -t agent-tui-e2e -f docker/Dockerfile .

# Run E2E tests in Docker container
docker-test: docker-build
    docker run --rm agent-tui-e2e

# Start interactive shell in Docker container for debugging
docker-shell: docker-build
    docker run --rm -it --entrypoint /bin/bash agent-tui-e2e
