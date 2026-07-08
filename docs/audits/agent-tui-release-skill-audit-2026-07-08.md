# agent-tui Release and Skill Audit - 2026-07-08

Audit target: `/Users/pedroproenca/Documents/Projects/agent-tui`

Local checkout:

- Branch: `master`
- Commit: `de39f1f648f0`
- Local Rust workspace version: `1.0.2`
- Local npm package version: `1.0.2`
- Tested binary: `cli/target/debug/agent-tui`

## Summary

The latest CLI in this checkout is `1.0.2`, but it has not been released across the active public distribution channels. GitHub Releases and npm are still at `1.0.1`; crates.io is at `0.3.8` and appears to be a stale or legacy channel; Homebrew does not appear to be an active distribution channel.

The current repo-local `skills/agent-tui` skill is mostly aligned with the current CLI, but its references still contain several stale or misleading details. The older attached skill is not valid for the current CLI because it documents removed element-selector commands and flags such as `action`, `input`, `screenshot -e`, `screenshot -a`, `wait -e`, and `scroll-into-view`.

Cleanup note: repro snippets that set `AGENT_TUI_SOCKET`, `AGENT_TUI_SESSION_STORE`, and `AGENT_TUI_WS_STATE` should be run in an isolated shell. After reproducing, stop that temporary daemon with `"$BIN" --json sessions cleanup --all --yes` and `"$BIN" --json daemon stop --force --yes`, then remove the temporary directory.

## Release Channel Findings

### R1. Source version `1.0.2` is not published to GitHub Releases

Severity: high

Evidence:

- Local source has `1.0.2` in `cli/Cargo.toml` and `cli/package.json`.
- GitHub latest release is `v1.0.1`.
- `v1.0.2` release API returns `404`.
- `git ls-remote --tags https://github.com/pproenca/agent-tui.git 'refs/tags/v1.0.2*'` returned no tag.

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui
rg -n 'version = "1.0.2"|"version": "1.0.2"' cli/Cargo.toml cli/package.json
curl -fsSL https://api.github.com/repos/pproenca/agent-tui/releases/latest | jq -r '.tag_name, .html_url'
curl -fsSL https://api.github.com/repos/pproenca/agent-tui/releases/tags/v1.0.2
git ls-remote --tags https://github.com/pproenca/agent-tui.git 'refs/tags/v1.0.2*'
```

Observed:

- Local source: `1.0.2`
- Latest GitHub release: `v1.0.1`
- `v1.0.2` release lookup: `404`
- `v1.0.2` tag lookup: no output

Expected:

- If `1.0.2` is the latest intended CLI, GitHub should have a `v1.0.2` tag/release with binaries and `checksums-sha256.txt`.

### R2. Source version `1.0.2` is not published to npm packages

Severity: high

Affected packages:

- `agent-tui`
- `agent-tui-darwin-arm64`
- `agent-tui-darwin-x64`
- `agent-tui-linux-arm64`
- `agent-tui-linux-x64`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui
npm view agent-tui version dist-tags.latest optionalDependencies --json
for p in agent-tui agent-tui-darwin-arm64 agent-tui-darwin-x64 agent-tui-linux-arm64 agent-tui-linux-x64; do
  npm view "$p@1.0.2" version --json
done
```

Observed:

- `agent-tui` latest is `1.0.1`.
- Optional platform dependencies in the npm meta package point to `1.0.1`.
- `npm view *@1.0.2` returns `E404`.

Expected:

- If `1.0.2` is the latest intended CLI, npm latest should be `1.0.2` for the meta package and all platform packages.

### R3. crates.io is stale or no longer an active release channel

Severity: medium

Evidence:

- crates.io reports `agent-tui` max/newest/default version `0.3.8`.
- Current `cli/crates/agent-tui/Cargo.toml` has `publish = false`.
- The current release workflow appears focused on GitHub binary assets and npm packages.

Steps to reproduce:

```bash
curl -fsSL -H 'User-Agent: agent-tui-release-audit' \
  https://crates.io/api/v1/crates/agent-tui | jq -r '.crate.max_version, .crate.newest_version, .crate.default_version'
rg -n 'publish = false' /Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/Cargo.toml
```

Observed:

