# OpenAI Codex Rust Audit Program

## Purpose / Big Picture

Create and execute a persistent, resumable audit program for `/Users/pedroproenca/Documents/Projects/agent-tui` that evaluates the Rust workspace against all 60 rules in the `openai-codex-rust-patterns` skill. The visible outcome is a repo-local audit system that can be resumed by any later session without re-discovery: a full file-by-rule tracker, a findings ledger, and a progress narrative that records which audit units are complete and what was found.

## Progress

- [x] (2026-04-12 22:03Z) Create the persistent audit artifacts: plan, matrix, and findings ledger.
- [x] (2026-04-12 22:03Z) Complete the workspace and architecture tranche (`A08`) and record findings.
- [x] (2026-04-12 22:06Z) Complete the domain invariants tranche (`A01`) and record findings.
- [ ] Complete the error-boundary tranche (`A02`) and record findings.
- [ ] Execute the remaining feature-slice and shared-runtime audits until every audit unit in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md` is marked complete.
- [ ] Close the plan by filling the retrospective and moving it to `completed/`.

## Surprises & Discoveries

- `2026-04-12 22:00Z` The repository does not currently contain `/Users/pedroproenca/Documents/Projects/agent-tui/docs/PLANS.md` or `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/`, so this plan uses the `exec-plan` skill's required structure directly instead of inheriting a local template.
- `2026-04-12 22:03Z` The first matrix draft undercounted the perimeter because it omitted non-`src` audit targets such as `build.rs`, workspace manifests, API specs, and Rust-to-web boundary files. The corrected baseline is `130` targets and `7,800` audit cells.
- `2026-04-12 22:06Z` The domain already contains a strong `TerminalSize` invariant type, but spawn and resize request DTOs still move raw `u16` dimensions through the boundary, which means the invariant is not actually enforced at all entry points.

## Decision Log

- `2026-04-12 22:00Z` Use a generated TSV matrix for the file-by-rule audit cells instead of a Markdown table because the state space is too large for manual editing and needs deterministic regeneration.
- `2026-04-12 22:00Z` Treat the existing audit inventory at `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md` as the coverage perimeter and audit-unit registry rather than duplicating that taxonomy in this plan.
- `2026-04-12 22:03Z` Expand the matrix perimeter beyond Rust `src/**/*.rs` to include manifests, `build.rs`, API specs, and Rust-to-web contract files because those artifacts materially affect the skill's `workspace`, `proto`, and `tui` audit categories.

## Outcomes & Retrospective

(fill when complete)

## Context and Orientation

The codebase is a Unix-only Rust workspace rooted at `/Users/pedroproenca/Documents/Projects/agent-tui/cli`. It contains 8 crates plus workspace tests and tooling:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask`

Definitions used by this audit:

- `rule`: one concrete guideline from `openai-codex-rust-patterns`, for example `defensive-deny-unwrap-workspace-wide`.
- `audit category`: one of the 10 top-level buckets such as `defensive`, `errors`, or `workspace`.
- `audit unit`: one feature slice or shared runtime area defined in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
- `audit cell`: one `file_path × rule_id` review record.
- `matrix`: the generated baseline table of all audit cells.
- `findings ledger`: the narrative log of concrete findings, decisions, and tranche completion status.

Known counts at plan creation:

- Audit targets in scope: `130`
- Rules in scope: `60`
- Baseline audit cells: `7,800`

Authoritative source files for the audit:

- Skill entry point: `/Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/SKILL.md`
- Full skill index: `/Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/AGENTS.md`
- Audit inventory: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`

Persistent artifacts created by this plan:

- Plan: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md`
- Matrix: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`
- Findings ledger: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`

## Plan of Work

### Milestone 1: Create durable audit state

Goal: establish files that let later sessions continue without rebuilding scope.

Work: generate the file-by-rule matrix from the current Rust file list and the 60 skill rule identifiers; create a findings ledger that records tranche completion, findings, and next targets.

Result: any later session can answer "what is left?" and "what has been reviewed?" from repo files instead of conversation history.

