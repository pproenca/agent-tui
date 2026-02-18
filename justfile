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

# Run CI checks (format, clippy, architecture, tests, version).
ready:
    cargo run -p xtask -- ci

# Run Clean Architecture boundary checks and emit a dependency graph snapshot.
boundaries:
    @mkdir -p docs/architecture
    cargo run -p xtask -- architecture graph --format json > docs/architecture/dependencies.json
    cargo run -p xtask -- architecture check --verbose

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
    cargo test --workspace --lib --test cli_smoke --test cli_contracts --test cli_command_contracts

# Run slow, core-runtime command E2E coverage.
test-core-e2e:
    cargo test -p agent-tui --features slow-tests --test system_e2e

# Verify generated CLI docs are up-to-date.
check-cli-docs-sync:
    cargo run -p agent-tui --bin agent-tui-cli-docs
    git diff --exit-code -- ../docs/cli/agent-tui.md

# Verify skill docs do not reference unsupported CLI commands/flags.
check-skill-docs-sync:
    ../scripts/check-skill-docs-sync.sh

# Run tui-explorer skill unit tests.
check-tui-explorer:
    ../scripts/check-tui-explorer.sh

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
    @rm -rf crates/agent-tui-app/assets/web
    @mkdir -p crates/agent-tui-app/assets/web
    cp -a ../web/public/. crates/agent-tui-app/assets/web/

    # Keep facade crate mirror for legacy packaging/scripts.
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
release bump="patch":
    cargo run -p xtask -- release {{bump}}

# Internal: ensure bun is installed.
_ensure-bun:
    @command -v bun >/dev/null || { echo "bun is required for this recipe. Install bun and re-run."; exit 1; }

# Internal: ensure cargo-watch is installed.
_ensure-cargo-watch:
    @command -v cargo-watch >/dev/null || { echo "cargo-watch is required for watch. Install with 'cargo install cargo-watch' and re-run."; exit 1; }
