# agent-tui justfile
# Run `just` for help, `just <recipe>` to execute

# Use bash strict mode for reliable failures and pipe handling.
set shell := ["bash", "-euo", "pipefail", "-c"]
set working-directory := "cli"

# Default recipe - show help in file order
default:
    @just --list --unsorted

# Primary commands (DHH-style: a short, opinionated list)
# Start the daemon in dev mode.
dev:
    cargo run -p agent-tui -- daemon

# Run CLI health check.
health:
    cargo run -p agent-tui -- health

# Run CI checks (format, clippy, architecture, tests, version).
ready: _ensure-bun
    bun scripts/xtask.ts ci

# Run Clean Architecture boundary checks and emit a dependency graph snapshot.
boundaries: _ensure-bun
    @mkdir -p docs/architecture
    bun scripts/xtask.ts architecture graph --format json > docs/architecture/dependencies.json
    bun scripts/xtask.ts architecture check --verbose

# Format Rust code.
format:
    cargo fmt --all

# Verify formatting.
format-check:
    cargo fmt --all -- --check

# Lint Rust workspace.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run test suite.
test:
    cargo test --workspace --lib --test cli_smoke --test cli_contracts

# Build Rust workspace (ensures embedded web UI is fresh).
build: web-sync
    cargo build --workspace

# Release build (ensures embedded web UI is fresh).
build-release: web-sync
    cargo build --workspace --release

# Install web UI dependencies.
web-install: _ensure-bun
    (cd ../web && bun install)

# Build web UI.
web-build: _ensure-bun
    (cd ../web && bun install && bun run build)

# Build web UI and sync it into the CLI embedded assets.
web-sync: web-build
    @rm -rf crates/agent-tui/assets/web
    @mkdir -p crates/agent-tui/assets/web
    cp -a ../web/public/. crates/agent-tui/assets/web/

# Clean build artifacts.
clean:
    cargo clean

# Build and open docs.
doc:
    cargo doc --workspace --no-deps --open

# Generate clap-based CLI docs.
cli-docs:
    cargo run -p agent-tui --bin agent-tui-cli-docs

# Watch Rust workspace and rebuild on changes (requires cargo-watch).
watch: _ensure-cargo-watch
    cargo watch -x "build"

# Create and push a release tag (no version bump commit).
# Usage: just release bump=patch|minor|major|x.y.z
release bump="patch": _ensure-bun
    bun scripts/xtask.ts release {{bump}}

# Internal: ensure bun is installed.
_ensure-bun:
    @command -v bun >/dev/null || { echo "bun is required for this recipe. Install bun and re-run."; exit 1; }

# Internal: ensure cargo-watch is installed.
_ensure-cargo-watch:
    @command -v cargo-watch >/dev/null || { echo "cargo-watch is required for watch. Install with 'cargo install cargo-watch' and re-run."; exit 1; }
