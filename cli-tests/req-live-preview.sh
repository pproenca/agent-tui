#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-live-preview.sh [--skip-build] [--keep]

Live preview scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. inspect daemon and live-preview status
4. fetch the embedded /ui route and verify live stop semantics

Examples:
  cli-tests/req-live-preview.sh
  cli-tests/req-live-preview.sh --skip-build
  cli-tests/req-live-preview.sh --keep
EOF
}

scenario_bootstrap "live-preview" usage "$@"

require_command curl

log_step "Starting isolated daemon"
"$BIN" daemon run >"$ARTIFACT_DIR/daemon.stdout.log" 2>"$ARTIFACT_DIR/daemon.stderr.log" &
DAEMON_PID=$!
for attempt in $(seq 1 100); do
  if "$BIN" --json sessions >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
  if [[ "$attempt" -eq 100 ]]; then
    echo "Timed out waiting for daemon readiness" >&2
    exit 1
  fi
done

log_step "Live step: inspect daemon and live-preview status"
daemon_status="$("$BIN" daemon status --json)"
live_status="$("$BIN" live status --json)"
live_start="$("$BIN" live start --json)"

printf '%s\n' "$daemon_status" >"$ARTIFACT_DIR/daemon-status.json"
printf '%s\n' "$live_status" >"$ARTIFACT_DIR/live-status.json"
printf '%s\n' "$live_start" >"$ARTIFACT_DIR/live-start.json"

assert_contains "$daemon_status" "\"running\": true"
assert_contains "$daemon_status" "\"version_mismatch\": false"
assert_contains "$live_status" "\"running\": true"
assert_contains "$live_start" "\"running\": true"

ui_url="$(json_field "$live_status" ui_url)"
ws_url="$(json_field "$live_status" ws_url)"
if [[ -z "$ui_url" || -z "$ws_url" ]]; then
  echo "Live status did not expose ui_url/ws_url" >&2
  printf '%s\n' "$live_status" >&2
  exit 1
fi

log_step "Live step: fetch the embedded UI"
curl -fsS "$ui_url" >"$ARTIFACT_DIR/ui.html"
assert_file_contains "$ARTIFACT_DIR/ui.html" "<!doctype html>"
assert_file_contains "$ARTIFACT_DIR/ui.html" "<html"

log_step "Live step: verify stop semantics for daemon-backed preview"
live_stop="$("$BIN" live stop --json)"
printf '%s\n' "$live_stop" >"$ARTIFACT_DIR/live-stop.json"
assert_contains "$live_stop" "\"stopped\": false"
assert_contains "$live_stop" 'run `agent-tui daemon stop --yes` to stop it.'

live_status_after_stop="$("$BIN" live status --json)"
printf '%s\n' "$live_status_after_stop" >"$ARTIFACT_DIR/live-status-after-stop.json"
assert_contains "$live_status_after_stop" "\"running\": true"
assert_contains "$live_status_after_stop" "\"ui_url\": \"$ui_url\""

printf '\nLive preview scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