- crates.io latest: `0.3.8`
- local crate: `publish = false`

Expected:

- Either document crates.io as retired/legacy, or restore it to the release checklist and publish the current version there.

### R4. Homebrew is not an active distribution channel

Severity: low

Evidence:

- No Homebrew formula/tap files were found in the repo.
- `brew info agent-tui --json=v2` reports no available formula.
- Public formula/tap probes returned `404`.

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui
rg -n 'homebrew|brew' README.md install.sh cli docs skills .github
brew info agent-tui --json=v2
curl -fsSIL https://formulae.brew.sh/api/formula/agent-tui.json
curl -fsSIL https://raw.githubusercontent.com/pproenca/homebrew-tap/HEAD/Formula/agent-tui.rb
```

Observed:

- No active Homebrew channel found.

Expected:

- If Homebrew is intended, add and release a formula. Otherwise avoid listing it as a distribution channel.

### R5. Local PATH install is stale

Severity: low

Evidence:

- `agent-tui --version` on this machine reports `0.3.8`.
- The local checkout builds `1.0.2`.
- The repo-local npm shim at `cli/bin/agent-tui` cannot find its platform binary in the checkout.

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui
which agent-tui
agent-tui --version
cargo run --bin agent-tui -p agent-tui --manifest-path cli/Cargo.toml -- --version
./cli/bin/agent-tui --version
```

Observed:

- PATH binary: `/opt/homebrew/bin/agent-tui`, version `0.3.8`
- Local build: `agent-tui 1.0.2`
- Repo npm shim: `Binary not found for darwin-arm64: agent-tui-darwin-arm64`

Expected:

- Local PATH should not be used as evidence of current release status unless explicitly reinstalled.

## Current Repo Skill Findings

### S1. `kill` cleanup examples are not automation-safe without `--yes`

Severity: high

Locations:

- `skills/agent-tui/SKILL.md:21`
- `skills/agent-tui/SKILL.md:30`
- `skills/agent-tui/references/test-plan.md:16`
- `skills/agent-tui/references/test-plan.md:25`
- `skills/agent-tui/references/flows.md:18`
- `skills/agent-tui/references/flows.md:27`
- `skills/agent-tui/references/flows.md:36`
- `skills/agent-tui/references/demo.md:32`
- `skills/agent-tui/references/use-cases.md:6`
- `skills/agent-tui/references/session-lifecycle.md:9`
- `skills/agent-tui/references/session-lifecycle.md:24`

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
export AGENT_TUI_NO_INPUT=1
run_payload=$("$BIN" --json run sh -- -lc 'sleep 30')
session_id=$(printf '%s' "$run_payload" | jq -r .session_id)
"$BIN" --session "$session_id" --json kill
```

Observed:

```json
{
  "category": "invalid_input",
  "code": 64,
  "message": "Confirmation required. Re-run with --yes to perform the action or --dry-run to preview it.",
  "retryable": false,
  "suggestion": "Add --yes to proceed or --dry-run to preview the change."
}
```

Expected:

- Skill automation examples should use `kill --yes` or explicitly explain that plain `kill` is interactive only.

### S2. `live stop` does not stop daemon-backed live preview

Severity: medium

Locations:

- `skills/agent-tui/SKILL.md:45`
- `skills/agent-tui/references/flows.md:43`
- `skills/agent-tui/references/use-cases.md:16`
- `skills/agent-tui/references/clarifications.md:16`

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
export AGENT_TUI_UI_STATE="$TMPDIR/ui.json"
export AGENT_TUI_NO_INPUT=1
"$BIN" --json daemon start
"$BIN" --json live start
"$BIN" --json live stop
"$BIN" --json live status
```

Observed:

```json
{
  "stopped": false,
  "reason": "live preview is served by the daemon; run `agent-tui daemon stop --yes` to stop it."
}
```

`live status` still reports `running: true`.

Expected:

- Skill text should explain that `live stop` does not stop daemon-backed preview and that users need `daemon stop --yes` when they want to stop the daemon itself.

### S3. Screenshot `--region` is documented as usable, but currently rejected

Severity: medium

Location:

