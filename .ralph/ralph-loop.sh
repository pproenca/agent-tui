#!/usr/bin/env bash
# ralph-loop.sh - Run RALPH iterations until complete
#
# Usage:
#   .ralph/ralph-loop.sh              # Run the loop
#   .ralph/ralph-loop.sh --dry-run    # Show prompt without running
#   .ralph/ralph-loop.sh --once       # Single iteration
#   .ralph/ralph-loop.sh --status     # Show current status
#
# AFK Monitoring:
#   tmux new -s ralph '.ralph/ralph-loop.sh'  # Run in tmux
#   tmux attach -t ralph                       # Reattach later
#   tail -f .ralph/ralph.log                   # Watch log
#   /ralph-status                              # Check from Claude

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
readonly PROJECT_ROOT
readonly PROMPT_FILE="${SCRIPT_DIR}/PROMPT.md"
readonly TODO_FILE="${SCRIPT_DIR}/TODO.md"
readonly STATUS_FILE="${SCRIPT_DIR}/status.json"
readonly PROGRESS_FILE="${SCRIPT_DIR}/progress.txt"
readonly LOG_FILE="${SCRIPT_DIR}/ralph.log"
readonly MAX_ITERATIONS="${RALPH_MAX_ITERATIONS:-50}"

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

info() { echo -e "${BLUE}[ralph]${NC} $*"; }
success() { echo -e "${GREEN}[ralph]${NC} $*"; }
warn() { echo -e "${YELLOW}[ralph]${NC} $*"; }
error() { echo -e "${RED}[ralph]${NC} $*" >&2; }

DRY_RUN=false
RUN_ONCE=false
SHOW_STATUS=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true; shift ;;
    --once) RUN_ONCE=true; shift ;;
    --status) SHOW_STATUS=true; shift ;;
    --help)
      echo "Usage: $0 [--dry-run] [--once] [--status]"
      echo "  --dry-run  Show prompt without running"
      echo "  --once     Single iteration"
      echo "  --status   Show current status and exit"
      echo ""
      echo "Environment:"
      echo "  RALPH_MAX_ITERATIONS  Max iterations (default: 50)"
      echo ""
      echo "AFK Monitoring:"
      echo "  tmux new -s ralph '.ralph/ralph-loop.sh'"
      echo "  tail -f .ralph/ralph.log"
      exit 0
      ;;
    *) error "Unknown option: $1"; exit 1 ;;
  esac
done

# Count completed vs total tasks (handles both formats)
count_tasks() {
  local completed=0
  local total=0
  if [[ -f "${TODO_FILE}" ]]; then
    # Table format: | [x] | or | [ ] |
    # Note: grep -c outputs "0" even on no matches (exit code 1), so use || : not || echo 0
    completed=$(grep -cE '^\| \[x\]' "${TODO_FILE}" 2>/dev/null || :)
    local table_total
    table_total=$(grep -cE '^\| \[.\]' "${TODO_FILE}" 2>/dev/null || :)
    # List format: - [x] or - [ ]
    local list_completed
    list_completed=$(grep -cE '^- \[x\]' "${TODO_FILE}" 2>/dev/null || :)
    local list_total
    list_total=$(grep -cE '^- \[.\]' "${TODO_FILE}" 2>/dev/null || :)
    completed=$((completed + list_completed))
    total=$((table_total + list_total))
  fi
  echo "${completed}:${total}"
}

# Update status.json
update_status() {
  local status="$1"
  local iteration="$2"
  local counts
  counts=$(count_tasks)
  local completed="${counts%%:*}"
  local total="${counts##*:}"
  local now
  now=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  local started_at
  # Use jq to output proper JSON (quoted string or null)
  started_at=$(jq '.startedAt' "${STATUS_FILE}" 2>/dev/null || echo null)

  cat > "${STATUS_FILE}" << EOF
{
  "task": "workspace-migration",
  "status": "${status}",
  "iteration": ${iteration},
  "maxIterations": ${MAX_ITERATIONS},
  "startedAt": ${started_at},
  "lastRunAt": "${now}",
  "completedTasks": ${completed},
  "totalTasks": ${total}
}
EOF
}

