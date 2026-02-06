# Repository Guidelines

## Project Structure & Module Organization
- `cli/`: Rust workspace for the CLI/daemon. Facade crate at `cli/crates/agent-tui/`, internal layer crates under `cli/crates/agent-tui-{common,domain,usecases,adapters,infra,app}/`, and Rust task runner at `cli/crates/xtask/`.
- `web/`: Bun-based live preview UI. Source in `web/src/`, built assets in `web/public/`.
- `docs/`: repository-level documentation.
- `scripts/` and `install.sh`: release/installation helpers.
- `skills/`: agent skill definitions and references.

## Build, Test, and Development Commands
Commands are managed via `just` from the repo root (it runs in `cli/`).
- `just dev`: run the daemon in dev mode.
- `just health`: run the CLI health check.
- `just build` / `just build-release`: build Rust workspace (installs web deps first).
- `just web-build`: build the web UI with Bun.
- `just test`: run Rust tests (smoke/contract focus).
- `just ready`: full CI-style checks (fmt, clippy, architecture, tests, version).

## Coding Style & Naming Conventions
- Rust: format with `rustfmt` and lint with `clippy` (`just format`, `just lint`). Follow standard Rust naming (snake_case for functions/vars, PascalCase for types).
- TypeScript/Bun: keep code consistent with existing style; avoid hand-editing generated files in `web/public/`.
- Keep changes minimal and cohesive; prefer small, focused modules.

## Testing Guidelines
- Unit/integration tests live under `cli/crates/agent-tui/tests` and module-level `#[cfg(test)]` blocks.
- Run `just test` for local verification; use `just ready` before PRs. Optional dependency checks run if `cargo-machete` is installed.

## Commit & Pull Request Guidelines
- Commit messages generally follow a conventional pattern: `type: summary` (e.g., `feat: ...`, `fix: ...`, `chore: ...`, `refactor: ...`, `ci: ...`, `release: ...`). Keep summaries short and imperative.
- PRs should include: a clear description, tests run, and linked issues. Add screenshots or clips for UI changes in `web/`.

## Configuration Notes
- The web server reads daemon state from `AGENT_TUI_API_STATE` (defaults to `~/.agent-tui/api.json`). Document any new env vars you introduce.