- `skills/agent-tui/references/command-atlas.md:23`

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
run_payload=$("$BIN" --json run sh -- -lc 'printf READY; sleep 5')
session_id=$(printf '%s' "$run_payload" | jq -r .session_id)
"$BIN" --session "$session_id" screenshot --region header
```

Observed:

```text
agent-tui: Error: RPC error (-32019): Invalid input for region: Named snapshot regions are not supported
Suggestion: Adjust the invalid input and retry the command.
```

Expected:

- The command atlas should say `--region` is reserved/currently rejected, or omit it from recommended skill commands.

### S4. `wait --assert` timeout exit code is documented incorrectly

Severity: medium

Location:

- `skills/agent-tui/references/command-atlas.md:43`

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
run_payload=$("$BIN" --json run sh -- -lc 'printf READY; sleep 5')
session_id=$(printf '%s' "$run_payload" | jq -r .session_id)
set +e
"$BIN" --session "$session_id" --json wait NEVER --assert --timeout 50
echo "exit=$?"
```

Observed:

- Exit code: `75`
- JSON error category: `timeout`

Expected:

- `command-atlas.md` should document timeout exit code `75`, matching current help.

### S5. Detach sequence docs disagree with the current CLI

Severity: medium

Locations:

- `skills/agent-tui/references/session-lifecycle.md:20`
- `skills/agent-tui/references/flows.md:50`
- Related non-skill drift: `README.md:148`

Steps to reproduce:

```bash
/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui sessions attach --help
rg -n 'Ctrl-P Ctrl-Q|Ctrl-P Ctrl-B' /Users/pedroproenca/Documents/Projects/agent-tui/skills/agent-tui README.md docs/cli/agent-tui.md
```

Observed:

- Skill references say `Ctrl-P Ctrl-Q`.
- CLI help and generated docs say default detach keys are `Ctrl-P Ctrl-B`.

Expected:

- Skill references should match current CLI help.

### S6. Screenshot JSON contract overstates the default `cursor` field

Severity: low

Location:

- `skills/agent-tui/references/output-contract.md:11`

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
run_payload=$("$BIN" --json run sh -- -lc 'printf READY; sleep 5')
session_id=$(printf '%s' "$run_payload" | jq -r .session_id)
"$BIN" --session "$session_id" --json screenshot --strip-ansi
"$BIN" --session "$session_id" --json screenshot --strip-ansi --include-cursor
```

Observed:

- Default screenshot JSON includes `screenshot` and `session_id`.
- `cursor` is only present when `--include-cursor` is passed.

Expected:

- `output-contract.md` should mark `cursor` as conditional on `--include-cursor`.

### S7. `sessions switch` JSON message says "attached"

Severity: low

Steps to reproduce:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
a=$("$BIN" --json run sh -- -lc 'sleep 20' | jq -r .session_id)
b=$("$BIN" --json run sh -- -lc 'sleep 20' | jq -r .session_id)
"$BIN" --json sessions switch "$a"
```

Observed:

```json
{
  "message": "Now attached to session <id>",
  "session_id": "<id>",
  "success": true
}
```

Expected:

- Message should say "switched to session" or similar. Switching active session is not the same as attaching.

## Older Attached Skill Findings

Attachment audited:

`/Users/pedroproenca/.codex/attachments/566a3486-d7e1-41bc-a5df-13ee0a4cf5e4/pasted-text.txt`

### A1. Element-ref screenshot mode is stale

Severity: high

Locations:

- `pasted-text.txt:102`
- `pasted-text.txt:157`
- `pasted-text.txt:183`
- `pasted-text.txt:188`
- `pasted-text.txt:191`
- `pasted-text.txt:200`
- `pasted-text.txt:206`
- `pasted-text.txt:310`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- screenshot -e --json
```

Observed:

```text
error: unexpected argument '-e' found
```

Expected:

- Replace with `screenshot --json`; remove element-ref claims.

### A2. Accessibility screenshot mode is stale

Severity: high

Locations:

- `pasted-text.txt:108`
- `pasted-text.txt:311`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- screenshot -a --interactive-only
```

Observed:

```text
error: unexpected argument '-a' found
```

Expected:

- Remove `screenshot -a` and `--interactive-only`; current screenshots are terminal text/ANSI captures, with optional cursor inclusion.

