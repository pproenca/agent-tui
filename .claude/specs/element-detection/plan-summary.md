# Element Detection - Implementation Plan

> Generated: 2026-01-23
> Status: planned
> Spec: [spec.md](./spec.md)
> Tasks: 34 total

## Pipeline Status

- [ ] Brainstorm: (skipped - requirements clear from agent-browser reference)
- [x] Specification: `.claude/specs/element-detection/spec.md`
- [x] Task Plan: This file + TaskList

## Phase Overview

### Phase 1: New Role Types (Tasks 1-8)
Add new VOM roles needed for Claude Code element detection:
- `Status` - Spinner and status indicator detection
- `ToolBlock` - Tool use block with rounded borders
- `PromptMarker` - Input prompt marker (`>`)
- Y/N button enhancement for permission dialogs

### Phase 2: Snapshot Format (Tasks 9-16)
Implement agent-browser style snapshot output:
- Text format: `- button "[ OK ]" [ref=e1]`
- RefMap for element lookup by ref ID
- Interactive-only filtering (`-i` flag)
- JSON output with tree, refs, stats

### Phase 3: IPC Integration (Tasks 17-22)
Wire snapshot through daemon:
- IPC request/response types
- Snapshot use case (Clean Architecture)
- JSON-RPC handler registration

### Phase 4: CLI Integration (Tasks 23-28)
CLI commands for snapshot and ref-based interaction:
- `agent-tui snapshot` command
- Click by ref: `agent-tui click @e1`
- Output formatting options

### Phase 5: E2E Validation (Tasks 29-32)
End-to-end tests for Claude Code patterns:
- Permission dialog detection
- Status indicator detection

### Phase 6: Cleanup (Tasks 33-34)
Code organization and robustness:
- Extract patterns to dedicated module
- Property tests for determinism

## Task Summary

| ID | Type | Task | Blocked By | Status |
|----|------|------|------------|--------|
| **Phase 1: New Role Types** |
| 1 | RED | Write test for Status role detection | - | pending |
| 2 | GREEN | Implement Status role detection | 1 | pending |
| 3 | RED | Write test for ToolBlock role detection | - | pending |
| 4 | GREEN | Implement ToolBlock role detection | 3 | pending |
| 5 | RED | Write test for PromptMarker role detection | - | pending |
| 6 | GREEN | Implement PromptMarker role detection | 5 | pending |
| 7 | RED | Write test for Y/N button detection | - | pending |
| 8 | GREEN | Implement Y/N button detection | 7 | pending |
| **Phase 2: Snapshot Format** |
| 9 | RED | Write test for snapshot text format | - | pending |
| 10 | GREEN | Implement snapshot text format | 9,2,4,6,8 | pending |
| 11 | RED | Write test for RefMap generation | - | pending |
| 12 | GREEN | Implement RefMap generation | 11,10 | pending |
| 13 | RED | Write test for interactive filter | - | pending |
| 14 | GREEN | Implement interactive filter | 13,12 | pending |
| 15 | RED | Write test for snapshot JSON output | - | pending |
| 16 | GREEN | Implement snapshot JSON output | 15,14 | pending |
| **Phase 3: IPC Integration** |
| 17 | RED | Write test for snapshot IPC types | - | pending |
| 18 | GREEN | Implement snapshot IPC types | 17,16 | pending |
| 19 | RED | Write test for snapshot use case | - | pending |
| 20 | GREEN | Implement snapshot use case | 19,18 | pending |
| 21 | RED | Write test for snapshot handler | - | pending |
| 22 | GREEN | Implement snapshot handler | 21,20 | pending |
| **Phase 4: CLI Integration** |
| 23 | RED | Write test for CLI snapshot command | - | pending |
| 24 | GREEN | Implement CLI snapshot command | 23,22 | pending |
| 25 | RED | Write test for snapshot command handler | - | pending |
| 26 | GREEN | Implement snapshot command handler | 25,24 | pending |
| 27 | RED | Write test for click by ref | - | pending |
| 28 | GREEN | Implement click by ref | 27,26 | pending |
| **Phase 5: E2E Validation** |
| 29 | RED | Write E2E test for permission dialog | - | pending |
| 30 | GREEN | Verify permission dialog detection | 29,28 | pending |
| 31 | RED | Write E2E test for status indicator | - | pending |
| 32 | GREEN | Verify status indicator detection | 31,30 | pending |
| **Phase 6: Cleanup** |
| 33 | REFACTOR | Extract patterns to module | 32 | pending |
| 34 | REFACTOR | Add property tests | 33 | pending |

## Statistics

- **Total tasks**: 34
- **Test tasks (RED)**: 17
- **Implementation tasks (GREEN)**: 15
- **Refactor tasks**: 2
- **Phases**: 6

## Parallelization Opportunities

Phase 1 tasks can run in parallel (4 independent RED-GREEN pairs):
- Tasks 1-2 (Status)
- Tasks 3-4 (ToolBlock)
- Tasks 5-6 (PromptMarker)
- Tasks 7-8 (Y/N buttons)

Phase 2-6 are sequential due to dependencies.

## How to Execute

1. Start with any unblocked RED task (Tasks 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31)
2. Mark task `in_progress` while working
3. Mark `completed` when tests pass
4. Dependencies auto-unblock next tasks
5. Run `cargo test --workspace` to verify all tests pass

## Key Design Decisions

1. **Output format**: Agent-browser style `- role "name" [ref=eN]`
2. **RefMap**: HashMap for O(1) element lookup by ref
3. **Interactive roles**: Button, Input, Checkbox, Tab, MenuItem, PromptMarker
4. **Non-interactive roles**: StaticText, Panel, Status, ToolBlock
5. **Clean Architecture**: Use case layer between handler and VOM
