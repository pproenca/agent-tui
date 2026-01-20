# Implement Spec: {{SPEC_NAME}}

## Your Task

You are implementing TUI commands for agent-tui to achieve browser parity. Your goal is to implement all commands specified in the spec file below.

## Implementation Checklist

For each command in the spec:

1. **CLI Command** (`/cli/src/commands.rs`)
   - Add subcommand with clap derive macro
   - Match exact argument names and types from spec
   - Include help text

2. **Protocol Types** (`/cli/src/protocol.rs`)
   - Add `METHOD_*` constant for JSON-RPC method name
   - Add request struct (params)
   - Add response struct if needed

3. **Daemon Handler** (`/cli/src/daemon/server.rs`)
   - Add match arm in request handler
   - Implement the actual functionality
   - Return appropriate response

4. **Quality Gates**
   - Run `cargo clippy --all-targets --all-features -- -D warnings` - must pass
   - Run `cargo test` - must pass
   - No unused imports or dead code

## Key Files to Modify

```
cli/src/commands.rs    - CLI definitions (clap)
cli/src/protocol.rs    - JSON-RPC types and methods
cli/src/daemon/server.rs - Request handlers
cli/src/daemon/session.rs - Session implementation (if needed)
cli/src/daemon/terminal.rs - Terminal emulation (if needed)
cli/src/daemon/detection/ - Element detection (if needed)
```

## Code Style Requirements

- Follow existing patterns in the codebase
- Use `#[serde(rename_all = "camelCase")]` for JSON serialization
- Use `clap::ValueEnum` for enum CLI arguments
- Keep handlers focused - delegate to session methods where appropriate
- No unnecessary comments - code should be self-explanatory

## Verification Steps

After implementing:

```bash
cd cli
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Both must pass before you are done.

## When Complete

When you have:
- Implemented all commands from the spec
- Verified with clippy (no warnings)
- Verified with tests (all pass)

Output this exact tag:

<promise>IMPLEMENTED</promise>

Do NOT output this tag until all verification steps pass.
