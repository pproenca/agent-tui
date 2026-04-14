#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-sessions-switch.sh [--skip-build] [--keep]

Sessions scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. spawn two long-lived shell sessions
4. verify sessions list/show/switch behavior and active-session defaults

Examples:
  cli-tests/req-sessions-switch.sh
  cli-tests/req-sessions-switch.sh --skip-build
  cli-tests/req-sessions-switch.sh --keep
EOF
}

scenario_bootstrap "sessions-switch" usage "$@"

require_command sh

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

log_step "Sessions step: spawn two shell sessions"
alpha_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "printf 'alpha session\\n'; sleep 20")"
alpha_id="$(json_field "$alpha_payload" session_id)"
if [[ -z "$alpha_id" ]]; then
  echo "Failed to start alpha session" >&2
  printf '%s\n' "$alpha_payload" >&2
  exit 1
fi

beta_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "printf 'beta session\\n'; sleep 20")"
beta_id="$(json_field "$beta_payload" session_id)"
if [[ -z "$beta_id" ]]; then
  echo "Failed to start beta session" >&2
  printf '%s\n' "$beta_payload" >&2
  exit 1
fi

"$BIN" --session "$alpha_id" wait --assert "alpha session" -t 5000 >/dev/null
"$BIN" --session "$beta_id" wait --assert "beta session" -t 5000 >/dev/null

sessions_json="$("$BIN" --json sessions)"
printf '%s\n' "$sessions_json" >"$ARTIFACT_DIR/sessions-list.json"
assert_contains "$sessions_json" "\"active_session\": \"$beta_id\""
assert_contains "$sessions_json" "\"id\": \"$alpha_id\""
assert_contains "$sessions_json" "\"id\": \"$beta_id\""

show_alpha_json="$("$BIN" --json sessions show "$alpha_id")"
printf '%s\n' "$show_alpha_json" >"$ARTIFACT_DIR/show-alpha.json"
assert_contains "$show_alpha_json" "\"id\": \"$alpha_id\""
assert_contains "$show_alpha_json" "\"active_session\": \"$beta_id\""

log_step "Sessions step: switch active session to alpha"
"$BIN" --json sessions switch "$alpha_id" >"$ARTIFACT_DIR/switch-alpha.json"
after_alpha_json="$("$BIN" --json sessions)"
assert_contains "$after_alpha_json" "\"active_session\": \"$alpha_id\""
"$BIN" screenshot --strip-ansi >"$ARTIFACT_DIR/active-alpha.txt"
assert_file_contains "$ARTIFACT_DIR/active-alpha.txt" "alpha session"

log_step "Sessions step: switch active session back to beta"
"$BIN" --json sessions switch "$beta_id" >"$ARTIFACT_DIR/switch-beta.json"
after_beta_json="$("$BIN" --json sessions)"
assert_contains "$after_beta_json" "\"active_session\": \"$beta_id\""
"$BIN" screenshot --strip-ansi >"$ARTIFACT_DIR/active-beta.txt"
assert_file_contains "$ARTIFACT_DIR/active-beta.txt" "beta session"

"$BIN" --session "$alpha_id" kill --yes >/dev/null 2>&1 || true
"$BIN" --session "$beta_id" kill --yes >/dev/null 2>&1 || true

printf '\nSessions switch scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
