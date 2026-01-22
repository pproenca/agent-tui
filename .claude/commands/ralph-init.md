---
description: "Initialize a RALPH loop (.ralph/) with SPEC, PROMPT, TODO, and runner script"
argument-hint: "[task-name]"
allowed-tools:
  - Read
  - Write
  - Bash
  - AskUserQuestion
---

# RALPH Loop Initialization

Create a complete RALPH (Repetitive Autonomous Loop with Persistent History) setup in `.ralph/` directory.

## What is RALPH?

RALPH is an autonomous AI agent pattern where progress persists in files and git history, not the LLM context. Each iteration gets fresh context, picks up from files, implements one task, commits, and repeats until done.

## Core Principle

**Files are memory.** Progress lives in:
- `SPEC.md` - The PRD/requirements (what to build)
- `TODO.md` - Prioritized task checklist (what's left)
- `PROMPT.md` - Instructions for each iteration (how to work)
- `progress.txt` - Learnings log (what was discovered)
- `status.json` - Machine-readable state (for monitoring)
- Git history - What's been done

## Interactive Setup Flow

Ask these questions using AskUserQuestion tool:

### 1. Task Identity
- **Task name**: Short identifier (e.g., "workspace-migration", "auth-feature")
- **Description**: One-line summary of what RALPH will accomplish

### 2. Specification Details
- **Goal**: What is the end state? (Be specific and measurable)
- **Phases**: Break work into 2-5 phases (each phase = group of related tasks)
- **Success criteria**: How do we know it's done? (tests pass, builds succeed, etc.)

### 3. Task Breakdown
For each phase, ask:
- What are the individual tasks? (Each should fit in one context window)
- What order should they execute?
- What verification command confirms each task?

### 4. Context
- **Plan file path**: Optional reference to an existing plan (e.g., `~/.claude/plans/my-plan.md`)
- **Import mappings**: Any before/after mappings for refactoring tasks

## File Generation

After collecting answers, generate these files:

### .ralph/SPEC.md

```markdown
# [Task Name] Specification

## Goal
[One paragraph describing the end state]

## Success Criteria
- [ ] [Measurable criterion 1]
- [ ] [Measurable criterion 2]
- [ ] Final verification: `[command]` passes

## Phases

### Phase 1: [Name]
[Description of this phase's purpose]

### Phase 2: [Name]
[Description]

[... more phases]

## Context
[Any import mappings, dependencies, constraints]

## Out of Scope
- [What this RALPH loop will NOT do]
```

### .ralph/TODO.md

```markdown
# [Task Name] Progress

## Phase 1: [Name]

| Status | Task | Verification |
|--------|------|--------------|
| [ ] | [Task description] | `[verify command]` |
| [ ] | [Task description] | `[verify command]` |

## Phase 2: [Name]

| Status | Task | Verification |
|--------|------|--------------|
| [ ] | [Task description] | `[verify command]` |

[... more phases]

## Completion
- [ ] All tasks checked
- [ ] Final verification passes: `[command]`
```

### .ralph/PROMPT.md

```markdown
# [Task Name]

You are completing [brief description] following the RALPH pattern.

## Your Context

1. Read the spec: `cat .ralph/SPEC.md`
2. Check progress: `cat .ralph/TODO.md`
3. Verify current state: `[verification command]`

## Your Task

1. **Find next task**: First unchecked `[ ]` item in TODO.md
2. **Implement it**: Make the minimal changes needed
3. **Verify**: Run the task's verification command
4. **Update TODO.md**: Mark the task `[x]` complete
5. **Log learnings**: Append to `.ralph/progress.txt`:
   - What you did
   - Any gotchas discovered
   - Patterns for future iterations
6. **Report**: State what you did and what's next

## Rules

- One task per iteration (keep changes small)
- Always verify before marking complete
- If stuck, document the blocker in TODO.md and progress.txt, then continue
- Follow existing code patterns
- Commit after each task with descriptive message

## Stop Condition

When all TODO.md items are `[x]` AND final verification passes, output:

<promise>RALPH_COMPLETE</promise>
```

### .ralph/progress.txt

```text
# [Task Name] Progress Log
# Each iteration appends learnings here

---
[Initialized: YYYY-MM-DD HH:MM]
```

### .ralph/status.json

```json
{
  "task": "[task-name]",
  "status": "pending",
  "iteration": 0,
  "maxIterations": 50,
  "startedAt": null,
  "lastRunAt": null,
  "completedTasks": 0,
  "totalTasks": 0,
  "currentPhase": null
}
```

### .ralph/ralph-loop.sh

```bash
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

# Count completed vs total tasks
count_tasks() {
  local completed=0
  local total=0
  if [[ -f "${TODO_FILE}" ]]; then
    completed=$(grep -cE '^\| \[x\]|^- \[x\]' "${TODO_FILE}" 2>/dev/null || echo 0)
    total=$(grep -cE '^\| \[.\]|^- \[.\]' "${TODO_FILE}" 2>/dev/null || echo 0)
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

  cat > "${STATUS_FILE}" << EOF
{
  "status": "${status}",
  "iteration": ${iteration},
  "maxIterations": ${MAX_ITERATIONS},
  "startedAt": $(jq -r '.startedAt // null' "${STATUS_FILE}" 2>/dev/null || echo null),
  "lastRunAt": "${now}",
  "completedTasks": ${completed},
  "totalTasks": ${total}
}
EOF
}

# Show status
show_status() {
  if [[ -f "${STATUS_FILE}" ]]; then
    echo "═══════════════════════════════════════"
    echo "  RALPH Status"
    echo "═══════════════════════════════════════"
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

is_complete() {
  if grep -qE '^\| \[ \]' "${TODO_FILE}" 2>/dev/null; then
    return 1
  fi
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

  # Initialize status
  if [[ ! -f "${STATUS_FILE}" ]] || [[ $(jq -r '.startedAt' "${STATUS_FILE}") == "null" ]]; then
    cat > "${STATUS_FILE}" << EOF
{
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
    ((iteration++))

    info "═══════════════════════════════════════"
    info "  Iteration ${iteration}/${MAX_ITERATIONS}"
    info "═══════════════════════════════════════"
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

    info "Running claude..."
    if cat "${PROMPT_FILE}" | claude --dangerously-skip-permissions 2>&1 | tee -a "${LOG_FILE}"; then
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
```

## Execution

After generating files:

1. Create the `.ralph/` directory
2. Write all files:
   - `SPEC.md` - Requirements/PRD
   - `TODO.md` - Task checklist
   - `PROMPT.md` - Iteration instructions
   - `progress.txt` - Learnings log
   - `status.json` - Machine state
   - `ralph-loop.sh` - Runner script
3. Make `ralph-loop.sh` executable: `chmod +x .ralph/ralph-loop.sh`
4. Show the user a summary of what was created

## How to Run

```bash
# Interactive (watch it work)
.ralph/ralph-loop.sh

# Single iteration (test first)
.ralph/ralph-loop.sh --once

# AFK in tmux (detach with Ctrl-B D)
tmux new -s ralph '.ralph/ralph-loop.sh'

# Check status anytime
.ralph/ralph-loop.sh --status
# or from Claude:
/ralph-status

# Watch logs
tail -f .ralph/ralph.log
```

## Tips for Good RALPH Tasks

- **Measurable**: "Add login form" not "improve auth"
- **Small**: Each task fits in one context window
- **Verifiable**: Every task has a check command
- **Ordered**: Dependencies flow correctly
- **Atomic**: One commit per task
