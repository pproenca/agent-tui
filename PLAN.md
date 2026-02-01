# Plan: Remove Element Targeted Actions + Reference System

## Goal
Remove the element reference system (e.g., `@e1`, `ref=e1`, `refs` maps) and all element-targeted actions across the CLI/daemon stack. The system should focus on non-element features only.

## Scope Summary
- Delete element references in snapshots, domain types, conversions, and element models.
- Remove all element-targeted commands/usecases/routes (click/fill/select/toggle/focus/scroll-into-view, element waits).
- Remove RPC/IPC payloads containing refs and ref-based errors/suggestions.
- Update CLI UX, presenters, and docs to reflect removal.
- Remove tests/mocks/fixtures tied to refs and element actions.

## Detailed Steps
1. **Core domain cleanup**
   - Remove reference types and snapshot ref generation.
   - Strip element-ref fields and lookup helpers.
2. **Usecase + port cleanup**
   - Delete element actions usecase and element-based wait conditions.
   - Update session repository port and errors to remove element-specific APIs.
3. **Adapter + RPC/IPC cleanup**
   - Remove element endpoints, request parsing, and response fields in daemon/RPC/IPC.
   - Update presenter and snapshot adapters accordingly.
4. **CLI cleanup**
   - Remove element commands/aliases (action, scroll-into-view, find if ref-based, `@e1` shorthand).
   - Remove selector parsing for element refs.
5. **Infra cleanup**
   - Remove element lookup/action implementations in terminal/session repository.
6. **Tests + docs**
   - Delete/refactor tests and docs referencing element refs or element actions.

## File Inventory (all expected edits/removals)

### Core domain
- `cli/crates/agent-tui/src/domain/types.rs`
- `cli/crates/agent-tui/src/domain/conversions.rs`
- `cli/crates/agent-tui/src/domain/core/element.rs`
- `cli/crates/agent-tui/src/domain/core/vom/snapshot.rs`
- `cli/crates/agent-tui/src/domain/core/test_fixtures.rs`

### Usecases + ports
- `cli/crates/agent-tui/src/usecases/elements.rs`
- `cli/crates/agent-tui/src/usecases/mod.rs`
- `cli/crates/agent-tui/src/usecases/session.rs`
- `cli/crates/agent-tui/src/usecases/wait_condition.rs`
- `cli/crates/agent-tui/src/usecases/wait.rs`
- `cli/crates/agent-tui/src/usecases/ports/session_repository.rs`
- `cli/crates/agent-tui/src/usecases/ports/errors.rs`
- `cli/crates/agent-tui/src/usecases/ports/test_support/element_builder.rs`
- `cli/crates/agent-tui/src/usecases/ports/test_support/mock_session.rs`
- `cli/crates/agent-tui/src/usecases/ports/test_support/mod.rs`

### Adapters / RPC / IPC / Presenter
- `cli/crates/agent-tui/src/adapters/daemon/handlers/elements.rs`
- `cli/crates/agent-tui/src/adapters/daemon/router.rs`
- `cli/crates/agent-tui/src/adapters/daemon/usecase_container.rs`
- `cli/crates/agent-tui/src/adapters/daemon/error.rs`
- `cli/crates/agent-tui/src/adapters/rpc.rs`
- `cli/crates/agent-tui/src/adapters/ipc/params.rs`
- `cli/crates/agent-tui/src/adapters/ipc/snapshot_dto.rs`
- `cli/crates/agent-tui/src/adapters/ipc/types.rs`
- `cli/crates/agent-tui/src/adapters/ipc/mod.rs`
- `cli/crates/agent-tui/src/adapters/snapshot_adapters.rs`
- `cli/crates/agent-tui/src/adapters/presenter.rs`
- `cli/crates/agent-tui/src/adapters/mod.rs`
- `cli/crates/agent-tui/src/adapters/daemon/handlers/mod.rs`

### CLI app layer
- `cli/crates/agent-tui/src/app/commands.rs`
- `cli/crates/agent-tui/src/app/mod.rs`
- `cli/crates/agent-tui/src/app/handlers.rs`
- `cli/crates/agent-tui/src/common/color.rs`

### Infra
- `cli/crates/agent-tui/src/infra/daemon/terminal_state.rs`
- `cli/crates/agent-tui/src/infra/daemon/session.rs`
- `cli/crates/agent-tui/src/infra/daemon/repository.rs`
- `cli/crates/agent-tui/src/infra/daemon/mod.rs`

### Tests
- `cli/crates/agent-tui/src/domain/core/vom/snapshot.rs`
- `cli/crates/agent-tui/src/domain/conversions.rs`
- `cli/crates/agent-tui/src/domain/core/test_fixtures.rs`
- `cli/crates/agent-tui/src/usecases/elements.rs`
- `cli/crates/agent-tui/src/usecases/wait_condition.rs`
- `cli/crates/agent-tui/src/usecases/ports/test_support/element_builder.rs`
- `cli/crates/agent-tui/src/usecases/ports/test_support/mock_session.rs`
- `cli/crates/agent-tui/src/adapters/ipc/params.rs`
- `cli/crates/agent-tui/src/adapters/ipc/snapshot_dto.rs`
- `cli/crates/agent-tui/src/adapters/ipc/types.rs`
- `cli/crates/agent-tui/src/adapters/rpc.rs`
- `cli/crates/agent-tui/src/adapters/presenter.rs`
- `cli/crates/agent-tui/src/adapters/daemon/error.rs`
- `cli/crates/agent-tui/src/app/commands.rs`
- `cli/crates/agent-tui/src/app/mod.rs`
- `cli/crates/agent-tui/src/app/handlers.rs`
- `cli/crates/agent-tui/tests/cli_smoke.rs`
- `cli/crates/agent-tui/tests/common/mock_daemon.rs`

### Docs + skills
- `README.md`
- `skills/agent-tui/SKILL.md`
- `skills/agent-tui/references/assertions.md`
- `skills/agent-tui/references/command-atlas.md`
- `skills/agent-tui/references/decision-tree.md`
- `skills/agent-tui/references/demo.md`
- `skills/agent-tui/references/flows.md`
- `skills/agent-tui/references/output-contract.md`
- `skills/agent-tui/references/recovery.md`
- `skills/agent-tui/references/test-plan.md`
- `skills/agent-tui/references/use-cases.md`

## Out of Scope
- `web/public/app.js` and `cli/crates/agent-tui/assets/web/app.js` (generated assets)
- Git tag refs in `cli/scripts/xtask.ts`
