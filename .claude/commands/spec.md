---
description: "Transform brainstorm into formal specification with decisions made"
argument-hint: "<feature-name>"
allowed-tools:
  - Read
  - Write
  - Glob
  - Grep
  - AskUserQuestion
  - Task
  - WebSearch
---

# Spec Command

Transform a brainstorm artifact into a formal, actionable specification with all decisions made.

## Artifact Chain

```
.claude/specs/<feature-name>/
├── brainstorm.md    ← Input (from /brainstorm)
└── spec.md          ← Output (this command)
```

Downstream: `/plan-tdd <feature-name>` reads `spec.md` to create tasks.

## Execution Flow

### Step 1: Load Brainstorm

Extract feature name from argument: `$ARGUMENTS`

Look for brainstorm at: `.claude/specs/<feature-name>/brainstorm.md`

If not found:
```
No brainstorm found for '<feature-name>'.

Options:
○ Run `/brainstorm <feature-name>` first (Recommended)
○ Create spec from scratch
○ Point to different brainstorm file
```

If found, read the entire brainstorm.md file.

### Step 2: Resolve Open Questions

Extract all `- [ ]` items from "Open questions" section.

For each unresolved question, use AskUserQuestion to get a decision:
```
Open Question: <question from brainstorm>

Context: <relevant context from brainstorm>

○ Option A: <if applicable>
○ Option B: <if applicable>
○ Need more research
○ Out of scope for now
```

If "Need more research" is selected, optionally use WebSearch or Task(Explore) to gather information, then re-ask.

### Step 3: Choose Approach

If multiple approaches were identified in brainstorm:

```
Which approach should we pursue?

Based on brainstorm, the options are:

○ Approach A: <name>
  <brief description>
  Pros: <key pros>
  Cons: <key cons>

○ Approach B: <name>
  <brief description>
  Pros: <key pros>
  Cons: <key cons>

○ Hybrid: Combine elements
○ Need more analysis
```

### Step 4: Define Scope

Present must-haves and nice-to-haves from brainstorm:

```
Confirm scope for this specification:

Must-haves (from brainstorm):
☑ <item 1>
☑ <item 2>
☐ <item 3> - move to nice-to-have?

Nice-to-haves:
☐ <item 1> - include in v1?
☐ <item 2>

Add anything missing?
```

### Step 5: Architectural Decisions

Based on chosen approach and codebase context, make architectural decisions:

For each significant decision:
```
Architectural Decision: <topic>

Context: <why this decision matters>

Options:
○ Option A: <description>
○ Option B: <description>

Recommendation: <which and why>
```

Capture as ADR (Architecture Decision Record) format.

### Step 6: Define Interfaces

Based on the approach, define key interfaces/contracts:

- Public API surface
- Data structures
- Integration points
- Error handling strategy

### Step 7: Generate Specification

Write to `.claude/specs/<feature-name>/spec.md`:

```markdown
# <Feature Name> - Specification

> Generated: <timestamp>
> Status: specified
> Brainstorm: [brainstorm.md](./brainstorm.md)
> Next: `/plan-tdd <feature-name>` to create implementation tasks

## Overview

### Problem Statement
<refined from brainstorm - one clear paragraph>

### Solution Summary
<chosen approach in 2-3 sentences>

### Success Criteria
- [ ] <measurable criterion 1>
- [ ] <measurable criterion 2>
- [ ] <measurable criterion 3>

## Scope

### In Scope (v1)
- <feature 1>
- <feature 2>

### Out of Scope
- <explicitly excluded item 1>
- <explicitly excluded item 2>

### Future Considerations
- <potential v2 feature>

## Architecture

### Chosen Approach
<detailed description of the selected approach>

### Rationale
<why this approach over alternatives>

### Component Overview
```
<ASCII diagram or description of components>
```

## Architectural Decisions

### ADR-1: <Decision Title>

**Status:** Accepted

**Context:** <why this decision is needed>

**Decision:** <what we decided>

**Consequences:**
- <positive consequence>
- <negative consequence / trade-off>

### ADR-2: <Decision Title>
...

## Interfaces

### Public API

```rust
// Or appropriate language
<key interfaces, traits, function signatures>
```

### Data Structures

```rust
<key types, structs, enums>
```

### Error Handling

<error strategy, error types, recovery behavior>

## Integration Points

### Dependencies
- <internal dependency>: <how it's used>
- <external dependency>: <how it's used>

### Affected Components
- <component>: <what changes>

## Constraints

### Technical
<from brainstorm, refined>

### Performance Requirements
<specific numbers if applicable>

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| <risk 1> | <H/M/L> | <H/M/L> | <mitigation> |

## Open Items

### Resolved (from brainstorm)
- [x] <question> → <answer>
- [x] <question> → <answer>

### Deferred
- <item deferred to future>

## Testing Strategy

### Unit Tests
<what to unit test>

### Integration Tests
<what to integration test>

### Acceptance Criteria Tests
<how to verify success criteria>

## Rollout Plan

### Phase 1
<initial rollout scope>

### Phase 2
<expanded scope if applicable>

---

## Next Steps

Run `/plan-tdd <feature-name>` to:
1. Decompose into implementation tasks
2. Create TDD task chains
3. Set up dependencies
4. Start building!
```

### Step 8: Summary

Display to user:
```
## Specification Complete

Artifact: `.claude/specs/<feature-name>/spec.md`

### Decisions Made
- Approach: <chosen approach>
- Scope: <X> must-haves, <Y> nice-to-haves deferred
- ADRs: <count> architectural decisions recorded

### Key Interfaces
<list main interfaces defined>

### Risks Identified
<count> risks with mitigations

### Next Step
Run `/plan-tdd <feature-name>` to create implementation tasks.
```

## Spec Quality Checklist

Before completing, verify:
- [ ] All open questions resolved or explicitly deferred
- [ ] Single approach chosen (no "maybe this or that")
- [ ] Success criteria are measurable
- [ ] Scope is explicit (in/out)
- [ ] Key interfaces defined
- [ ] Risks have mitigations
- [ ] Testing strategy outlined