# Show status
show_status() {
  if [[ -f "${STATUS_FILE}" ]]; then
    echo "========================================"
    echo "  RALPH Status"
    echo "========================================"
    local status iteration completed total
    status=$(jq -r '.status' "${STATUS_FILE}")
    iteration=$(jq -r '.iteration' "${STATUS_FILE}")
    completed=$(jq -r '.completedTasks' "${STATUS_FILE}")
    total=$(jq -r '.totalTasks' "${STATUS_FILE}")
    echo "Status:     ${status}"
    echo "Iteration:  ${iteration}/${MAX_ITERATIONS}"
    echo "Progress:   ${completed}/${total} tasks"
    echo "Last run:   $(jq -r '.lastRunAt // "never"' "${STATUS_FILE}")"
    echo ""
    echo "Recent log:"
    tail -20 "${LOG_FILE}" 2>/dev/null || echo "(no log yet)"
  else
    echo "No RALPH status found. Run the loop first."
  fi
}

# Check if migration is complete (handles both formats)
is_complete() {
  # Table format: | [ ] |
  if grep -qE '^\| \[ \]' "${TODO_FILE}" 2>/dev/null; then
    return 1
  fi
  # List format: - [ ]
  if grep -qE '^- \[ \]' "${TODO_FILE}" 2>/dev/null; then
    return 1
  fi
  return 0
}

# Log to progress.txt
log_progress() {
  local msg="$1"
  echo "[$(date '+%Y-%m-%d %H:%M')] ${msg}" >> "${PROGRESS_FILE}"
}

main() {
  if [[ "${SHOW_STATUS}" == "true" ]]; then
    show_status
    exit 0
  fi

  local iteration=0
  local start_time
  start_time=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

  info "Starting RALPH loop"
  info "Project: ${PROJECT_ROOT}"
  info "Max iterations: ${MAX_ITERATIONS}"
  echo ""

  cd "${PROJECT_ROOT}"

  # Initialize status if needed
  if [[ ! -f "${STATUS_FILE}" ]] || [[ $(jq -r '.startedAt' "${STATUS_FILE}") == "null" ]]; then
    cat > "${STATUS_FILE}" << EOF
{
  "task": "workspace-migration",
  "status": "running",
  "iteration": 0,
  "maxIterations": ${MAX_ITERATIONS},
  "startedAt": "${start_time}",
  "lastRunAt": "${start_time}",
  "completedTasks": 0,
  "totalTasks": 0
}
EOF
  fi

  log_progress "RALPH loop started (max ${MAX_ITERATIONS} iterations)"

  while [[ ${iteration} -lt ${MAX_ITERATIONS} ]]; do
    ((++iteration))

    info "========================================"
    info "  Iteration ${iteration}/${MAX_ITERATIONS}"
    info "========================================"
    echo ""

    update_status "running" "${iteration}"

    if is_complete; then
      success "RALPH_COMPLETE - All tasks done!"
      update_status "complete" "${iteration}"
      log_progress "COMPLETE: All tasks finished"
      exit 0
    fi

    if [[ "${DRY_RUN}" == "true" ]]; then
      echo "=== PROMPT ==="
      cat "${PROMPT_FILE}"
      echo "=============="
      exit 0
    fi

    stream_text='select(.type == "assistant").message.content[]? | select(.type == "text").text // empty | gsub("\n"; "\r\n") | . + "\r\n\n"'

    info "Running claude..."
    if cat "${PROMPT_FILE}" | claude --verbose --print --output-format stream-json --dangerously-skip-permissions | tee "${LOG_FILE}" | jq --unbuffered -rj "$stream_text"; then
      success "Iteration ${iteration} complete"
      log_progress "Iteration ${iteration} completed"
    else
      warn "Claude exited with error, continuing..."
      log_progress "Iteration ${iteration} had errors"
    fi

    echo ""

    if [[ "${RUN_ONCE}" == "true" ]]; then
      info "Single iteration complete (--once)"
      update_status "paused" "${iteration}"
      exit 0
    fi

    # Check for completion signal in output
    if tail -100 "${LOG_FILE}" 2>/dev/null | grep -q "RALPH_COMPLETE"; then
      success "RALPH_COMPLETE signal detected!"
      update_status "complete" "${iteration}"
      log_progress "COMPLETE: Signal detected"
      exit 0
    fi

    sleep 2
  done

  warn "Max iterations (${MAX_ITERATIONS}) reached"
  update_status "max_iterations" "${iteration}"
  log_progress "STOPPED: Max iterations reached"
  exit 1
}

main "$@"
