---
name: code-review
description: "Comprehensive code review orchestrator that launches three specialized review agents in parallel: clean-code-reviewer (readability, maintainability, Clean Code principles), rust-code-reviewer (Rust idioms, safety, performance, principal engineer perspective), and clean-architecture-reviewer (dependency rules, layer separation, architectural boundaries). Use when: (1) reviewing recent code changes before committing, (2) reviewing a PR or CL, (3) user asks to 'review my code', 'check my changes', or 'review this', (4) after completing significant implementation work. This skill follows Google's code review standard: approve CLs that improve overall code health, even if not perfect."
---

# Code Review

Orchestrate a comprehensive code review using three specialized agents in parallel.

## Review Philosophy

Follow Google's code review standard:

> Reviewers should favor approving a CL once it is in a state where it **definitely improves the overall code health** of the system, even if the CL isn't perfect.

Key principles:
- **No perfect code exists**—only better code. Seek continuous improvement, not perfection.
- **Balance progress with quality**—don't block good changes for minor issues.
- **Prefix optional suggestions with "Nit:"**—indicates polish, not requirement.
- **Never approve code that worsens code health**—except in emergencies.
- **Technical facts overrule opinions**—style guide is authoritative on style matters.
- **Mentoring is valuable**—educational comments prefixed with "Nit:" are welcome.

## Workflow

### Step 1: Identify Changes to Review

Determine what code to review:

**If reviewing uncommitted changes:**
```bash
git diff          # Unstaged changes
git diff --staged # Staged changes
```

**If reviewing recent commits:**
```bash
git log -n 5 --oneline                    # Find commits
git diff HEAD~N..HEAD                      # Diff last N commits
git diff main..HEAD                        # Diff from main branch
```

**If reviewing a PR:**
```bash
gh pr diff <number>                        # View PR diff
gh pr view <number> --json files           # List changed files
```

### Step 2: Launch Parallel Review Agents

Launch all three review agents **in parallel** using the Task tool:

1. **clean-code-reviewer** (opus)
   - Evaluates: naming, functions, comments, formatting, error handling, boundaries
   - Focus: readability, maintainability, Clean Code principles

2. **rust-code-reviewer** (opus)
   - Evaluates: design, functionality, complexity, tests, Rust idioms, naming, comments, style
   - Focus: principal engineer perspective, Rust safety and performance

3. **clean-architecture-reviewer** (opus)
   - Evaluates: dependency rules, layer separation, architectural boundaries
   - Focus: Clean Architecture compliance, proper abstractions

**Example prompt for each agent:**
```
Review the following code changes. Focus on [agent's specialty].

Changed files:
- path/to/file1.rs
- path/to/file2.rs

[Include the diff or file contents]
```

### Step 3: Synthesize Results

After all agents complete, synthesize their findings into a unified review:

```markdown
## Code Review Summary

### Overall Assessment
[One paragraph: Is this ready to merge? What are the main concerns?]

### Critical Issues (Must Fix)
[Issues that would worsen code health if merged]

### Important Issues (Should Fix)
[Significant improvements that should be addressed]

### Suggestions (Consider)
[Nice-to-have improvements, prefixed with "Nit:" if optional]

### Positive Observations
[What the code does well—reinforce good practices]

### Verdict
[APPROVE | APPROVE WITH COMMENTS | REQUEST CHANGES]
```

## Conflict Resolution

When reviewers disagree:
1. Prefer technical facts and data over opinions
2. Follow the style guide on style matters
3. For design decisions with multiple valid approaches, accept the author's preference if justified
4. Escalate to team discussion if consensus cannot be reached

## Review Scope

Focus the review on:
- **Recently changed code**—not the entire codebase
- **Behavior changes**—not stylistic preferences in unchanged code
- **Actual problems**—not hypothetical concerns
- **Actionable feedback**—specific suggestions with examples
