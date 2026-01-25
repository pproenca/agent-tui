# Command DX Audit - Brainstorm

> Generated: 2026-01-25
> Status: brainstorm
> Next: `/spec command-dx-audit` to formalize

## Problem Space

### What problem are we solving?

A comprehensive audit of agent-tui's command developer experience (DX) and AI experience, ensuring the CLI is production-ready for programmatic use by Claude Code and other AI models. The audit covers:

1. **AI integration issues** - Models struggle to understand command output, parse errors, and decide when to retry
2. **Developer friction** - Human developers find the CLI hard to debug or integrate into scripts
3. **Production stability** - Commands fail unexpectedly, exit codes are incorrect, multi-threading causes race conditions

### Who experiences this problem?

**Primary**: AI agents (Claude Code and other LLMs) programmatically controlling TUI applications via agent-tui commands

**Secondary**:
- Script authors writing automation
- Power users interactively testing/debugging

### Impact of not solving

**Blocks AI adoption** - Claude Code cannot reliably use agent-tui, making it unsuitable for production AI workflows. This is the critical blocker - if AI agents can't trust command output and exit codes, they can't build reliable automation on top of agent-tui.

### Current workarounds

- Manual intervention to debug failures
- Special-case handling in AI prompts for known inconsistencies
- Retry loops with arbitrary backoff for flaky commands
- Parsing unstructured text output with regex

## Solution Space

### Ideal end state

A CLI that is:

1. **Machine-first output** - All commands return structured JSON with consistent schema, deterministic error codes
2. **Self-documenting** - Commands explain what they did, what changed, and what to do next in output
3. **Fail-fast semantics** - Commands fail immediately with clear reasons rather than hanging or timing out silently

### Possible approaches

#### Approach A: JSON Schema Standardization

- Define strict JSON schemas for all command outputs
- Every command returns `{ "success": bool, "data": {...}, "error": {...} }`
- Error responses include category, code, retryable flag, and suggested action
- **Pros**: Machine-parseable, consistent, self-documenting
- **Cons**: Breaking change for scripts parsing current output

#### Approach B: Exit Code Rationalization

- Audit all exit codes against sysexits.h and LSB conventions
- Define semantic exit code ranges (0=success, 1-63=user errors, 64-78=system errors)
- Map all error categories to specific exit codes
- **Pros**: Works with shell scripts, standard Unix semantics
- **Cons**: Limited information density in single integer

#### Approach C: Error Protocol Enhancement

- Every error includes: code, message, category, retryable, context, suggestion
- Errors are structured for AI reasoning (what failed, why, what to do)
- Include "machine_message" separate from human-readable text
- **Pros**: AI can reason about errors programmatically
- **Cons**: Complexity in error construction

#### Approach D: Full Command Audit (Recommended)

Combine A, B, C with systematic review of each command:
- Standardize JSON output schema per command
- Rationalize exit codes to consistent semantics
- Enhance error protocol for AI reasoning
- Add observability (structured logging, tracing)
- Define versioning strategy for API stability

### Similar solutions

- **kubectl**: JSON/YAML output, consistent exit codes, machine-readable status
- **gh (GitHub CLI)**: Structured JSON output, clear error messages, retry guidance
- **aws cli**: `--output json`, consistent error structure, exit code semantics

## Constraints

### Technical

- **Major refactor OK** - Can redesign commands from scratch if needed for AI-first design
- No hard backward compatibility requirement
- Must remain pure Rust, no external runtime dependencies
- Must work on macOS and Linux

### Business

- AI adoption is the priority - optimize for Claude Code integration
- Performance matters but correctness matters more
- Timeline: Not specified, but blocking AI adoption is urgent

### User experience

- AI agents are primary users - optimize for machine parsing
- Human-readable output remains valuable for debugging
- `--json` flag pattern is acceptable for dual-mode output

## Risks & Unknowns

### Known risks

1. **Concurrency edge cases** - Multiple AI agents sharing daemon, race conditions in session management
2. **Platform differences** - macOS vs Linux behavior in PTY, signals, sockets
3. **Performance under load** - Many concurrent sessions, rapid command sequences, memory pressure
4. **Resource leaks** - Sessions not cleaned up, file handles left open, zombie processes
5. **Signal handling** - Improper SIGTERM/SIGKILL handling, orphaned child processes

### Open questions

- [ ] What JSON schema version/format should we use? (JSON Schema draft-07?)
- [ ] Should we version the API explicitly (v1, v2)?
- [ ] How do we handle long-running commands (attach, wait) in terms of output?
- [ ] What observability format? (OpenTelemetry? Custom structured logs?)
- [ ] How do we test AI integration systematically?
- [ ] What's the migration path for existing scripts?

