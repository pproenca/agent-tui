# Testing Overhaul Plan

Long-running refactor to move agent-tui’s tests to a lean, behaviour-first architecture that aligns with Clean Architecture and Rust systems principles. Keep this document as the shared source of truth across iterations.

## Goals
- Tests describe behaviour and user-visible outcomes, not implementation details.
- Fast, reliable feedback on PRs; slow/expensive checks run nightly.
- Layer-isolated testing: domain ↔ use cases ↔ adapters ↔ infrastructure.
- Deterministic, low-flake runs with explicit runtime budgets.
- Coverage and contract drift visibility; prevent silent protocol breaks.

## Non-Goals
- No broad feature changes to the product itself.
- No strict coverage gating on system/E2E layers (informational only).
- No attempt to keep every existing test; redundancy will be removed.

## Target End-State
- **Pyramid:** Domain/property (~30%), Use-case contracts (~25%), Adapter/CLI contracts (~25%), System/E2E (~10%), Non-functional (~10%).
- **Harness v2:** Shared Tokio runtime (once_cell), fault injection (disconnect/delay/malformed), stable CLI runner, golden snapshot helper.
- **Contracts:** JSON fixtures for requests/responses + structured errors; run against MockDaemon in PRs, real daemon nightly.
- **Adapters/CLI:** Golden outputs (text + JSON) for exit codes and key lines; minimal stable assertions.
- **Use Cases:** Stateful in-memory repos; table-driven sad paths per error category; outcome assertions only.
- **Domain:** Property tests for selectors and invariant checks; deduplicated example lists.
- **System/E2E:** 4–6 real-daemon workflows + 1 failure + 1 lock/timeout case, written in Rust (replaces large bash suite).
- **CI:** `cargo nextest` tiered filters on PRs; nightly runs system + contract-vs-real + mutation on domain/use-case; coverage reported via llvm-cov; runtime/flake budgets enforced.

## Work Streams & Order
1) **Prune & Harness:** Remove no-op/call-count tests; introduce Harness v2 with shared runtime and fault injection.
2) **Contracts:** Add `tests/contracts/` fixtures + runner; convert first CLI file to golden snapshot style.
3) **Use Cases:** Refactor to in-memory repos and outcome-based sad-path tables; drop interaction assertions.
4) **Domain Props:** Add property tests for selector invariants; prune duplicate example tests.
5) **System/E2E:** Recreate minimal Rust E2E suite against real daemon; fix `just test-e2e` target.
6) **CI Matrix:** Switch to nextest tiers; gate PRs on fast tiers; move system/mutation to nightly; publish coverage and runtime budgets.
7) **Quality Gates:** Add contract drift check; set flake/runtime budgets; informational coverage thresholds (domain 85/70 line/branch, adapters 70).

## Runtime Budgets (initial)
- Fast tiers (domain/use-case/contract): <3 minutes CI wall clock.
- System/E2E: <2 minutes nightly; fail on overrun.

## References
- Clean Architecture rules: dependency direction, layer isolation, humble objects at boundaries.
- Rust systems style: clear module boundaries, error handling with thiserror/anyhow, property-based tests where state-free.
- Testing craft (Uncle Bob, Kent Beck): self-validating, intention-revealing tests; minimal duplication; behaviour over mechanics.
