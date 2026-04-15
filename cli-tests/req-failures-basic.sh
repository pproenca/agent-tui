#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-failures-basic.sh [--skip-build] [--keep]

Failures scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. assert missing-session and confirmation-required failures
4. assert wait timeout and cleanup idempotence

Examples:
  cli-tests/req-failures-basic.sh
  cli-tests/req-failures-basic.sh --skip-build
  cli-tests/req-failures-basic.sh --keep
EOF
}

scenario_bootstrap "failures-basic" usage "$@"

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

log_step "Failure step: missing session screenshot"
set +e
"$BIN" --session "missing123" screenshot >"$ARTIFACT_DIR/missing-screenshot.stdout.log" 2>"$ARTIFACT_DIR/missing-screenshot.stderr.log"
missing_screenshot_code=$?
set -e
if [[ "$missing_screenshot_code" -ne 69 ]]; then
  echo "Unexpected exit code for missing-session screenshot: $missing_screenshot_code" >&2
  cat "$ARTIFACT_DIR/missing-screenshot.stderr.log" >&2
  exit 1
fi
assert_file_contains "$ARTIFACT_DIR/missing-screenshot.stderr.log" "Session not found: missing123"

log_step "Failure step: --no-input confirmation gate"
set +e
"$BIN" --no-input --session "missing123" kill >"$ARTIFACT_DIR/no-input-kill.stdout.log" 2>"$ARTIFACT_DIR/no-input-kill.stderr.log"
no_input_kill_code=$?
set -e
if [[ "$no_input_kill_code" -ne 64 ]]; then
  echo "Unexpected exit code for --no-input kill without --yes: $no_input_kill_code" >&2
  cat "$ARTIFACT_DIR/no-input-kill.stderr.log" >&2
  exit 1
fi
assert_file_contains "$ARTIFACT_DIR/no-input-kill.stderr.log" "Confirmation required."

log_step "Failure step: wait timeout"
run_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "sleep 5")"
session_id="$(json_field "$run_payload" session_id)"
if [[ -z "$session_id" ]]; then
  echo "Failed to start timeout session" >&2
  printf '%s\n' "$run_payload" >&2
  exit 1
fi

printf '%s\n' "$run_payload" >"$ARTIFACT_DIR/timeout-run.json"
set +e
"$BIN" --session "$session_id" wait --assert "never-seen" -t 100 >"$ARTIFACT_DIR/wait-timeout.stdout.log" 2>"$ARTIFACT_DIR/wait-timeout.stderr.log"
wait_timeout_code=$?
set -e
if [[ "$wait_timeout_code" -ne 75 ]]; then
  echo "Unexpected exit code for wait timeout: $wait_timeout_code" >&2
  cat "$ARTIFACT_DIR/wait-timeout.stderr.log" >&2
  exit 1
fi
assert_file_contains "$ARTIFACT_DIR/wait-timeout.stderr.log" "Wait condition not met within timeout."

log_step "Failure step: cleanup is idempotent for naturally exited sessions"
"$BIN" --session "$session_id" kill --yes >/dev/null

cleanup_run_payload="$("$BIN" --json run --cols 120 --rows 40 sh -- -lc "printf 'cleanup-once\n'; sleep 1")"
cleanup_session_id="$(json_field "$cleanup_run_payload" session_id)"
if [[ -z "$cleanup_session_id" ]]; then
  echo "Failed to start cleanup probe session" >&2
  printf '%s\n' "$cleanup_run_payload" >&2
  exit 1
fi

printf '%s\n' "$cleanup_run_payload" >"$ARTIFACT_DIR/cleanup-run.json"
"$BIN" --session "$cleanup_session_id" wait --assert "cleanup-once" -t 5000 >/dev/null
sleep 2

cleanup_first="$("$BIN" sessions cleanup --yes --json)"
cleanup_second="$("$BIN" sessions cleanup --yes --json)"
printf '%s\n' "$cleanup_first" >"$ARTIFACT_DIR/cleanup-first.json"
printf '%s\n' "$cleanup_second" >"$ARTIFACT_DIR/cleanup-second.json"
assert_contains "$cleanup_first" "\"sessions_cleaned\": 1"
assert_contains "$cleanup_second" "\"sessions_cleaned\": 0"

sessions_after_cleanup="$("$BIN" --json sessions)"
printf '%s\n' "$sessions_after_cleanup" >"$ARTIFACT_DIR/sessions-after-cleanup.json"
assert_contains "$sessions_after_cleanup" "\"active_session\": null"
assert_contains "$sessions_after_cleanup" "\"sessions\": []"

printf '\nFailures basic scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
