# agent-tui justfile
# Run `just` for help, `just <recipe>` to execute

set shell := ["bash", "-uc"]
set working-directory := "cli"

# Default recipe - show help
default:
    @just --list

# Primary commands (DHH-style: a short, opinionated list)
dev:
    cargo run -p agent-tui -- daemon

health:
    cargo run -p agent-tui -- health

ready: format-check lint test
    @echo "All checks passed!"

format:
    cargo fmt --all

format-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --lib --test cli_smoke --test cli_contracts

build:
    @just web-install
    cargo build --workspace

build-release:
    @just web-install
    cargo build --workspace --release

web-install:
    @command -v bun >/dev/null || { echo "bun is required for web UI. Install bun and re-run."; exit 1; }
    (cd ../web && bun install)

web-build:
    @command -v bun >/dev/null || { echo "bun is required for web UI. Install bun and re-run."; exit 1; }
    (cd ../web && bun install && bun run build)

clean:
    cargo clean

doc:
    cargo doc --workspace --no-deps --open

release bump="patch":
    cargo xtask release {{bump}}

setup:
    cargo xtask hooks install
