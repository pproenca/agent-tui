---
description: "Check RALPH loop progress and status"
allowed-tools:
  - Read
  - Bash
---

# RALPH Status Check

Display current RALPH loop status from `.ralph/` directory.

## Actions

1. Read `.ralph/status.json` for machine state
2. Read `.ralph/TODO.md` to count completed vs remaining tasks
3. Show recent entries from `.ralph/progress.txt` (last 10 lines)
4. Show recent log output from `.ralph/ralph.log` (last 20 lines)

## Output Format

Present a clear status report:

```
═══════════════════════════════════════
  RALPH Status: [task-name]
═══════════════════════════════════════

Status:     [running|paused|complete|max_iterations]
Iteration:  [N]/[max]
Progress:   [completed]/[total] tasks ([percent]%)
Started:    [timestamp]
Last run:   [timestamp]

## Completed Tasks
- [x] Task 1
- [x] Task 2

## Remaining Tasks
- [ ] Task 3
- [ ] Task 4

## Recent Progress Log
[last 10 lines from progress.txt]

## Recent Output
[last 20 lines from ralph.log]
```

## If No RALPH Loop Exists

If `.ralph/` directory doesn't exist, inform the user:

"No RALPH loop found. Use `/ralph-init` to create one."
