# Release Readiness Checklist

Current target for #36: `1.0.2`.

```bash
VERSION=1.0.2
```

Use this checklist when publishing credentials or public registry state are not available to the implementing agent. Every command below is intended to be run from the repository root unless it explicitly changes directory.

## Required Credentials

- GitHub release publishing: repository Actions permissions for Releases and attestations.
- npm: trusted publishing for `agent-tui` and the supported platform packages.
- crates.io: `CARGO_REGISTRY_TOKEN`.
- Homebrew: `HOMEBREW_TAP_TOKEN` with write access to `pproenca/homebrew-tap`.
- Local tools for smoke checks: `gh`, `curl`, `jq`, `npm`, `cargo`, and `brew` for the Homebrew lane.

## Pre-Publish Gate

```bash
just ready
just release-channel-inventory
just release-channel-verify "$VERSION"
```

Expected result after publishing: `just ready` succeeds, the inventory lists all six active channels, and `just release-channel-verify "$VERSION"` reports `PASS` for `github-releases`, `npm`, `crates-io`, `homebrew`, `install-script`, and `source-install`.

If the version has not been published yet, `install-script` and `source-install` may be the only passing channels. Continue only when the publish credentials above are available.

## Publish

For the current `1.0.2` release line, verify that the latest public tag is `v1.0.1`, then trigger the workflow with a patch bump:

```bash
git fetch --tags origin
git tag --list 'v*' --sort=-v:refname | head -n 5
gh workflow run release.yml --ref master -f bump=patch -f dry_run=false
gh run watch --exit-status
```

Expected result: the release workflow resolves `VERSION=1.0.2`, creates GitHub Release `v1.0.2`, publishes npm packages, publishes crates.io crates, updates the Homebrew tap, and runs the channel verification jobs.

If the release must be driven by an explicit tag instead:

```bash
git fetch --tags origin
git tag -a "v$VERSION" -m "agent-tui $VERSION"
git push origin "v$VERSION"
gh run watch --exit-status
```

Expected result: the tag-triggered workflow publishes the same `v1.0.2` artifacts and all verification jobs pass.

## Channel Verification

```bash
cd cli
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel github-releases
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel install-script
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel npm
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel crates-io
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel source-install
cargo run -p xtask -- release-channels verify --target-version "$VERSION" --channel homebrew
cd ..
```

Expected result: every command prints `Release channel verification target: 1.0.2` and a `PASS <channel>` line for the requested channel.

## Install Smokes

### GitHub Releases

```bash
tmp="$(mktemp -d)"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) asset=agent-tui-darwin-arm64 ;;
  Darwin-x86_64) asset=agent-tui-darwin-x64 ;;
  Linux-aarch64|Linux-arm64) asset=agent-tui-linux-arm64 ;;
  Linux-x86_64) asset=agent-tui-linux-x64 ;;
  *) echo "unsupported smoke platform" >&2; exit 2 ;;
esac
curl -fsSL "https://github.com/pproenca/agent-tui/releases/download/v$VERSION/$asset" -o "$tmp/agent-tui"
chmod +x "$tmp/agent-tui"
"$tmp/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

### Install Script

```bash
INSTALL_DIR="$(mktemp -d)"
AGENT_TUI_SKIP_PM=1 \
  AGENT_TUI_VERSION="$VERSION" \
  AGENT_TUI_INSTALL_DIR="$INSTALL_DIR" \
  sh ./install.sh
"$INSTALL_DIR/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

### npm

```bash
NPM_CONFIG_PREFIX="$(mktemp -d)"
export NPM_CONFIG_PREFIX
npm install -g "agent-tui@$VERSION"
"$NPM_CONFIG_PREFIX/bin/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

### crates.io

```bash
CRATES_ROOT="$(mktemp -d)"
cargo install agent-tui --version "$VERSION" --root "$CRATES_ROOT" --locked
"$CRATES_ROOT/bin/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

### Source Install

```bash
SOURCE_ROOT="$(mktemp -d)"
cargo install --path cli/crates/agent-tui --root "$SOURCE_ROOT" --locked --force
"$SOURCE_ROOT/bin/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

To smoke the tagged source path after the GitHub tag is visible:

```bash
TAG_SOURCE_ROOT="$(mktemp -d)"
cargo install --git https://github.com/pproenca/agent-tui.git --tag "v$VERSION" --path cli/crates/agent-tui --root "$TAG_SOURCE_ROOT" --locked
"$TAG_SOURCE_ROOT/bin/agent-tui" --version
```

Expected result: `agent-tui $VERSION`.

### Homebrew

```bash
brew update
brew tap pproenca/tap https://github.com/pproenca/homebrew-tap
brew install pproenca/tap/agent-tui
brew upgrade pproenca/tap/agent-tui
agent-tui --version
```

Expected result: `agent-tui $VERSION`.

## Legacy Command Smoke

Run these against any installed candidate by setting `BIN` first:

```bash
BIN="${BIN:-agent-tui}"
SESSION_JSON="$("$BIN" --format json run sh -- -lc 'printf "ready\n"; cat')"
SESSION_ID="$(printf '%s' "$SESSION_JSON" | jq -r '.session_id')"
trap '"$BIN" --session "$SESSION_ID" kill --yes >/dev/null 2>&1 || true' EXIT

