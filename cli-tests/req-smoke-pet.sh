#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_ROOT="$ROOT/cli"
BIN="${AGENT_TUI_BIN:-$CLI_ROOT/target/debug/agent-tui}"

SKIP_BUILD=0
KEEP=0
ARTIFACT_DIR=""
RUNTIME_DIR=""
DAEMON_PID=""

usage() {
  cat <<'EOF'
Usage: cli-tests/req-smoke-pet.sh [--skip-build] [--keep]

Pet scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. sample top through a shell
4. open vim, type a note, and assert the screen

Examples:
  cli-tests/req-smoke-pet.sh
  cli-tests/req-smoke-pet.sh --skip-build
  cli-tests/req-smoke-pet.sh --keep
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --keep)
      KEEP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

json_field() {
  local payload="$1"
  local field="$2"
  printf '%s' "$payload" \
    | tr -d '\n' \
    | sed -n "s/.*\"${field}\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

log_step() {
  printf '\n[%s] %s\n' "$(date +%H:%M:%S)" "$1"
}

run_agent() {
  "$BIN" "$@"
}

run_agent_json() {
  "$BIN" --json "$@"
}

wait_for_daemon() {
  local attempt
  for attempt in $(seq 1 100); do
    if run_agent_json sessions >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done

  echo "Timed out waiting for daemon readiness" >&2
  return 1
}

assert_text() {
  local session_id="$1"
  local text="$2"
  run_agent --session "$session_id" wait --assert "$text" -t 5000 >/dev/null
}

capture_screen() {
  local session_id="$1"
  local output_path="$2"
  run_agent --session "$session_id" screenshot --strip-ansi >"$output_path"
}

kill_session() {
  local session_id="$1"
  run_agent --session "$session_id" kill --yes >/dev/null 2>&1 || true
}

cleanup() {
  local status=$?

  if [[ -x "$BIN" ]]; then
    run_agent daemon stop --force --yes >/dev/null 2>&1 || true
  fi

  if [[ -n "$DAEMON_PID" ]]; then
    wait "$DAEMON_PID" >/dev/null 2>&1 || true
  fi

  if [[ "$status" -eq 0 && "$KEEP" -ne 1 ]]; then
    rm -rf "$ARTIFACT_DIR" "$RUNTIME_DIR"
  else
    printf '\nArtifacts kept at %s\n' "$ARTIFACT_DIR"
  fi

  exit "$status"
}

trap cleanup EXIT

ARTIFACT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agent-tui-pet-artifacts.XXXXXX")"
RUNTIME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agent-tui-pet-runtime.XXXXXX")"

export AGENT_TUI_SOCKET="$RUNTIME_DIR/agent-tui.sock"
export AGENT_TUI_WS_STATE="$RUNTIME_DIR/live-preview.json"
export AGENT_TUI_WS_LISTEN="127.0.0.1:0"
export NO_COLOR=1

if [[ "$SKIP_BUILD" -ne 1 && -z "${AGENT_TUI_BIN:-}" ]]; then
  log_step "Building workspace"
  (cd "$CLI_ROOT" && cargo build --workspace)
fi

if [[ ! -x "$BIN" ]]; then
  echo "Built binary not found: $BIN" >&2
  exit 1
fi

require_command sh
require_command top
require_command vim

log_step "Starting isolated daemon"
run_agent daemon run >"$ARTIFACT_DIR/daemon.stdout.log" 2>"$ARTIFACT_DIR/daemon.stderr.log" &
DAEMON_PID=$!
wait_for_daemon

log_step "Pet step 1: peek at top"
case "$(uname -s)" in
  Darwin)
    top_command="top -l 1 -stats pid,command,cpu,mem | sed -n '1,15p'; sleep 1"
    ;;
  Linux)
    top_command="top -b -n 1 | sed -n '1,15p'; sleep 1"
    ;;
  *)
    echo "Unsupported platform: $(uname -s)" >&2
    exit 1
    ;;
esac

top_payload="$(run_agent_json run --cols 120 --rows 40 sh -- -lc "$top_command")"
top_session_id="$(json_field "$top_payload" session_id)"
if [[ -z "$top_session_id" ]]; then
  echo "Failed to start top session" >&2
  exit 1
fi

assert_text "$top_session_id" "PID"
assert_text "$top_session_id" "COMMAND"
capture_screen "$top_session_id" "$ARTIFACT_DIR/top.txt"
kill_session "$top_session_id"

log_step "Pet step 2: leave a note in vim"
vim_payload="$(run_agent_json run --cols 120 --rows 40 -- vim -Nu NONE -n)"
vim_session_id="$(json_field "$vim_payload" session_id)"
if [[ -z "$vim_session_id" ]]; then
  echo "Failed to start vim session" >&2
  exit 1
fi

run_agent --session "$vim_session_id" wait --stable -t 5000 >/dev/null
run_agent --session "$vim_session_id" type "iagent-tui pet check" >/dev/null
run_agent --session "$vim_session_id" press Esc >/dev/null
assert_text "$vim_session_id" "agent-tui pet check"
capture_screen "$vim_session_id" "$ARTIFACT_DIR/vim.txt"
kill_session "$vim_session_id"

printf '\nPet scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
