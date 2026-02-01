# Dockerfile for E2E Tests - Brainstorm

> Generated: 2026-01-24
> Status: brainstorm
> Next: `/spec dockerfile-e2e-tests` to formalize

## Problem Space

### What problem are we solving?
- Need a containerized environment to run E2E tests reproducibly
- Current tests run natively on host OS, which can vary across developer machines
- Want to test against real TUI applications (not just bash)
- Claude Code tests are optional/CI-only - need a publicly available TUI alternative

### Who experiences this problem?
- Developers running tests locally with different OS configurations
- CI pipelines that need consistent test environments
- Contributors who want to validate changes against real TUI apps

### Impact of not solving
- Test flakiness due to environment differences
- Limited TUI coverage (bash-only without Claude)
- No portable way to run full E2E suite

### Current workarounds
- Tests run natively on ubuntu-latest/macos-latest in CI
- Claude Code E2E tests are skipped if `claude` binary not available locally
- Developers rely on mock tests for most validation

## Solution Space

### Ideal end state
- Single `docker run` command to execute full E2E suite
- Tests against interesting, real-world TUI applications
- Reproducible across all developer machines and CI

### Possible approaches

#### Approach A: htop
- Description: Interactive process viewer, widely available
- Pros:
  - Available in every Linux distro
  - Has recognizable UI components (meters, process list, F-key buttons)
  - Simple to install (`apt install htop`)
  - No configuration needed
- Cons:
  - Static output - limited interaction possibilities
  - Components may vary by version/distro

#### Approach B: vim/neovim
- Description: Classic text editor with rich TUI
- Pros:
  - Complex TUI with many component types
  - Highly interactive (modes, buffers, splits)
  - Tests real-world editing workflows
  - Neovim has better defaults
- Cons:
  - Steeper learning curve for test assertions
  - Need to handle modal editing in tests
  - Plugin ecosystem could complicate reproducibility

#### Approach C: lazygit
- Description: Terminal UI for git commands
- Pros:
  - Modern, attractive TUI with clear components
  - Rich interaction (panels, popups, confirmations)
  - Relevant to developer workflows
  - Good VOM test coverage (buttons, tabs, panels)
- Cons:
  - Requires git repo setup for meaningful tests
  - May need repo initialization per test

#### Approach D: tig
- Description: Text-mode interface for git
- Pros:
  - Simpler than lazygit
  - Stable, predictable UI
  - Standard in many distros
- Cons:
  - Less visually interesting
  - Fewer component types to test

#### Approach E: midnight commander (mc)
- Description: File manager with two-panel interface
- Pros:
  - Rich dual-panel interface
  - Menus, dialogs, function key bar
  - Long history, stable
  - Excellent for testing panels/inputs
- Cons:
  - Slightly dated look
  - Complex to navigate programmatically

#### Approach F: Multiple TUIs (comprehensive suite)
- Description: Test against several TUIs for broader coverage
- Pros:
  - Better VOM coverage across component types
  - More realistic test scenarios
  - Find edge cases across different rendering approaches
- Cons:
  - More complex Dockerfile
  - Longer test execution time
  - More maintenance burden

### Similar solutions
- termenv/lipgloss test containers
- charm-bracelet TUI testing approaches
- tmux-based test harnesses

## Constraints

### Technical
- Must have PTY access (docker --tty, /dev/pts)
- Unix socket communication within container
- Rust compilation may be slow in container
- Need to handle terminal size/TERM environment

### Business
- Should be quick to set up and run
- Low maintenance overhead
- Works in CI (GitHub Actions Docker support)

### User experience
- Single command to run tests
- Clear output and error reporting
- Easy to add new TUI tests

## Risks & Unknowns

### Known risks
- Docker + PTY interaction can be tricky
- VOM component classification may differ in container
- Test timing/stability with real TUI startup

### Open questions
- [ ] Which TUI provides best component variety for VOM testing?
- [ ] Should we pre-compile the binary or compile in container?
- [ ] How to handle TUI startup delays/readiness detection?
- [ ] Should we use alpine or debian base image?
- [ ] How to structure test scripts for different TUIs?

### Needs research/prototyping
- htop/lazygit/vim component classification with current VOM
- PTY behavior in Docker containers
- Test isolation within container

## Success Criteria

### Must-haves
- [ ] Dockerfile builds successfully
- [ ] Container runs agent-tui daemon
- [ ] At least one TUI test passes (spawn, snapshot, component classification)
- [ ] Works on developer machines and CI

### Nice-to-haves
- [ ] Multiple TUI support
- [ ] Parallelized test execution
- [ ] Cached builds for faster iteration
- [ ] Integration with existing test infrastructure

### Metrics
- Test execution time in container
- Component classification accuracy vs native
- Image size (smaller = better)

## Raw Ideas

1. Start with htop - it's the simplest and always available
2. Could use lazygit for more complex scenarios later
3. Two-stage Dockerfile: build stage (rust) + runtime stage (minimal)
4. Mount cargo cache for faster rebuilds during development
5. Consider using `just` recipes inside container
6. Could expose socket for debugging (docker run --volume)
7. Maybe ncurses-based test app (custom, simple, deterministic)?
8. vim with specific vimrc for predictable UI
9. Test harness could reuse RealTestHarness patterns

## Codebase Context

### Current Test Infrastructure
- `RealTestHarness` in `common/real_test_harness.rs` - spawns isolated daemon
- Tests use unique sockets: `/tmp/agent-tui-test-{UUID}.sock`
- Bash is primary test TUI currently
- VOM classifies components: Button, Tab, Input, StaticText, Panel, Checkbox, MenuItem

### Build System
- Workspace with multiple crates
- `cargo build --workspace --release` for full build
- `just ready` runs format-check, lint, test

### Relevant Commands
- `cargo run -p agent-tui -- daemon start --foreground`
- `cargo run -p agent-tui -- run htop` (spawn TUI)
- `cargo run -p agent-tui -- snap` (take snapshot)

---

## Next Steps

Run `/spec dockerfile-e2e-tests` to:
1. Resolve open questions
2. Choose TUI applications to test
3. Create formal specification with Dockerfile structure