### Needs research/prototyping

1. **AI integration proof-of-concept** - Demo Claude Code using commands without special handling
2. **Concurrency stress testing** - Multiple agents hammering daemon simultaneously
3. **Platform testing matrix** - Verify behavior on macOS Intel/ARM, Linux x86/ARM
4. **Exit code audit** - Document current exit codes vs desired semantics

## Success Criteria

### Must-haves

- [ ] Passing test suite validating production-ready behavior for each command
- [ ] AI integration proof - Demo showing Claude Code reliably using commands without special handling
- [ ] Consistent JSON output schema across all commands
- [ ] Rationalized exit codes with documented semantics
- [ ] Enhanced error protocol with retryable/category/suggestion fields

### Nice-to-haves

- [ ] Documented standards (formal spec of exit codes, JSON schemas, error categories)
- [ ] Observability - structured logging and tracing
- [ ] API versioning strategy
- [ ] Performance benchmarks under load

### Metrics

- **Primary**: Claude Code can complete a 10-step TUI automation without manual intervention
- **Secondary**: All commands return valid JSON when `--json` flag is used
- **Secondary**: All commands return documented exit codes
- **Secondary**: Zero race conditions under concurrent load testing

## Raw Ideas

### From codebase exploration

**Current State Findings**:

1. **Exit Codes** - Already using sysexits.h conventions (64-78 range) plus LSB code 3 for daemon status. Good foundation but inconsistently applied.

2. **Error Protocol** - `ClientError` enum has categories but not all errors use them. `RpcError` includes suggestion field but not always populated.

3. **JSON Output** - `--json` flag exists but output structure varies by command. No formal schema.

4. **Threading** - Thread pool with 4-8 workers, Arc+Mutex for shared state. Need to audit for race conditions.

5. **Timeouts** - Multiple timeout layers (read/write/idle) but not all configurable. Wait command has timeout but exit code semantics unclear.

6. **Signal Handling** - SIGTERM/SIGKILL for daemon stop, but attach mode signal handling is basic.

### AI-specific observations

1. **Output parsing challenge**: AI needs to know "did this succeed?" before parsing details. Current output mixes success/failure indicators.

2. **Retry semantics**: AI needs to know "should I retry this?" Current errors don't always indicate retryability clearly.

3. **State inference**: After running a command, AI needs to know "what is the current state?" Commands don't always report resulting state.

4. **Action chaining**: AI often needs to run sequences. Commands don't suggest "next action" clearly.

### Observability needs

1. **Structured logging** - Commands should log structured events (JSON) for debugging
2. **Request tracing** - Correlate CLI invocation to daemon RPC to PTY operation
3. **Metrics** - Command latency, success rate, error distribution

### Versioning considerations

1. **API versioning** - JSON output schema should be versioned
2. **Protocol versioning** - JSON-RPC methods should be versioned
3. **CLI versioning** - Already have version command and mismatch warnings

## Codebase Context

### Command Architecture Summary

| Area | Current State | Production Readiness |
|------|---------------|---------------------|
| **CLI Framework** | Clap with global options | ✅ Good |
| **Exit Codes** | sysexits.h + LSB | ⚠️ Inconsistent |
| **Error Handling** | Typed ClientError | ⚠️ Incomplete |
| **JSON Output** | `--json` flag | ⚠️ No schema |
| **Threading** | Thread pool, Arc+Mutex | ⚠️ Needs audit |
| **Timeouts** | Multiple layers | ⚠️ Needs review |
| **Signal Handling** | Basic SIGTERM/SIGKILL | ⚠️ Needs audit |
| **Testing** | Unit + integration | ⚠️ Needs expansion |

### Key Files for Audit

- `crates/agent-tui/src/commands.rs` - CLI definitions
- `crates/agent-tui/src/handlers.rs` - Command execution
- `crates/agent-tui/src/app.rs` - Exit code handling
- `crates/agent-tui-ipc/src/error.rs` - Error types
- `crates/agent-tui-ipc/src/client.rs` - IPC client
- `crates/agent-tui-daemon/src/server.rs` - Daemon server
- `crates/agent-tui-common/src/error_codes.rs` - Error categories

---

## Next Steps

Run `/spec command-dx-audit` to:
1. Resolve open questions
2. Define JSON schemas for each command
3. Specify exit code semantics
4. Create formal specification for implementation
