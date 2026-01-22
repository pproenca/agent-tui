#!/usr/bin/env bash
# ralph-loop.sh - Ralph Wiggum style migration loop
#
# Simple pattern: PROMPT + PLAN + TODO in a loop until done
#
# Usage:
#   .ralph/ralph-loop.sh                    # Run the loop
#   .ralph/ralph-loop.sh --dry-run          # Show prompt without running
#   .ralph/ralph-loop.sh --once             # Run once, don't loop

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"
readonly PROJECT_ROOT
readonly PROMPT_FILE="${SCRIPT_DIR}/PROMPT.md"
readonly PLAN_FILE="${HOME}/.claude/plans/goofy-knitting-hammock.md"
readonly TODO_FILE="${SCRIPT_DIR}/TODO.md"
readonly LOG_FILE="${SCRIPT_DIR}/ralph.log"

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

info() {
  echo -e "${BLUE}[ralph]${NC} $*"
}

success() {
  echo -e "${GREEN}[ralph]${NC} $*"
}

warn() {
  echo -e "${YELLOW}[ralph]${NC} $*"
}

error() {
  echo -e "${RED}[ralph]${NC} $*" >&2
}

# Parse arguments
DRY_RUN=false
RUN_ONCE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --once)
      RUN_ONCE=true
      shift
      ;;
    --help)
      echo "Usage: $0 [--dry-run] [--once]"
      echo ""
      echo "Options:"
      echo "  --dry-run   Show the prompt without running claude"
      echo "  --once      Run once instead of looping"
      exit 0
      ;;
    *)
      error "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Build the full prompt with plan context
build_prompt() {
  cat << EOF
$(cat "${PROMPT_FILE}")

---

## Full Plan (for reference)

\`\`\`markdown
$(cat "${PLAN_FILE}")
\`\`\`
EOF
}

# Check if migration is complete
is_complete() {
  # Check if all TODO items are marked done
  if grep -q '^\- \[ \]' "${TODO_FILE}" 2>/dev/null; then
    return 1  # Still have unchecked items
  fi

  # Check if workspace builds (use subshell to isolate cd)
  if (cd "${PROJECT_ROOT}" && cargo build --workspace >/dev/null 2>&1); then
    return 0  # Complete!
  fi

  return 1  # Build fails
}

# Main loop
main() {
  local iteration=0

  info "Starting ralph-loop migration"
  info "Project: ${PROJECT_ROOT}"
  info "Plan: ${PLAN_FILE}"
  info "Prompt: ${PROMPT_FILE}"
  echo ""

  cd "${PROJECT_ROOT}"

  while :; do
    ((iteration++))

    info "════════════════════════════════════════════════"
    info "  Iteration ${iteration}"
    info "════════════════════════════════════════════════"
    echo ""

    # Check if already complete
    if is_complete; then
      success "MIGRATION COMPLETE!"
      success "All TODO items checked and workspace builds."
      exit 0
    fi

    # Build and send prompt
    local prompt
    prompt=$(build_prompt)

    if [[ "${DRY_RUN}" == "true" ]]; then
      echo "========== PROMPT =========="
      echo "${prompt}"
      echo "============================"
      exit 0
    fi

    # Run claude with the prompt
    info "Sending prompt to claude..."
    echo ""

    if echo "${prompt}" | claude --dangerously-skip-permissions 2>&1 | tee -a "${LOG_FILE}"; then
      success "Iteration ${iteration} complete"
    else
      warn "Claude exited with error, continuing..."
    fi

    echo ""

    # Single run mode
    if [[ "${RUN_ONCE}" == "true" ]]; then
      info "Single run complete (--once mode)"
      exit 0
    fi

    # Brief pause between iterations
    info "Pausing 2s before next iteration..."
    sleep 2
  done
}

main "$@"
