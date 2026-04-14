#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_ROOT="$ROOT/cli"
SUITE_DIR="$ROOT/cli-tests"
BIN="${AGENT_TUI_BIN:-$CLI_ROOT/target/debug/agent-tui}"

KEEP=0
FILTER=""
TIER="required"

usage() {
  cat <<'EOF'
Usage: cli-tests/run.sh [--keep] [--filter name] [--tier required|all]

Runs every bash scenario in cli-tests/ against the built agent-tui binary.

Examples:
  cli-tests/run.sh
  cli-tests/run.sh --filter pet
  cli-tests/run.sh --tier all
  cli-tests/run.sh --keep
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep)
      KEEP=1
      shift
      ;;
    --filter)
      FILTER="${2:-}"
      shift 2
      ;;
    --tier)
      TIER="${2:-}"
      shift 2
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

case "$TIER" in
  required|all) ;;
  *)
    echo "Unsupported --tier value: $TIER" >&2
    exit 2
    ;;
esac

log_step() {
  printf '\n[%s] %s\n' "$(date +%H:%M:%S)" "$1"
}

if [[ -z "${AGENT_TUI_BIN:-}" ]]; then
  log_step "Building workspace"
  (cd "$CLI_ROOT" && cargo build --workspace)
fi

if [[ ! -x "$BIN" ]]; then
  echo "Built binary not found: $BIN" >&2
  exit 1
fi

case "$TIER" in
  required)
    scenario_glob='req-*.sh'
    ;;
  all)
    scenario_glob='*.sh'
    ;;
esac

mapfile -t scenarios < <(
  find "$SUITE_DIR" -maxdepth 1 -type f -name "$scenario_glob" ! -name 'run.sh' | sort
)

if [[ -n "$FILTER" ]]; then
  filtered=()
  for scenario in "${scenarios[@]}"; do
    name="$(basename "$scenario" .sh)"
    if [[ "$name" == *"$FILTER"* ]]; then
      filtered+=("$scenario")
    fi
  done
  scenarios=("${filtered[@]}")
fi

if [[ "${#scenarios[@]}" -eq 0 ]]; then
  echo "No cli-tests scenarios found" >&2
  exit 1
fi

passed=0
for scenario in "${scenarios[@]}"; do
  name="$(basename "$scenario")"
  log_step "Running $name"
  AGENT_TUI_BIN="$BIN" bash "$scenario" --skip-build $([[ "$KEEP" -eq 1 ]] && printf '%s' --keep)
  passed=$((passed + 1))
done

printf '\nCLI test suite passed.\n'
printf 'Tier: %s\n' "$TIER"
printf 'Scenarios: %d\n' "$passed"
