---
description: "Create comprehensive TDD task list with carpaccio-style breakdown and dependencies"
argument-hint: "<feature-name>"
allowed-tools:
  - Read
  - Write
  - Glob
  - Grep
  - TaskCreate
  - TaskUpdate
  - TaskList
  - AskUserQuestion
  - Task
  - EnterPlanMode
---

# TDD Task Planning Command

Create an extensive, detailed task list using TDD methodology with carpaccio-style slicing. This command is designed for long coding sessions where comprehensive tracking is essential.

## Artifact Chain

This command is part of the brainstorm → spec → plan pipeline:

```
.claude/specs/<feature-name>/
├── brainstorm.md    ← From /brainstorm (optional)
├── spec.md          ← From /spec (recommended input)
└── plan-summary.md  ← Output: task plan summary
```

**Input Priority:**
1. If `.claude/specs/<feature-name>/spec.md` exists → use it as primary input
2. If `.claude/specs/<feature-name>/brainstorm.md` exists → use it (less structured)
3. If neither exists → gather requirements interactively or explore codebase

## Core Principles

### 1. Carpaccio Slicing
Every task must be:
- **Atomic**: Completable in a single focused action (< 30 min of work)
- **Demonstrable**: Has visible, verifiable output
- **Valuable**: Delivers a small increment of functionality
- **Vertical**: Touches all necessary layers (test → impl → integration)

### 2. TDD Workflow (Red-Green-Refactor)
For every feature, create this task chain:
1. **RED** - Write failing test(s) first
2. **GREEN** - Implement minimal code to pass
3. **REFACTOR** - Clean up while keeping tests green

### 3. Dependency Chains
Use `addBlockedBy` to enforce:
- Test tasks block their implementation tasks
- Implementation tasks block refactor tasks
- Earlier phases block later phases

## Execution Flow

### Step 1: Load Context from Artifact Chain

Extract feature name from argument: `$ARGUMENTS`

**Check for existing artifacts in order:**

```
1. .claude/specs/<feature-name>/spec.md      → Full specification (best)
2. .claude/specs/<feature-name>/brainstorm.md → Raw ideas (usable)
3. Neither exists                             → Interactive mode
```

**If spec.md exists:**
- Read the entire specification
- Extract: success criteria, scope, architecture decisions, interfaces, testing strategy
- Use these directly to inform task creation
- Skip most interactive questions (spec has answers)

**If only brainstorm.md exists:**
- Read it for context
- Warn user: "Found brainstorm but no spec. Consider running `/spec <feature-name>` first for better results."
- Offer to continue anyway or run /spec first

**If neither exists:**
- Use AskUserQuestion to gather:
  - What is the end goal?
  - What are the acceptance criteria?
  - Are there any constraints or preferences?

### Step 2: Explore the Codebase
Use the Task tool with subagent_type=Explore to understand:
- Existing architecture and patterns
- Where the feature fits in the codebase
- Related code that might be affected
- Existing test patterns and conventions

### Step 3: Decompose into Phases
Break the feature into 2-6 phases, each representing a logical milestone.

Example phases for "Add user authentication":
1. Domain models (User entity, credentials value object)
2. Repository interface and in-memory implementation
3. Authentication use case
4. HTTP handler/controller
5. Integration tests
6. Error handling edge cases

### Step 4: Slice Each Phase into Carpaccio Tasks
For each phase, create atomic tasks following TDD:

**Pattern A: New Functionality**
```
Phase N: [Feature Component]
├── Task N.1: [RED] Write failing test for [specific behavior]
├── Task N.2: [GREEN] Implement [specific behavior] (blocked by N.1)
├── Task N.3: [RED] Write failing test for [edge case]
├── Task N.4: [GREEN] Handle [edge case] (blocked by N.2, N.3)
└── Task N.5: [REFACTOR] Clean up [component] (blocked by N.4)
```

**Pattern B: Refactoring**
```
Phase N: [Refactor Component]
├── Task N.1: [GREEN] Add characterization test for current behavior
├── Task N.2: [REFACTOR] Extract [abstraction] (blocked by N.1)
├── Task N.3: [GREEN] Verify tests still pass (blocked by N.2)
└── Task N.4: [REFACTOR] Clean up old code (blocked by N.3)
```

### Step 5: Create Tasks with Full Details

Use TaskCreate for EVERY task with these fields:

```
subject: "[RED|GREEN|REFACTOR] <imperative action>"
description: |
  ## Goal
  <What this task accomplishes>

  ## Acceptance Criteria
  - [ ] <Specific, verifiable criterion 1>
  - [ ] <Specific, verifiable criterion 2>

  ## Files to Touch
  - `path/to/file.rs` - <what to do>

  ## Test Command
  `cargo test <specific_test_name>`

  ## Notes
  <Any context, gotchas, or references>

activeForm: "<Present continuous form for spinner>"
```

### Step 6: Set Up Dependencies

After creating all tasks, use TaskUpdate to set up the dependency graph:

```
TaskUpdate(taskId: "2", addBlockedBy: ["1"])  # impl blocked by test
TaskUpdate(taskId: "3", addBlockedBy: ["2"])  # refactor blocked by impl
TaskUpdate(taskId: "5", addBlockedBy: ["4"])  # phase 2 blocked by phase 1
```

### Step 7: Output Summary & Save Artifact

After creating all tasks, call TaskList and:

**1. Save plan summary to `.claude/specs/<feature-name>/plan-summary.md`:**

