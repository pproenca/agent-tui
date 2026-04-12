# Core Beliefs: Agent-First Operating Principles

agent-tui is built with agents as primary code authors. These beliefs guide
decision-making when there are multiple valid approaches.

## 1. The Compiler Is the First Reviewer

Clean Architecture boundaries are enforced by Cargo crate separation, not by
convention or code review. If a dependency rule can be expressed as a crate
boundary, it must be — so that violations fail at compile time, not in review.

**Decision framework**: When adding a new module, ask: "If an agent puts this
in the wrong layer, will the compiler catch it?" If no, the boundary isn't
strong enough.

## 2. Prefer Boring, Agent-Legible Technology

Technologies with stable APIs, broad training data representation, and
predictable behavior are preferred over cutting-edge alternatives. Agents
reason by pattern-matching against training data — a library with thousands
of examples wins over one released last month.

**Concrete examples in this repo**:
- `clap` for CLI parsing (ubiquitous, derive-macro based, zero ambiguity)
- `axum` for HTTP/WS (well-documented, composable, tower-based)
- `thiserror`/`anyhow` for errors (the Rust community standard)
- `tracing` for observability (the de facto Rust tracing library)

## 3. Forward-Only Dependencies, No Exceptions

The dependency graph flows in one direction: `common → domain → usecases →
adapters/infra → app → facade`. Cross-cutting concerns (logging, errors)
enter through explicit interfaces, not ambient imports.

**Why this matters for agents**: An agent working in `usecases` never needs
to understand PTY internals, daemon lifecycle, or JSON-RPC formatting. The
crate boundary makes it impossible to accidentally couple business logic to
infrastructure. This reduces the context an agent needs to reason correctly.

## 4. Trait Ports Over Concrete Dependencies

Use cases define trait ports (`SessionRepository`, `SessionOps`);
infrastructure provides implementations. This lets agents write and test
business logic without spinning up PTY sessions or daemon processes.

**When to add a new trait port**: When a use case needs something from the
outside world (storage, terminal, network). The use case defines what it
needs; infra decides how to provide it.

## 5. Structured Errors Are Agent Context

Error types are not just for users — they're context that agents use to
diagnose and fix problems. Every error should carry:
- A category (`ErrorCategory`) for programmatic handling
- A human-readable message for debugging
- Source error chain for traceability

Generic errors like "something went wrong" are useless to agents. Specific
errors like "session not found: {id}" let agents self-correct immediately.

## 6. Repository-Local Is the Only Real

If knowledge isn't in the repo, it doesn't exist for agents. Architecture
decisions, team conventions, domain rules — all must be encoded as versioned
artifacts (`ARCHITECTURE.md`, `.harness/`, `docs/`).

**The test**: If a new agent run starts with zero context beyond the repo,
can it make correct decisions? If not, the missing context needs to be encoded.

## 7. Small Crates, Clear Ownership

Each crate owns one concept at one architectural layer. When a crate starts
doing too much, split it. The cost of an extra crate (a few lines in
`Cargo.toml`) is far lower than the cost of an agent misunderstanding a
module's responsibilities.

**Current crate count**: 8 (including xtask). Each averages ~3k LOC.
If a crate exceeds ~5k LOC, consider whether it's doing too much.

## 8. Constraints Enable Agent Speed

Strict rules (Clippy denials, crate boundaries, CI gates) feel restrictive
in a human workflow. For agents, they're multipliers — once encoded,
constraints apply everywhere at once. An agent working within well-defined
boundaries ships faster than one guessing at boundaries that don't exist.

**The posture**: Enforce boundaries centrally (crate deps, Clippy, CI).
Allow autonomy locally (implementation style within a crate). The resulting
code may not match human stylistic preferences — and that's fine, as long
as it's correct, maintainable, and legible to the next agent run.