### A3. `action` command and selector model are removed

Severity: high

Locations:

- `pasted-text.txt:136`
- `pasted-text.txt:139`
- `pasted-text.txt:163`
- `pasted-text.txt:184`
- `pasted-text.txt:185`
- `pasted-text.txt:189`
- `pasted-text.txt:192`
- `pasted-text.txt:201`
- `pasted-text.txt:207`
- `pasted-text.txt:214`
- `pasted-text.txt:218`
- `pasted-text.txt:282`
- `pasted-text.txt:283`
- `pasted-text.txt:284`
- `pasted-text.txt:297`
- `pasted-text.txt:314`
- `pasted-text.txt:315`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- action --help
```

Observed:

```text
error: unrecognized subcommand 'action'
```

Expected:

- Replace click/fill/select/toggle examples with current `press`, `type`, and `scroll` workflows.

### A4. `input` command is stale

Severity: high

Locations:

- `pasted-text.txt:139`
- `pasted-text.txt:318`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- input "text"
```

Observed:

```text
error: unrecognized subcommand 'input'
```

Expected:

- Use `type "text"`.

### A5. `wait -e @ref` is stale

Severity: high

Locations:

- `pasted-text.txt:120`
- `pasted-text.txt:121`
- `pasted-text.txt:322`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- wait -e @e1 --gone
```

Observed:

```text
error: unexpected argument '@e1' found
```

Expected:

- Current `wait` supports text presence, text disappearance with `--gone`, and `--stable`; it does not support element refs.

### A6. `scroll-into-view` is stale

Severity: high

Locations:

- `pasted-text.txt:145`
- `pasted-text.txt:269`

Steps to reproduce:

```bash
cd /Users/pedroproenca/Documents/Projects/agent-tui/cli
cargo run -q --bin agent-tui -- scroll-into-view @e1
```

Observed:

```text
error: unrecognized subcommand 'scroll-into-view'
```

Expected:

- Use directional `scroll <up|down|left|right> [AMOUNT]`, or keyboard navigation via `press`.

## Confirmed Passing Workflow

This workflow passed against the local `1.0.2` debug binary with isolated daemon/session state:

```bash
set -euo pipefail
BIN=/Users/pedroproenca/Documents/Projects/agent-tui/cli/target/debug/agent-tui
TMPDIR=$(mktemp -d)
export AGENT_TUI_SOCKET="$TMPDIR/agent-tui.sock"
export AGENT_TUI_SESSION_STORE="$TMPDIR/sessions.jsonl"
export AGENT_TUI_WS_STATE="$TMPDIR/api.json"
run_payload=$("$BIN" --json run --cols 80 --rows 12 sh -- -lc 'printf "READY\n"; read line; printf "ECHO:%s\n" "$line"; sleep 3')
session_id=$(printf '%s' "$run_payload" | jq -r .session_id)
"$BIN" --session "$session_id" --json wait READY --assert --timeout 5000
"$BIN" --session "$session_id" --json screenshot --strip-ansi
"$BIN" --session "$session_id" --json type hello
"$BIN" --session "$session_id" --json press Enter
"$BIN" --session "$session_id" --json wait ECHO:hello --assert --timeout 5000
"$BIN" --session "$session_id" --json screenshot --strip-ansi
"$BIN" --session "$session_id" --json kill --yes
```

Observed:

- `run` returned `pid` and `session_id`.
- `wait READY --assert` returned `{ "found": true }`.
- `type hello` and `press Enter` returned `{ "success": true }`.
- `wait ECHO:hello --assert` returned `{ "found": true }`.
- `kill --yes` returned `{ "success": true }`.

## Subagent-Only Observations Not Reproduced Locally

One adversarial subagent reported that explicit `daemon start` returned success and then the daemon immediately became unreachable on macOS with `Connection refused`. I reran the same temporary-environment pattern locally and could not reproduce it: `daemon status` returned `running: true` for the reported PID.

The same subagent also reported that `run --json` emitted `Note: Starting daemon in background...` before the JSON payload. In local reproduction, the note was emitted to stderr and stdout remained valid JSON, so direct stdout JSON parsing with `jq` works.

Both are worth watching in CI or repeated stress runs, but they are not confirmed findings from this audit.
