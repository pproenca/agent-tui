# Clean Code Audit Fixes - Implementation Plan

> Generated: 2026-01-23
> Status: planned
> Source: [clean-code-audit.md](../../audits/clean-code-audit.md)
> Tasks: 11 total

## Pipeline Status

- [x] Audit: `.claude/audits/clean-code-audit.md`
- [x] Task Plan: This file + TaskList

## Overview

This plan addresses 12 HIGH severity violations from the Clean Code audit, all related to the `cmt-avoid-redundant` rule. The violations are redundant doc comments that restate what the code already expresses.

### Violation Pattern

**In domain/types.rs:**
```rust
// BEFORE (Redundant - struct name is self-documenting)
/// Input for spawning a new session.
pub struct SpawnInput { ... }

// AFTER
pub struct SpawnInput { ... }
```

**In usecases/*.rs:**
```rust
// BEFORE (Redundant - trait name is self-documenting)
/// Use case for clicking an element.
pub trait ClickUseCase { ... }

/// Implementation of the click use case.
pub struct ClickUseCaseImpl { ... }

// AFTER
pub trait ClickUseCase { ... }
pub struct ClickUseCaseImpl { ... }
```

## Phase Overview

### Phase 1: Baseline Verification
Task: #1
Verify current code compiles before any changes.

### Phase 2: Domain Layer Cleanup
Task: #2
Remove ~45 redundant doc comments from `domain/types.rs`.

### Phase 3: Use Case Layer Cleanup
Tasks: #3, #4, #5, #6, #7, #8, #9, #10
Remove redundant doc comments from all 7 use case files:
- `elements.rs`: 37 comments (23 trait + 14 impl)
- `input.rs`: 8 comments (4 trait + 4 impl)
- `snapshot.rs`: 2 comments (1 trait + 1 impl)
- `recording.rs`: 6 comments (3 trait + 3 impl)
- `wait.rs`: 2 comments (1 trait + 1 impl)
- `diagnostics.rs`: 16 comments (8 trait + 8 impl)
- `session.rs`: 12 comments (6 trait + 6 impl)

### Phase 4: Final Verification
Task: #11
Run full test suite and clippy to verify no regressions.

## Task Summary

| ID | Type | Task | Blocked By | Status |
|----|------|------|------------|--------|
| 1 | GREEN | Characterization test: verify baseline compiles | - | pending |
| 2 | REFACTOR | Remove redundant comments from domain/types.rs | 1 | pending |
| 3 | REFACTOR | Remove redundant trait comments from elements.rs | 1 | pending |
| 4 | REFACTOR | Remove redundant impl comments from elements.rs | 3 | pending |
| 5 | REFACTOR | Remove redundant comments from input.rs | 1 | pending |
| 6 | REFACTOR | Remove redundant comments from snapshot.rs | 1 | pending |
| 7 | REFACTOR | Remove redundant comments from recording.rs | 1 | pending |
| 8 | REFACTOR | Remove redundant comments from wait.rs | 1 | pending |
| 9 | REFACTOR | Remove redundant comments from diagnostics.rs | 1 | pending |
| 10 | REFACTOR | Remove redundant comments from session.rs | 1 | pending |
| 11 | GREEN | Run full test suite to verify no regressions | 2-10 | pending |

## Statistics

- Total tasks: 11
- Test tasks (GREEN): 2
- Refactor tasks: 9
- Phases: 4
- Comments to remove: ~128 total
  - domain/types.rs: ~45
  - usecases/*.rs: ~83

## How to Execute

1. Start working - Claude will track via TaskList
2. Each task marked `in_progress` while working
3. Mark `completed` when done
4. Dependencies auto-unblock next tasks

## Dependencies Graph

```
                    [1] Baseline Verification
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
        [2] types.rs    [3] elements.rs   [5-10] other
              │               │           usecases
              │               ▼               │
              │         [4] elements.rs      │
              │              impl            │
              │               │               │
              └───────────────┼───────────────┘
                              │
                              ▼
                    [11] Final Test Suite
```

## Good Practices to Preserve

Per the audit, these comments should NOT be removed:
- `session.rs` lock ordering comment (explains "why" - deadlock prevention)
- `commands.rs` CLI doc comments (functional - used for --help output)
- Test file section separators
- `mock_daemon.rs` design rationale comment
