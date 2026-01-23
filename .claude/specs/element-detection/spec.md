# Element Detection Specification

> Feature: Claude Code Element Detection
> Status: planning
> Reference: [agent-browser](https://github.com/vercel-labs/agent-browser)

## Overview

Implement comprehensive element detection for TUI applications, with Claude Code as the primary use case. The approach mirrors agent-browser's snapshot system: generate a text-based accessibility tree with refs for deterministic element selection.

## Goals

1. **Full Claude Code coverage**: Detect all interactive elements (permission dialogs, status indicators, tool use blocks)
2. **Agent-browser compatible output**: `- button "Submit" [ref=e1]` format
3. **Interactive filtering**: `-i` flag to show only actionable elements
4. **Stable refs**: Elements can be referenced by `@e1` without re-querying

## Output Format

### Snapshot Tree (agent-browser style)

```
- input ">" [ref=e1] [cursor]
- text "Thinking..." [ref=e2] [status=active]
- panel [ref=e3]
  - text "Use tool: Write" [ref=e4]
  - button "[ Y ]" [ref=e5]
  - button "[ N ]" [ref=e6]
- text "Created file: src/main.rs" [ref=e7]
```

### RefMap Structure

```rust
pub struct RefMap {
    /// Maps ref ID (e.g., "e1") to element metadata
    refs: HashMap<String, ElementRef>,
}

pub struct ElementRef {
    pub role: Role,
    pub name: Option<String>,
    pub bounds: Rect,
    pub visual_hash: u64,
    /// For disambiguation when multiple elements have same role+name
    pub nth: Option<usize>,
}
```

### JSON Output

```json
{
  "tree": "- button \"[ OK ]\" [ref=e1]\n- input \">\" [ref=e2]",
  "refs": {
    "e1": { "role": "button", "name": "[ OK ]", "bounds": { "x": 10, "y": 5, "width": 6, "height": 1 } },
    "e2": { "role": "input", "name": ">", "bounds": { "x": 0, "y": 0, "width": 80, "height": 1 } }
  },
  "stats": {
    "total": 15,
    "interactive": 3,
    "lines": 15
  }
}
```

## Claude Code UI Patterns

### 1. Permission Dialogs (Y/N Prompts)

```
┌─────────────────────────────────────┐
│ Allow tool: Write to src/main.rs?   │
│                                     │
│        [ Y ]       [ N ]            │
└─────────────────────────────────────┘
```

Detection:
- Panel with box-drawing borders
- Button patterns: `[ Y ]`, `[ N ]`, `[Yes]`, `[No]`
- Centered layout within panel bounds

### 2. Status Indicators

```
◐ Thinking...
✓ Done
⠋ Loading...
```

Detection:
- Single character spinner prefix: `◐◑◒◓⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`
- Status text patterns: "Thinking", "Loading", "Processing"
- Role: `Status` (new role to add)

### 3. Tool Use Blocks

```
╭─ Write ──────────────────────────────╮
│ Path: src/main.rs                    │
│ Content:                             │
│   fn main() {                        │
│       println!("Hello");             │
│   }                                  │
╰──────────────────────────────────────╯
```

Detection:
- Rounded box corners: `╭╮╰╯`
- Title in top border
- Role: `ToolBlock` (new role to add)

### 4. Input Prompt

```
> your prompt here_
```

Detection:
- `>` prefix character
- Cursor position marks input area
- Role: `Input` (existing)

### 5. Menu/Selection Lists

```
❯ Option 1 (selected)
  Option 2
  Option 3
```

Detection:
- Selection marker: `❯`, `>`, `→`, `▶`
- Highlighted/inverse video on selected item
- Role: `MenuItem` (existing), add `selected` attribute

## New Roles to Add

| Role | Pattern | Interactive |
|------|---------|-------------|
| `Status` | Spinner + status text | No |
| `ToolBlock` | Rounded box with tool content | No |
| `PromptMarker` | `>` at line start | Yes |
| `Spinner` | Animation character sequence | No |

## Snapshot Options

```
agent-tui snapshot                    # Full element tree
agent-tui snapshot -i                 # Interactive elements only
agent-tui snapshot --json             # JSON output with refs
agent-tui snapshot --format tree      # Indented tree format
agent-tui snapshot --format flat      # Flat list format
```

## API Changes

### New IPC Methods

```rust
// Request
{ "method": "snapshot", "params": { "interactive": true, "format": "tree" } }

// Response
{
  "tree": "...",
  "refs": { ... },
  "stats": { "total": 15, "interactive": 3 }
}
```

### CLI Commands

```bash
# Take snapshot
agent-tui snapshot -s <session>

# Click by ref
agent-tui click -s <session> @e1

# Fill by ref
agent-tui fill -s <session> @e2 "my input"
```

## Architecture

### Clean Architecture Layers

```
Domain Layer (agent-tui-core):
├── vom/
│   ├── mod.rs           # Core types: Rect, Cluster, Component, Role
│   ├── segmentation.rs  # Raster scan → clusters
│   ├── classifier.rs    # Clusters → components with roles
│   ├── snapshot.rs      # NEW: Component tree → text snapshot + RefMap
│   └── patterns/        # NEW: Pattern detection modules
│       ├── mod.rs
│       ├── claude_code.rs   # Claude Code specific patterns
│       ├── dialogs.rs       # Permission dialog detection
│       └── status.rs        # Status/spinner detection

Interface Adapters (agent-tui-daemon):
├── handlers/
│   └── snapshot.rs      # Snapshot request handler
├── usecases/
│   └── snapshot.rs      # Snapshot use case

Infrastructure (agent-tui-cli):
├── commands/
│   └── snapshot.rs      # CLI snapshot command
```

## Testing Strategy

### Unit Tests
- Pattern detection for each Claude Code element type
- Snapshot formatting (tree vs flat vs JSON)
- RefMap generation and lookup

### Property Tests
- Refs are unique across snapshot
- Bounds are within screen dimensions
- Interactive filter only returns interactive roles

### Integration Tests
- End-to-end snapshot via daemon
- Click/fill by ref via daemon

### Golden Tests
- Snapshot output for known screen states
- RefMap structure for known elements

## Success Criteria

1. [ ] All Claude Code interactive elements detected with correct roles
2. [ ] Snapshot output matches agent-browser format
3. [ ] Click/fill by ref works for all interactive elements
4. [ ] Interactive filter reduces output to actionable elements only
5. [ ] JSON output includes complete RefMap with bounds
6. [ ] Status indicators (spinners) correctly identified
7. [ ] Permission dialogs (Y/N) correctly detected as buttons
8. [ ] Tool use blocks identified with content extraction

## Out of Scope

- Hierarchical parent-child relationships (flat list for v1)
- Element state tracking across snapshots (refs regenerated each time)
- Custom role definitions (hardcoded patterns only)
