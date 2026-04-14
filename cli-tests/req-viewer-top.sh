#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-viewer-top.sh [--skip-build] [--keep]

Viewer scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. run a top snapshot through a shell
4. assert readable terminal output via wait and screenshot

Examples:
  cli-tests/req-viewer-top.sh
  cli-tests/req-viewer-top.sh --skip-build
  cli-tests/req-viewer-top.sh --keep
EOF
}

scenario_bootstrap "viewer-top" usage "$@"

require_command sh
require_command top

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

log_step "Viewer step: capture top output"
case "$(uname -s)" in
  Darwin)
    top_command="top -l 1 -stats pid,command,cpu,mem | sed -n '1,25p'; sleep 10"
    ;;
  Linux)
    top_command="top -b -n 1 | sed -n '1,25p'; sleep 10"
    ;;
  *)
    echo "Unsupported platform: $(uname -s)" >&2
    exit 1
    ;;
esac

top_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "$top_command")"
top_session_id="$(json_field "$top_payload" session_id)"
if [[ -z "$top_session_id" ]]; then
  echo "Failed to start top session" >&2
  printf '%s\n' "$top_payload" >&2
  exit 1
fi

"$BIN" --session "$top_session_id" wait --assert "PID" -t 5000 >/dev/null
"$BIN" --session "$top_session_id" wait --assert "COMMAND" -t 5000 >/dev/null
"$BIN" screenshot --strip-ansi >"$ARTIFACT_DIR/viewer-active.txt"
"$BIN" --session "$top_session_id" screenshot --strip-ansi >"$ARTIFACT_DIR/viewer-session.txt"
assert_file_contains "$ARTIFACT_DIR/viewer-active.txt" "PID"
assert_file_contains "$ARTIFACT_DIR/viewer-session.txt" "COMMAND"

"$BIN" --session "$top_session_id" kill --yes >/dev/null 2>&1 || true

printf '\nViewer top scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
