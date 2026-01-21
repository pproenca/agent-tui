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
    cd cli && cargo fmt --all

# Check formatting without modifying
format-check:
    cd cli && cargo fmt --all -- --check

# Run clippy lints
lint:
    cd cli && cargo lint

# Run tests
test:
    cd cli && cargo test

# Run tests with output
test-verbose:
    cd cli && cargo test -- --nocapture

# Build debug
build:
    cd cli && cargo build

# Build release
build-release:
    cd cli && cargo build --release

# Clean build artifacts
clean:
    cd cli && cargo clean

# Run the daemon in foreground
daemon:
    cd cli && cargo run -- daemon

# Run health check
health:
    cd cli && cargo run -- health

# Watch and rebuild on changes (requires cargo-watch)
watch:
    cd cli && cargo watch -x build

# Check for unused dependencies (requires cargo-udeps and nightly)
udeps:
    cd cli && cargo +nightly udeps --all-targets

# Update dependencies
update:
    cd cli && cargo update

# Generate documentation
doc:
    cd cli && cargo doc --no-deps --open
