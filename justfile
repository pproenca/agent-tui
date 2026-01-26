# agent-tui justfile
# Run `just` for help, `just <recipe>` to execute

set shell := ["bash", "-uc"]
set working-directory := "cli"

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

# Run fast tests (unit + smoke integration)
test:
    cargo test --workspace --lib --test integration_smoke_tests

# Run unit tests only (~2s)
test-unit:
    cargo test --lib --workspace

# Run smoke integration tests (fast)
test-integration:
    cargo test --test integration_smoke_tests

# Run slow integration/E2E suite (feature-gated)
test-slow:
    cargo test --test slow_tests --features slow-tests

# Prep for nextest tiers (CI hook to be wired later)
test-fast-nextest:
    cargo nextest run --workspace --filter-set 'expr(not test-type(system))'

test-system-nextest:
    cargo nextest run --workspace --filter-set 'expr(test-type(system))'

# Run E2E tests with real daemon (slow)
test-e2e:
    cargo test --test slow_tests --features slow-tests e2e_

# Run fast tests with output
test-verbose:
    cargo test --workspace --lib --test integration_smoke_tests -- --nocapture

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
