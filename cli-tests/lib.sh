#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_ROOT="$ROOT/cli"
BIN="${AGENT_TUI_BIN:-$CLI_ROOT/target/debug/agent-tui}"

KEEP=0
SKIP_BUILD=0
ARTIFACT_DIR=""
RUNTIME_DIR=""
DAEMON_PID=""

parse_common_args() {
  local usage_fn="$1"
  shift

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
        "$usage_fn"
        exit 0
        ;;
      *)
        echo "Unknown argument: $1" >&2
        "$usage_fn" >&2
        exit 2
        ;;
    esac
  done
}

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

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if ! printf '%s' "$haystack" | grep -Fq "$needle"; then
    echo "Expected output to contain: $needle" >&2
    printf '%s\n' "$haystack" >&2
    exit 1
  fi
}

assert_file_contains() {
  local path="$1"
  local needle="$2"
  if ! grep -Fq "$needle" "$path"; then
    echo "Expected file to contain: $needle" >&2
    cat "$path" >&2
    exit 1
  fi
}

log_step() {
  printf '\n[%s] %s\n' "$(date +%H:%M:%S)" "$1"
}

cleanup() {
  local status=$?

  if [[ -x "$BIN" ]]; then
    "$BIN" daemon stop --force --yes >/dev/null 2>&1 || true
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

init_scenario() {
  local slug="$1"

  ARTIFACT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agent-tui-${slug}-artifacts.XXXXXX")"
  RUNTIME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/agtui.XXXXXX")"

  export AGENT_TUI_SOCKET="$RUNTIME_DIR/a.sock"
  export AGENT_TUI_WS_STATE="$RUNTIME_DIR/l.json"
  export AGENT_TUI_WS_LISTEN="127.0.0.1:0"
  export NO_COLOR=1

  trap cleanup EXIT
}

build_agent_if_needed() {
  if [[ "$SKIP_BUILD" -ne 1 && -z "${AGENT_TUI_BIN:-}" ]]; then
    log_step "Building workspace"
    (cd "$CLI_ROOT" && cargo build --workspace)
  fi
}

require_agent_binary() {
  if [[ ! -x "$BIN" ]]; then
    echo "Built binary not found: $BIN" >&2
    exit 1
  fi
}

scenario_bootstrap() {
  local slug="$1"
  local usage_fn="$2"
  shift 2

  parse_common_args "$usage_fn" "$@"
  init_scenario "$slug"
  build_agent_if_needed
  require_agent_binary
}
