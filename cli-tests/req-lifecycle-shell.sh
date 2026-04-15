#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-lifecycle-shell.sh [--skip-build] [--keep]

Lifecycle scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. run a long-lived shell session
4. restart it, kill it, and clean up the dead session record

Examples:
  cli-tests/req-lifecycle-shell.sh
  cli-tests/req-lifecycle-shell.sh --skip-build
  cli-tests/req-lifecycle-shell.sh --keep
EOF
}

scenario_bootstrap "lifecycle-shell" usage "$@"

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

log_step "Lifecycle step: start a long-lived shell session"
run_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "printf 'lifecycle-ready\n'; sleep 20")"
session_id="$(json_field "$run_payload" session_id)"
if [[ -z "$session_id" ]]; then
  echo "Failed to start lifecycle session" >&2
  printf '%s\n' "$run_payload" >&2
  exit 1
fi

printf '%s\n' "$run_payload" >"$ARTIFACT_DIR/run.json"
"$BIN" --session "$session_id" wait --assert "lifecycle-ready" -t 5000 >/dev/null
"$BIN" --session "$session_id" screenshot --strip-ansi >"$ARTIFACT_DIR/before-restart.txt"

log_step "Lifecycle step: restart the session"
restart_payload="$("$BIN" --session "$session_id" restart --yes --json)"
old_session_id="$(json_field "$restart_payload" old_session_id)"
new_session_id="$(json_field "$restart_payload" new_session_id)"
if [[ -z "$old_session_id" || -z "$new_session_id" ]]; then
  echo "Failed to restart lifecycle session" >&2
  printf '%s\n' "$restart_payload" >&2
  exit 1
fi
if [[ "$old_session_id" == "$new_session_id" ]]; then
  echo "Restart returned the same session id" >&2
  printf '%s\n' "$restart_payload" >&2
  exit 1
fi

printf '%s\n' "$restart_payload" >"$ARTIFACT_DIR/restart.json"
"$BIN" --session "$new_session_id" wait --assert "lifecycle-ready" -t 5000 >/dev/null
"$BIN" --session "$new_session_id" screenshot --strip-ansi >"$ARTIFACT_DIR/after-restart.txt"

sessions_after_restart="$("$BIN" --json sessions)"
printf '%s\n' "$sessions_after_restart" >"$ARTIFACT_DIR/sessions-after-restart.json"
assert_contains "$sessions_after_restart" "\"active_session\": \"$new_session_id\""
if printf '%s' "$sessions_after_restart" | grep -Fq "\"id\": \"$old_session_id\""; then
  echo "Old session still present after restart" >&2
  printf '%s\n' "$sessions_after_restart" >&2
  exit 1
fi

log_step "Lifecycle step: kill the restarted session and clean it up"
"$BIN" --session "$new_session_id" kill --yes >/dev/null
sessions_after_kill="$("$BIN" --json sessions)"
printf '%s\n' "$sessions_after_kill" >"$ARTIFACT_DIR/sessions-after-kill.json"
assert_contains "$sessions_after_kill" "\"active_session\": null"
assert_contains "$sessions_after_kill" "\"sessions\": []"

log_step "Lifecycle step: let a shell exit naturally and clean up the dead record"
cleanup_run_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "printf 'cleanup-ready\n'; sleep 1")"
cleanup_session_id="$(json_field "$cleanup_run_payload" session_id)"
if [[ -z "$cleanup_session_id" ]]; then
  echo "Failed to start cleanup probe session" >&2
  printf '%s\n' "$cleanup_run_payload" >&2
  exit 1
fi

printf '%s\n' "$cleanup_run_payload" >"$ARTIFACT_DIR/cleanup-run.json"
"$BIN" --session "$cleanup_session_id" wait --assert "cleanup-ready" -t 5000 >/dev/null
sleep 2

cleanup_payload="$("$BIN" sessions cleanup --yes --json)"
printf '%s\n' "$cleanup_payload" >"$ARTIFACT_DIR/cleanup.json"
assert_contains "$cleanup_payload" "\"sessions_cleaned\": 1"

sessions_after_cleanup="$("$BIN" --json sessions)"
printf '%s\n' "$sessions_after_cleanup" >"$ARTIFACT_DIR/sessions-after-cleanup.json"
assert_contains "$sessions_after_cleanup" "\"active_session\": null"
assert_contains "$sessions_after_cleanup" "\"sessions\": []"

printf '\nLifecycle shell scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
