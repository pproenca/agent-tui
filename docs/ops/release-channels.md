# Release Channels

agent-tui treats these distribution channels as active for release readiness:

- GitHub Releases: binary assets plus `checksums-sha256.txt`.
- npm: the `agent-tui` meta package and all supported platform packages.
- crates.io: the Rust package for `cargo install`.
- Homebrew: the project formula or tap.
- Install script: latest and `AGENT_TUI_VERSION` pinned installs from GitHub assets.
- Source install: the repository checkout build path.

Run the read-only verification gate before and after publishing:

```bash
just release-channel-inventory
just release-channel-verify 1.0.2
```

For CI or local tests that must not depend on live public registry state, pass a fixture file. Relative fixture paths are resolved from `cli/`; absolute paths are accepted.

```bash
just release-channel-verify 1.0.2 path/to/release-channel-fixture.json
```

The gate reports one `PASS` or `FAIL` line per channel and exits non-zero if any active channel is missing, stale, missing a required asset, missing platform package state, or missing formula state.
