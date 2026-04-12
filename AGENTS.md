# agent-tui — Agent Guide

CLI tool that enables AI agents to programmatically drive terminal applications.
Manages PTY sessions via a background daemon with JSON-RPC WebSocket API and embedded web UI.

## Non-Negotiable Rules

1. **Dependencies flow forward only.** `common → domain → usecases → adapters/infra → app → facade`. No reverse imports. Validated by `just boundaries` and CI.
2. **No panics in production code.** `unwrap_used` and `expect_used` are denied workspace-wide. Use `?`, `anyhow::Context`, or `thiserror` types. Tests may unwrap.
3. **No blocking in async context.** `std::thread::sleep`, unbounded channels, and `std::process::exit` (except `main.rs`) are banned via Clippy disallowed methods.
4. **All errors use structured types.** Domain errors use `thiserror`; infra/app errors use `anyhow` with `.context()`. Never swallow errors silently.
5. **Facade crate is a shell.** `cli/crates/agent-tui/` contains only `main.rs`, `lib.rs`, and `bin/*.rs`. All logic lives in layer crates.

## Repository Map

| Path | What it contains |
|------|-----------------|
| `ARCHITECTURE.md` | Domain boundaries, dependency graph, layer descriptions, where new code goes |
| `docs/design-docs/` | Design history, core beliefs, architectural decisions |
| `docs/api/` | OpenAPI + AsyncAPI specs, example clients (Rust, JS) |
| `docs/ops/` | Process model, deployment patterns |
| `docs/cli/` | Auto-generated CLI reference (do not hand-edit) |
| `cli/docs/architecture/` | Clean Architecture target state and dependency matrix |
| `cli/docs/plans/` | Historical refactor/restructure plans |
| `.harness/` | Machine-readable harness config (domains, principles, enforcement, quality) |
| `skills/` | Agent skill definitions and references |

## Tech Stack

- **Rust 1.85+** (2024 edition) — Tokio async runtime, Axum WebSocket server, Clap CLI
- **TypeScript/Bun** — Web UI with xterm.js terminal emulator
- **Build**: Cargo workspace (8 crates) + `just` task runner
- **Key deps**: `portable-pty`, `tracing`, `serde`, `crossterm`

## Platform Boundary

- **Supported targets**: Linux, macOS, and other Unix-like systems only
- **Required runtime primitives**: PTYs, Unix domain sockets, POSIX signals, and standard Unix shell environment variables (`HOME`, `SHELL`, optionally `XDG_RUNTIME_DIR`)
- **Shell integrations**: `bash`, `zsh`, `fish`, and `elvish`
- **Out of scope**: Windows, PowerShell, `COMSPEC`/`USERPROFILE` fallbacks, and Win32 process or path semantics
- **Path conventions**: Unix socket paths should follow `XDG_RUNTIME_DIR` when available; persistent metadata should stay under home-scoped dot directories unless explicitly overridden

## Verification

Run from repo root before marking work complete:

```bash
just ready          # Full CI: fmt, clippy, architecture, tests, version check
just test           # Fast: smoke, contract, unit tests
just boundaries     # Architecture boundary check + dependency graph snapshot
just lint           # Clippy with -D warnings
just format-check   # rustfmt verification
```

For slow E2E tests (daemon lifecycle): `just test-core-e2e`

## Code Organization

```
cli/crates/
├── agent-tui/          # Facade: main.rs, bin/ entry points only
├── agent-tui-common/   # Shared: DaemonError, error codes, colors, telemetry
├── agent-tui-domain/   # Domain: SessionId, SessionInfo, types, domain errors
├── agent-tui-usecases/ # Use cases: Screenshot, Input, Wait; trait ports
├── agent-tui-adapters/ # Adapters: CLI handlers, JSON-RPC presenters, DTOs
├── agent-tui-infra/    # Infra: PTY management, daemon, session repo, terminal
├── agent-tui-app/      # App: composition root, handler coordination, web assets
└── xtask/              # Build: architecture checks, CI orchestration, releases
web/                    # Bun-based live preview UI (embedded into app crate)
```

New code goes in the **innermost layer that owns the concept**:
- New type/model → `domain`
- New business rule → `usecases`
- New CLI handler or RPC presenter → `adapters`
- New PTY/daemon/terminal concern → `infra`
- New composition or wiring → `app`

## Review Agents

Three specialized agents run during code review (see `.claude/agents/`):
- **clean-architecture-reviewer** — Dependency rule and layer compliance
- **rust-principal-reviewer** — Design quality, correctness, complexity
- **clean-code-reviewer** — Readability, naming, maintainability
