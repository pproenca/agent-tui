# Architecture

agent-tui follows Clean Architecture with Cargo crate boundaries as the enforcement mechanism.
Each layer is a separate crate — the compiler prevents dependency rule violations. The architecture
prioritizes testability, clear ownership, and forward-only dependency flow.

## Domain Map

| Crate | Layer | Path | Responsibility | Allowed deps |
|-------|-------|------|---------------|--------------|
| `agent-tui-common` | Foundation | `cli/crates/agent-tui-common/` | Shared primitives: `DaemonError`, error codes, colors, telemetry helpers, sync utils | None |
| `agent-tui-domain` | Domain | `cli/crates/agent-tui-domain/` | Core types: `SessionId`, `SessionInfo`, wait conditions, domain errors | common |
| `agent-tui-usecases` | Use Cases | `cli/crates/agent-tui-usecases/` | Business logic: Screenshot, Input, Wait orchestration; trait ports (`SessionRepository`, `SessionOps`) | domain, common |
| `agent-tui-adapters` | Adapters | `cli/crates/agent-tui-adapters/` | Interface translation: CLI command handlers, JSON-RPC presenters, request/response DTOs | usecases, domain, common |
| `agent-tui-infra` | Infrastructure | `cli/crates/agent-tui-infra/` | External integrations: PTY management, daemon runtime, session repository impl, terminal emulation | usecases, domain, common |
| `agent-tui-app` | Application | `cli/crates/agent-tui-app/` | Composition root: wires all layers, coordinates handlers and services, embeds web UI assets | adapters, infra, usecases, domain, common |
| `agent-tui` | Facade | `cli/crates/agent-tui/` | Entry points only: `main.rs`, `lib.rs`, `bin/*.rs` — delegates to app | app |
| `xtask` | Tooling | `cli/crates/xtask/` | Build orchestration: architecture validation, CI pipeline, release management | (external only) |

## Dependency Rules

```
                    ┌──────────────────┐
                    │   agent-tui      │  Facade (main.rs only)
                    │   (facade)       │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │   agent-tui-app  │  Composition root
                    │   (application)  │
                    └──┬──────────┬────┘
                       │          │
          ┌────────────▼──┐  ┌───▼──────────────┐
          │  adapters     │  │  infra            │  Outer ring
          │  (interfaces) │  │  (implementations)│
          └───────┬───────┘  └────────┬──────────┘
                  │                   │
                  └─────────┬─────────┘
                            │
                   ┌────────▼─────────┐
                   │  usecases        │  Business logic
                   │  (orchestration) │
                   └────────┬─────────┘
                            │
              ┌─────────────┼─────────────┐
              │                           │
     ┌────────▼─────────┐   ┌────────────▼────┐
     │  domain           │   │  common          │  Inner ring
     │  (types + rules)  │   │  (primitives)    │
     └──────────────────┘   └─────────────────┘
```

**The rule**: arrows point inward only. A crate may only depend on crates in the same or inner rings. This is enforced at compile time by Cargo and validated in CI by `xtask architecture check`.

## Cross-Cutting Concerns

| Concern | How it enters | Used by |
|---------|--------------|---------|
| Error handling | `DaemonError` + `ErrorCategory` in common; `thiserror` types in domain | All crates |
| Structured logging | `tracing` crate; telemetry helpers in common | All crates |
| Serialization | `serde` derives on types; JSON-RPC formatting in adapters | domain, adapters, infra |
| Async runtime | Tokio configured in app/facade; passed down as runtime context | infra, app |
| Terminal state | PTY handles created in infra; accessed via `SessionOps` trait in usecases | usecases, infra, app |

Cross-cutting concerns enter through **explicit interfaces**, not ambient imports. The `usecases` crate defines trait ports (e.g., `SessionRepository`); `infra` provides implementations; `app` wires them together.

## Where New Code Goes

| If you're adding... | Put it in... | Why |
|--------------------|-------------|-----|
| A new data type or model | `domain` | Types are the innermost layer; everything else depends on them |
| A new business operation | `usecases` | Orchestration logic with trait-based ports for external deps |
| A new CLI subcommand | `adapters` (handler) + `usecases` (logic) | Adapters translate CLI input; usecases do the work |
| A new JSON-RPC method | `adapters` (presenter) + `usecases` (logic) | Same separation: adapters format, usecases execute |
| A new PTY/terminal feature | `infra` (implementation) + `usecases` (trait port) | Infra owns external integrations; usecases define the interface |
| Composition or startup wiring | `app` | The composition root connects everything |
| A new binary entry point | `facade` (agent-tui crate) | Only `main.rs` and `bin/*.rs` |

## Key Interfaces

| Interface | Location | Purpose |
|-----------|----------|---------|
| `SessionRepository` (trait) | `agent-tui-usecases::ports` | Port for session storage — infra provides the implementation |
| `SessionOps` (trait) | `agent-tui-usecases::ports` | Port for session operations (screenshot, input, wait) |
| `DaemonError` | `agent-tui-common` | Unified error type with `ErrorCategory` for structured error handling |
| `SessionId` / `SessionInfo` | `agent-tui-domain::types` | Core domain identifiers and session state |
| `WaitConditionType` | `agent-tui-domain::types` | Raw wait condition discriminant (text, stable, text_gone) |
| `WaitCondition` | `agent-tui-usecases::wait_condition` | Parsed wait condition with associated text data |

## Web UI

The web UI (`web/`) is a Bun-based TypeScript app using xterm.js for terminal rendering. It connects to the daemon via WebSocket for live session preview. Built assets are embedded into `agent-tui-app/assets/web/` at compile time via `just web-sync`.

## Enforcement

Architecture compliance is validated at three levels:
1. **Compile time**: Cargo crate dependencies — wrong imports fail compilation
2. **CI time**: `xtask architecture check --verbose` — validates dependency matrix against allowed rules
3. **Lint time**: Clippy disallowed methods — bans `std::thread::sleep`, unbounded channels, `std::process::exit`

See `cli/docs/architecture/clean_arch_target.md` for the full target state specification.
