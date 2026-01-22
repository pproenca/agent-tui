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
    cargo clippy --workspace -- -D warnings

# Run tests
test:
    cargo test --workspace

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
    cargo clippy -p {{crate}} -- -D warnings
