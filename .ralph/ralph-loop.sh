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

# Configurable constants
readonly MAX_ITERATIONS="${RALPH_MAX_ITERATIONS:-50}"
readonly ITERATION_DELAY="${RALPH_ITERATION_DELAY:-2}"
readonly COMPLETION_CHECK_LINES="${RALPH_COMPLETION_CHECK_LINES:-100}"

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

# Date format for ISO timestamps
readonly DATE_FORMAT="%Y-%m-%dT%H:%M:%SZ"
now() { date -u +"${DATE_FORMAT}"; }

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
      echo "  RALPH_MAX_ITERATIONS        Max iterations (default: 50)"
      echo "  RALPH_ITERATION_DELAY       Seconds between iterations (default: 2)"
      echo "  RALPH_COMPLETION_CHECK_LINES Lines to check for completion signal (default: 100)"
      echo ""
      echo "AFK Monitoring:"
      echo "  tmux new -s ralph '.ralph/ralph-loop.sh'"
      echo "  tail -f .ralph/ralph.log"
      exit 0
      ;;
    *) error "Unknown option: $1"; exit 1 ;;
  esac
done

# Check if there are incomplete tasks (handles both table and list formats)
has_incomplete_tasks() {
  [[ -f "${TODO_FILE}" ]] && grep -qE '^\| \[ \]|^- \[ \]' "${TODO_FILE}" 2>/dev/null
}

# Count completed vs total tasks (handles both formats)
count_tasks() {
  local completed=0 total=0
  if [[ -f "${TODO_FILE}" ]]; then
    # Table format: | [x] | or | [ ] |  and  List format: - [x] or - [ ]
    completed=$(grep -cE '^\| \[x\]|^- \[x\]' "${TODO_FILE}" 2>/dev/null || true)
    total=$(grep -cE '^\| \[.\]|^- \[.\]' "${TODO_FILE}" 2>/dev/null || true)
  fi
  echo "${completed}:${total}"
}

# Write status.json with given values
write_status_json() {
  local status="$1" iteration="$2" started_at="$3" last_run="$4" completed="$5" total="$6"
  cat > "${STATUS_FILE}" << EOF
{
  "task": "workspace-migration",
  "status": "${status}",
  "iteration": ${iteration},
  "maxIterations": ${MAX_ITERATIONS},
  "startedAt": ${started_at},
  "lastRunAt": "${last_run}",
  "completedTasks": ${completed},
  "totalTasks": ${total}
}
EOF
}

# Update status.json
update_status() {
  local status="$1" iteration="$2"
  local current_time started_at
  current_time=$(now)
  started_at=$(jq '.startedAt' "${STATUS_FILE}" 2>/dev/null || echo null)
  IFS=':' read -r completed total <<< "$(count_tasks)"
  write_status_json "${status}" "${iteration}" "${started_at}" "${current_time}" "${completed}" "${total}"
}

# Show status
show_status() {
  if [[ -f "${STATUS_FILE}" ]]; then
    echo "========================================"
    echo "  RALPH Status"
    echo "========================================"
    local status iteration completed total last_run
    read -r status iteration completed total last_run < <(
      jq -r '[.status, .iteration, .completedTasks, .totalTasks, (.lastRunAt // "never")] | @tsv' "${STATUS_FILE}"
    )
    echo "Status:     ${status}"
    echo "Iteration:  ${iteration}/${MAX_ITERATIONS}"
    echo "Progress:   ${completed}/${total} tasks"
    echo "Last run:   ${last_run}"
    echo ""
    echo "Recent log:"
    tail -20 "${LOG_FILE}" 2>/dev/null || echo "(no log yet)"
  else
    echo "No RALPH status found. Run the loop first."
  fi
}

# Log to progress.txt (ISO format for consistency)
log_progress() {
  local msg="$1"
  echo "[$(now)] ${msg}" >> "${PROGRESS_FILE}"
}

# jq filter for streaming Claude output (formatted for readability)
readonly STREAM_FILTER='
  select(.type == "assistant").message.content[]?
  | select(.type == "text").text // empty
  | gsub("\n"; "\r\n")
  | . + "\r\n\n"
'

main() {
  if ${SHOW_STATUS}; then
    show_status
    exit 0
  fi

  # Change to project root early (before any file checks)
  cd "${PROJECT_ROOT}"

  # Verify working directory
  info "Working directory: $(pwd)"
  info "Skills dir exists: $([[ -d .claude ]] && echo yes || echo no)"
  if [[ ! -f "CLAUDE.md" ]]; then
    warn "CLAUDE.md not found at ${PROJECT_ROOT} - Claude may not load project context"
  fi

  local iteration=0
  local start_time
  start_time=$(now)

  info "Starting RALPH loop"
  info "Project: ${PROJECT_ROOT}"
  info "Max iterations: ${MAX_ITERATIONS}"
  echo ""

  # Initialize status if needed
  if [[ ! -f "${STATUS_FILE}" ]] || jq -e '.startedAt == null' "${STATUS_FILE}" >/dev/null 2>&1; then
    write_status_json "running" 0 "\"${start_time}\"" "${start_time}" 0 0
  fi

  log_progress "RALPH loop started (max ${MAX_ITERATIONS} iterations)"

  while [[ ${iteration} -lt ${MAX_ITERATIONS} ]]; do
    ((++iteration))

    info "========================================"
    info "  Iteration ${iteration}/${MAX_ITERATIONS}"
    info "========================================"
    echo ""

    update_status "running" "${iteration}"

    if ! has_incomplete_tasks; then
      success "RALPH_COMPLETE - All tasks done!"
      update_status "complete" "${iteration}"
      log_progress "COMPLETE: All tasks finished"
      exit 0
    fi

    if ${DRY_RUN}; then
      echo "=== PROMPT ==="
      cat "${PROMPT_FILE}"
      echo "=============="
      exit 0
    fi

    info "Running claude..."
    if claude --verbose --print --output-format stream-json --dangerously-skip-permissions < "${PROMPT_FILE}" | tee "${LOG_FILE}" | jq --unbuffered -rj "$STREAM_FILTER"; then
      success "Iteration ${iteration} complete"
      log_progress "Iteration ${iteration} completed"
    else
      warn "Claude exited with error, continuing..."
      log_progress "Iteration ${iteration} had errors"
    fi

    echo ""

    if ${RUN_ONCE}; then
      info "Single iteration complete (--once)"
      update_status "paused" "${iteration}"
      exit 0
    fi

    # Check for completion signal in output
    if tail -"${COMPLETION_CHECK_LINES}" "${LOG_FILE}" 2>/dev/null | grep -q "RALPH_COMPLETE"; then
      success "RALPH_COMPLETE signal detected!"
      update_status "complete" "${iteration}"
      log_progress "COMPLETE: Signal detected"
      exit 0
    fi

    sleep "${ITERATION_DELAY}"
  done

  warn "Max iterations (${MAX_ITERATIONS}) reached"
  update_status "max_iterations" "${iteration}"
  log_progress "STOPPED: Max iterations reached"
  exit 1
}

main "$@"
