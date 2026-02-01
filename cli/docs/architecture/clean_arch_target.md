Target State: Single-Crate Clean Architecture (agent-tui)

Status: agreed target state for refactor
Scope: single Cargo package, single published binary

Goals
- Maintain a single published binary (no multi-crate publishing)
- Enforce Clean Architecture boundaries with module structure + visibility
- Keep Rust systems conventions (explicit modules, clear error types, minimal side effects)
- Make dependencies flow inward only

Non-Goals
- Splitting into multiple publishable crates
- Changing external CLI/RPC behavior or JSON schemas

High-Level Layout (single crate)
src/
  main.rs               // composition root only (CLI wiring + exit codes)
  lib.rs                // module declarations and limited re-exports

  domain/               // pure business rules, no IO/frameworks
    mod.rs
    errors.rs
    session.rs
    types.rs

  usecases/             // application logic + ports
    mod.rs
    ports/
      mod.rs
      session_repository.rs
      sleeper.rs
      terminal_io.rs
    session/
      mod.rs
      start.rs
      stop.rs
      list.rs
      cleanup.rs

  adapters/             // interface adapters (RPC/CLI presenters)
    mod.rs
    rpc/
      mod.rs
      parse.rs
      response.rs
    presenter/
      mod.rs
      text.rs
      json.rs

  infra/                // OS/socket/PTY/filesystem, implements ports
    mod.rs
    ipc/
      mod.rs
      unix_socket.rs
    terminal/
      mod.rs
      pty.rs
    process/
      mod.rs
      signals.rs

  app/                  // CLI application flow (thin)
    mod.rs
    dispatch.rs
    handlers/
      mod.rs
      sessions.rs
      recording.rs
      diagnostics.rs

Dependency Direction (must be enforced)
- domain: depends on nothing else (no serde_json, no std::fs, no thread/sleep)
- usecases: depends only on domain + usecases::ports
- adapters: depends on usecases + domain
- infra: depends on domain + usecases::ports
- app: depends on adapters + infra + usecases + domain
- main.rs: depends on app only

Boundary Rules
- No std::process::exit outside main.rs
- No JSON types outside adapters (serde_json::Value restricted to adapters)
- No IO or threading inside domain/usecases
- Usecases get time/sleep via ports (Sleeper trait)
- Repositories and external calls are ports implemented in infra

Visibility Rules
- Default to pub(crate) for internal types
- Only expose the minimal surface through lib.rs re-exports
- Avoid re-exporting infra types into app or domain

Error Handling
- domain errors in domain/errors.rs using thiserror
- usecases return domain/usecase errors, no printing/logging
- adapters map errors to RPC/CLI responses
- main.rs maps final errors to exit codes

Testing Strategy
- domain: pure unit tests
- usecases: unit tests with mock ports
- adapters: JSON/RPC contract tests
- infra: integration tests (socket/pty)

Enforcement Checklist
- Explicit module declarations in lib.rs and mod.rs
- Add boundary checks (ast-grep rules or CI script)
- Keep files under ~400 lines by splitting into submodules

Migration Notes
- Phase changes should not alter external CLI/RPC behavior
- Use small, reversible steps: move modules first, then tighten boundaries