"$BIN" --session "$SESSION_ID" --format json input "legacy text" > /tmp/agent-tui-input.json 2> /tmp/agent-tui-input.err
jq -e '.success == true' /tmp/agent-tui-input.json
grep -F 'agent-tui input is deprecated' /tmp/agent-tui-input.err

"$BIN" --session "$SESSION_ID" --format json action @field fill "legacy fill" > /tmp/agent-tui-action-fill.json 2> /tmp/agent-tui-action-fill.err
jq -e '.success == true' /tmp/agent-tui-action-fill.json
grep -F 'agent-tui action is deprecated' /tmp/agent-tui-action-fill.err

"$BIN" --session "$SESSION_ID" --format json action @button click > /tmp/agent-tui-action-click.json 2> /tmp/agent-tui-action-click.err
jq -e '.success == true' /tmp/agent-tui-action-click.json
grep -F 'agent-tui action is deprecated' /tmp/agent-tui-action-click.err

"$BIN" --session "$SESSION_ID" --format json screenshot -e > /tmp/agent-tui-screenshot-e.json 2> /tmp/agent-tui-screenshot-e.err
jq -e '.session_id == "'"$SESSION_ID"'"' /tmp/agent-tui-screenshot-e.json
grep -F 'agent-tui screenshot -e is deprecated' /tmp/agent-tui-screenshot-e.err

"$BIN" --session "$SESSION_ID" --format json screenshot -a > /tmp/agent-tui-screenshot-a.json 2> /tmp/agent-tui-screenshot-a.err
jq -e '.session_id == "'"$SESSION_ID"'"' /tmp/agent-tui-screenshot-a.json
grep -F 'agent-tui screenshot -a is deprecated' /tmp/agent-tui-screenshot-a.err

"$BIN" --session "$SESSION_ID" --format json wait -e ready --assert --timeout 5000 > /tmp/agent-tui-wait-e.json 2> /tmp/agent-tui-wait-e.err
jq -e '.found == true' /tmp/agent-tui-wait-e.json
grep -F 'agent-tui wait -e is deprecated' /tmp/agent-tui-wait-e.err

"$BIN" --format json scroll-into-view @field > /tmp/agent-tui-scroll-into-view.json 2> /tmp/agent-tui-scroll-into-view.err
jq -e '.scrolled == false' /tmp/agent-tui-scroll-into-view.json
grep -F 'agent-tui scroll-into-view is deprecated' /tmp/agent-tui-scroll-into-view.err
```

Expected result: every `jq` command succeeds, every `grep` finds a deprecation notice on stderr, and every JSON stdout file parses successfully. No deprecation notice should appear in JSON stdout.

## Modern Command Smoke

This smoke covers current `agent-tui press Enter`, `agent-tui type "modern"`, and `agent-tui scroll down` behavior without deprecation noise.

```bash
BIN="${BIN:-agent-tui}"
SESSION_JSON="$("$BIN" --format json run sh -- -lc 'printf "ready\n"; cat')"
SESSION_ID="$(printf '%s' "$SESSION_JSON" | jq -r '.session_id')"
trap '"$BIN" --session "$SESSION_ID" kill --yes >/dev/null 2>&1 || true' EXIT

"$BIN" --session "$SESSION_ID" --format json screenshot > /tmp/agent-tui-modern-screenshot.json 2> /tmp/agent-tui-modern-screenshot.err
"$BIN" --session "$SESSION_ID" --format json type "modern" > /tmp/agent-tui-modern-type.json 2> /tmp/agent-tui-modern-type.err
"$BIN" --session "$SESSION_ID" --format json press Enter > /tmp/agent-tui-modern-press.json 2> /tmp/agent-tui-modern-press.err
"$BIN" --session "$SESSION_ID" --format json scroll down > /tmp/agent-tui-modern-scroll.json 2> /tmp/agent-tui-modern-scroll.err
"$BIN" --session "$SESSION_ID" --format json wait ready --assert --timeout 5000 > /tmp/agent-tui-modern-wait.json 2> /tmp/agent-tui-modern-wait.err

jq -e '.session_id == "'"$SESSION_ID"'"' /tmp/agent-tui-modern-screenshot.json
jq -e '.success == true' /tmp/agent-tui-modern-type.json
jq -e '.success == true' /tmp/agent-tui-modern-press.json
jq -e '.success == true' /tmp/agent-tui-modern-scroll.json
jq -e '.found == true' /tmp/agent-tui-modern-wait.json
! grep -R 'deprecated' /tmp/agent-tui-modern-*.err
```

Expected result: every JSON stdout file parses successfully, modern commands succeed, and stderr contains no deprecation notices.

## Release Notes

The release notes must call out:

- Active channels: GitHub Releases, install script, npm, crates.io, source install, and Homebrew.
- The observed `agent-tui $VERSION` from every install smoke.
- Compatibility window: `input`, `action`, `screenshot -e`, `screenshot -a`, `wait -e`, and `scroll-into-view` are compatibility-only.
- Deprecation behavior: notices are emitted on stderr, JSON stdout remains valid, and the compatibility commands are planned for next-major removal.
