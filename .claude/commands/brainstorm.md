---
description: "Facilitate structured brainstorming session and capture ideas"
argument-hint: "<feature-name>"
allowed-tools:
  - Read
  - Write
  - Glob
  - Grep
  - AskUserQuestion
  - Task
---

# Brainstorm Command

Facilitate a structured brainstorming session and capture raw ideas into a discoverable artifact.

## Artifact Location

All brainstorming artifacts are stored in: `.claude/specs/<feature-name>/brainstorm.md`

This location is used by downstream commands:
- `/spec <feature-name>` reads from here to create formal specification
- `/plan-tdd <feature-name>` can read the full chain

## Execution Flow

### Step 1: Setup

Extract feature name from argument: `$ARGUMENTS`

If no argument provided, use AskUserQuestion:
```
What feature or idea do you want to brainstorm?
- Short name (e.g., "plugin-system", "auth-flow", "real-time-sync")
```

Create directory: `.claude/specs/<feature-name>/`

### Step 2: Context Gathering

Optionally explore the codebase to ground the brainstorm:

Use AskUserQuestion:
```
Should I explore the codebase first to understand relevant context?
○ Yes - explore architecture and patterns (Recommended)
○ No - start with blank slate
```

If yes, use Task tool with subagent_type=Explore to understand:
- Current architecture relevant to the feature
- Similar patterns already in codebase
- Potential integration points

### Step 3: Structured Brainstorming

Guide the user through these dimensions using AskUserQuestion and conversation:

#### 3.1 Problem Space
Ask and capture:
- What problem are we solving?
- Who experiences this problem?
- What's the impact of not solving it?
- Are there workarounds today?

#### 3.2 Solution Space
Ask and capture:
- What's the ideal end state?
- What are possible approaches?
- What similar solutions exist (in this codebase or elsewhere)?
- What are the trade-offs between approaches?

#### 3.3 Constraints
Ask and capture:
- Technical constraints (performance, compatibility, dependencies)
- Business constraints (timeline, resources, scope)
- User constraints (learning curve, migration path)

#### 3.4 Risks & Unknowns
Ask and capture:
- What could go wrong?
- What don't we know yet?
- What needs prototyping or research?

#### 3.5 Success Criteria
Ask and capture:
- How do we know when it's done?
- What are the must-haves vs nice-to-haves?
- What metrics matter?

### Step 4: Idea Dump

Encourage free-form ideas:
```
Now let's capture any other ideas, concerns, or thoughts.
Type freely - I'll organize everything at the end.
```

### Step 5: Generate Artifact

Write to `.claude/specs/<feature-name>/brainstorm.md`:

```markdown
# <Feature Name> - Brainstorm

> Generated: <timestamp>
> Status: brainstorm
> Next: `/spec <feature-name>` to formalize

## Problem Space

### What problem are we solving?
<captured content>

### Who experiences this problem?
<captured content>

### Impact of not solving
<captured content>

### Current workarounds
<captured content>

## Solution Space

### Ideal end state
<captured content>

### Possible approaches

#### Approach A: <name>
- Description: <what>
- Pros: <list>
- Cons: <list>

#### Approach B: <name>
- Description: <what>
- Pros: <list>
- Cons: <list>

### Similar solutions
<captured content>

## Constraints

### Technical
<captured content>

### Business
<captured content>

### User experience
<captured content>

## Risks & Unknowns

### Known risks
<captured content>

### Open questions
- [ ] <question 1>
- [ ] <question 2>

### Needs research/prototyping
<captured content>

## Success Criteria

### Must-haves
- [ ] <criterion>

### Nice-to-haves
- [ ] <criterion>

### Metrics
<captured content>

## Raw Ideas

<free-form captured content>

## Codebase Context

<if explored, relevant findings>

---

## Next Steps

Run `/spec <feature-name>` to:
1. Resolve open questions
2. Choose an approach
3. Create formal specification
```

### Step 6: Summary

Display to user:
```
## Brainstorm Complete

Artifact: `.claude/specs/<feature-name>/brainstorm.md`

### Captured
- Problem: <one-line summary>
- Approaches: <count> identified
- Open questions: <count>
- Risks: <count>

### Next Step
Run `/spec <feature-name>` to formalize into a specification.
```

## Tips for Good Brainstorming

- No idea is bad at this stage
- Capture everything, filter later
- Encourage "what if" thinking
- Don't solve problems yet, just identify them
- Quantity over quality initially
