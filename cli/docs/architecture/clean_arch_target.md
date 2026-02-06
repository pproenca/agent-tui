Target State: Internal Multi-Crate Clean Architecture (agent-tui workspace)

Status: agreed target state for enforcement
Scope: single published binary (`agent-tui`), multi-crate internal workspace

Goals
- Keep the external CLI/API behavior unchanged.
- Enforce dependency direction with Cargo crate boundaries.
- Enforce architecture policy with Rust-native tooling (`xtask` + `cargo metadata`).
- Keep runtime/event-driven guardrails in Clippy policy (no unbounded channels, no blocking `std::thread::sleep`).

Non-Goals
- Publishing layer crates independently.
- Reintroducing pattern-based architecture enforcement.

Workspace Layout
- `crates/agent-tui-common`: shared primitives/utilities.
- `crates/agent-tui-domain`: domain types and business rules.
- `crates/agent-tui-usecases`: use-case orchestration and ports.
- `crates/agent-tui-adapters`: interface adapters (RPC/presenters/router).
- `crates/agent-tui-infra`: infrastructure integrations (IPC/terminal/process/daemon infra).
- `crates/agent-tui-app`: application composition and command handling.
- `crates/agent-tui`: facade crate + binaries only (`main.rs`, `bin/*`).
- `crates/xtask`: Rust-native repo orchestration and architecture checks.

Allowed Internal Dependency Matrix
- `agent-tui-common` -> none
- `agent-tui-domain` -> `agent-tui-common`
- `agent-tui-usecases` -> `agent-tui-domain`, `agent-tui-common`
- `agent-tui-adapters` -> `agent-tui-usecases`, `agent-tui-domain`, `agent-tui-common`
- `agent-tui-infra` -> `agent-tui-usecases`, `agent-tui-domain`, `agent-tui-common`
- `agent-tui-app` -> `agent-tui-adapters`, `agent-tui-infra`, `agent-tui-usecases`, `agent-tui-domain`, `agent-tui-common`
- `agent-tui` -> `agent-tui-app` (plus external CLI/doc-generation crates)

Boundary/Policy Rules
- No production code in facade crate beyond `main.rs`, `lib.rs`, and `bin/*.rs`.
- No permissive "root bucket" fallback for architecture validation.
- No `std::process::exit` outside `main.rs`.
- Disallow in Clippy:
  - `std::thread::sleep`
  - `tokio::sync::mpsc::unbounded_channel`
  - `crossbeam_channel::unbounded`
  - `std::sync::mpsc::channel`

Enforcement Tooling
- `cargo run -p xtask -- architecture check --verbose`
  - validates workspace crate graph against the allowed matrix.
  - fails on unknown internal crates.
  - fails on unknown top-level production directories in `src/`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - enforces disallowed methods and lint policy.

CI Baseline
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo run -p xtask -- architecture check --verbose`
4. `cargo test --workspace`
