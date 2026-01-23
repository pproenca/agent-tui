# Workspace Migration Specification

## Goal
Restructure agent-tui from a single crate (`cli/`) into a proper Cargo workspace with 6 crates under `crates/`, following rust-coder patterns.

## Success Criteria
- [ ] All crates build: `cargo build --workspace`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Lint clean: `cargo clippy --workspace -- -D warnings`
- [ ] Format clean: `cargo fmt --check`
- [ ] CLI runs: `cargo run -p agent-tui -- --help`
- [ ] Old `cli/` directory removed

## Crate Structure

```
crates/
├── agent-tui/           # Binary - CLI entry point
├── agent-tui-common/    # Leaf - shared utilities
├── agent-tui-terminal/  # Terminal emulation, PTY
├── agent-tui-core/      # VOM, element detection
├── agent-tui-ipc/       # Client/server protocol
└── agent-tui-daemon/    # Daemon server logic
```

## Dependency Graph

```
agent-tui (binary)
    ├── agent-tui-daemon
    │       ├── agent-tui-core
    │       │       ├── agent-tui-terminal
    │       │       │       └── agent-tui-common
    │       │       └── agent-tui-common
    │       └── agent-tui-ipc
    │               └── agent-tui-common
    └── agent-tui-ipc
```

## Import Style
std → external → workspace → crate (one per line)

## Out of Scope
- Feature additions
- Performance optimizations
- New tests (only migrate existing)