Proof: the matrix row count matches `130 × 60 + 1 header`, and the findings ledger contains at least one tranche section.

### Milestone 2: Complete the workspace/architecture tranche

Goal: validate the compiler-level and workspace-level structure before deeper feature audits.

Work: inspect `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`, all crate `Cargo.toml` files, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/clippy.toml`, architecture docs, and architecture tests against the applicable `workspace-*`, `defensive-*`, and `testing-*` rules.

Result: a completed `A08` tranche with recorded passes/findings and matrix cells updated for the directly reviewed workspace files.

Proof: the findings ledger has an `A08` section with verdicts, and the matrix includes non-`pending` rows for the audited files/rules.

### Milestone 3: Complete the domain invariants tranche

Goal: validate the core semantic invariants that affect every feature slice.

Work: audit `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/**/*.rs` and any boundary parser that can weaken those invariants before values reach the use cases.

Result: completed `A01` section with concrete findings or explicit pass notes.

Proof: the ledger documents the completed tranche and the matrix rows for the reviewed domain files show non-`pending` statuses.

### Milestone 4: Complete the error-boundary tranche

Goal: validate the error-boundary discipline that affects every feature slice.

Work: audit `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`, and the surrounding boundary translators.

Result: completed `A02` section with concrete findings or explicit pass notes.

Proof: the ledger documents the completed tranche and the matrix rows for the reviewed error files show non-`pending` statuses.

### Milestone 5: Execute remaining tranches until the inventory is exhausted

Goal: finish every audit unit listed in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.

Work: iterate through the remaining feature slices and shared runtime areas, update the ledger after each tranche, and keep the matrix synchronized.

Result: all audit units are complete and all findings are recorded as accepted, deferred, or actionable.

Proof: no unfinished audit unit remains in the ledger and the plan can be moved to `completed/`.

## Concrete Steps

1. Create `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/` and `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/` if missing.
2. Generate the Rust file list with:
   `find /Users/pedroproenca/Documents/Projects/agent-tui/cli/crates -path '*/src/*.rs' -o -path '*/src/**/*.rs' -o -path '/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/*.rs' -o -path '/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/**/*.rs' | sort`
3. Generate the rule list with:
   `find /Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/references -maxdepth 1 -name '*.md' ! -name '_sections.md' | sort`
4. Build the TSV matrix with header columns:
   `file_path`, `rule_id`, `category`, `status`, `applicability`, `audit_units`, `reviewed_at`, `notes`
5. Create the findings ledger with sections for `Open Findings`, `Completed Tranches`, and `Next Queue`.
6. Audit tranche `A08` and immediately update both the ledger and matrix.
7. Continue with `A01` and `A02` after `A08` is written down.

Expected outputs:

- Matrix exists and has `7801` lines including the header.
- Findings ledger exists and names the next queued tranches.
- The plan file contains timestamped progress updates as milestones are completed.

## Validation and Acceptance

Validation commands:

1. `wc -l /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`
   Expected: `7801`
2. `rg -n '^## Completed Tranches|^## Next Queue' /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
   Expected: both headings present
3. `rg -n 'workspace-single-source-dependencies|workspace-lint-config-package|testing-path-attribute-sibling-tests' /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
   Expected: at least one `A08` review note referencing reviewed rules

Acceptance:

- The repo contains durable audit artifacts, not just a chat summary.
- The first tranche is genuinely complete and recorded.
- The plan can be resumed by a fresh session with no hidden context.

## Idempotence and Recovery

- Re-running the matrix generator is safe because it deterministically regenerates the full baseline from the current file list and the current skill reference list.
- If a later session needs to rebuild the matrix after files are added or removed, it should regenerate the file and record the reason in the `Decision Log`.
- If a tranche is partially reviewed but not finished, the findings ledger must note the partial state and the next exact files to continue from.
- No destructive repository operations are required. Recovery is document-driven: regenerate the matrix, keep the plan, and append findings rather than rewriting history.