```markdown
# <Feature Name> - Implementation Plan

> Generated: <timestamp>
> Status: planned
> Spec: [spec.md](./spec.md)
> Tasks: <N> total

## Pipeline Status

- [x] Brainstorm: `.claude/specs/<feature-name>/brainstorm.md`
- [x] Specification: `.claude/specs/<feature-name>/spec.md`
- [x] Task Plan: This file + TaskList

## Phase Overview

### Phase 1: <Name>
<description>
Tasks: <X>-<Y>

### Phase 2: <Name>
<description>
Tasks: <X>-<Y>

...

## Task Summary

| ID | Type | Task | Blocked By | Status |
|----|------|------|------------|--------|
| 1 | RED | Write test for X | - | pending |
| 2 | GREEN | Implement X | 1 | pending |
| 3 | REFACTOR | Clean up X | 2 | pending |
...

## Statistics

- Total tasks: N
- Test tasks (RED): X
- Implementation tasks (GREEN): Y
- Refactor tasks: Z
- Phases: P

## How to Execute

1. Start working - Claude will track via TaskList
2. Each task marked `in_progress` while working
3. Mark `completed` when done
4. Dependencies auto-unblock next tasks

Or for autonomous execution:
- Convert to RALPH: `/ralph-init <feature-name>` using this plan
```

**2. Display summary to user:**

```
## Task Plan Created: [Feature Name]

Artifacts:
- Spec: `.claude/specs/<feature-name>/spec.md`
- Plan: `.claude/specs/<feature-name>/plan-summary.md`
- Tasks: <N> tasks in TaskList

### Phase 1: [Name]
| ID | Type | Task | Blocked By |
|----|------|------|------------|
| 1 | RED | Write test for X | - |
| 2 | GREEN | Implement X | 1 |
| 3 | REFACTOR | Clean up X | 2 |

### Phase 2: [Name]
...

### Statistics
- Total tasks: N
- Test tasks (RED): X
- Implementation tasks (GREEN): Y
- Refactor tasks: Z

### Unresolved Questions
1. [Any questions that need user input]
2. [Decisions that affect implementation]

Ready to start? I'll begin with Task 1.
```

## Task Naming Conventions

### Subject Prefixes
- `[RED]` - Writing a failing test
- `[GREEN]` - Making tests pass
- `[REFACTOR]` - Improving without changing behavior
- `[SETUP]` - Configuration, scaffolding
- `[DOCS]` - Documentation (only when explicitly requested)

### Good Task Names (Imperative, Specific)
- `[RED] Write test for User entity validation`
- `[GREEN] Implement email format validation in User`
- `[REFACTOR] Extract validation logic to ValueObject`
- `[RED] Write integration test for login endpoint`

### Bad Task Names (Vague, Passive)
- `User validation` (not imperative)
- `Tests` (too vague)
- `Refactoring` (no specificity)
- `Handle edge cases` (which ones?)

## Slicing Examples

### Example 1: "Add password reset feature"

**Phase 1: Domain Layer**
1. [RED] Write test for ResetToken value object generation
2. [GREEN] Implement ResetToken with crypto-secure random
3. [RED] Write test for ResetToken expiration check
4. [GREEN] Implement expiration logic in ResetToken
5. [REFACTOR] Extract time utilities to shared module

**Phase 2: Use Case**
6. [RED] Write test for RequestPasswordReset use case - happy path
7. [GREEN] Implement RequestPasswordReset with token generation
8. [RED] Write test for user-not-found scenario
9. [GREEN] Handle user-not-found in use case
10. [REFACTOR] Extract email notification to port/interface

**Phase 3: Infrastructure**
11. [RED] Write test for token repository save/load
12. [GREEN] Implement in-memory TokenRepository
13. [RED] Write test for email sender mock
14. [GREEN] Implement EmailSender port

**Phase 4: Controller**
15. [RED] Write test for POST /reset-password endpoint
16. [GREEN] Implement controller calling use case
17. [RED] Write test for validation errors
18. [GREEN] Handle validation in controller
19. [REFACTOR] Standardize error response format

**Phase 5: Integration**
20. [RED] Write E2E test for full reset flow
21. [GREEN] Wire up all components in main
22. [REFACTOR] Final cleanup and documentation

### Example 2: "Refactor to Clean Architecture"

**Phase 1: Characterization**
1. [GREEN] Add characterization tests for existing behavior
2. [GREEN] Document current dependencies

**Phase 2: Extract Domain**
3. [REFACTOR] Move entities to domain layer
4. [GREEN] Verify tests pass after move
5. [REFACTOR] Remove framework imports from entities
6. [GREEN] Verify tests pass

**Phase 3: Extract Use Cases**
7. [REFACTOR] Create use case input/output ports
8. [REFACTOR] Move business logic to use cases
9. [GREEN] Verify tests pass

**Phase 4: Dependency Inversion**
10. [REFACTOR] Create repository interfaces in domain
11. [REFACTOR] Move implementations to infrastructure
12. [GREEN] Wire up with dependency injection
13. [GREEN] Full test suite passes

## Rules

1. **NEVER skip the RED phase** - Every implementation must have a test first
2. **ONE thing per task** - If you say "and", split it
3. **ALWAYS include test command** - Every task must be verifiable
4. **PREFER many small tasks** over few large ones
5. **USE dependencies** to enforce order
6. **ASK if unclear** - Use AskUserQuestion for any ambiguity

## After Planning

Once the task list is created:
1. User reviews and approves the plan
2. Start with Task 1, mark it `in_progress`
3. Complete each task, marking `completed` when done
4. TaskList shows what's unblocked next
5. Repeat until all tasks complete

## Integration with Long Sessions

This task list persists across context windows:
- Tasks survive conversation restarts
- Dependencies ensure correct ordering
- Status tracking shows progress
- Can be combined with RALPH for fully autonomous execution
