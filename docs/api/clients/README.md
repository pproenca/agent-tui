# Reference Clients

These clients demonstrate how to consume the live preview WebSocket stream,
including binary output frames (`encoding=binary`).

## JS/TS (Bun)

```bash
bun run docs/api/clients/js/stream.ts
```

The script reads `AGENT_TUI_WS_STATE` or `~/.agent-tui/api.json` by default.

## Rust

```bash
cd docs/api/clients/rust
cargo run -- --session active
```

Both clients support passing a token, overriding the WS URL, and selecting a session.
