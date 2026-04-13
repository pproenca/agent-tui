# OpenAI Codex Rust Timing Probe Follow-up

## Scope

This follow-up audit probes the remaining `park_timeout` / `elapsed` usage under sibling `*_tests.rs` files after the main `codex-rs` remediation program closed all formal findings in `/Users/pedroproenca/Documents/Projects/agent-tui`.

Inventory command used:

`rg -n 'park_timeout|Instant::elapsed\\(|\\.elapsed\\(' cli/crates -g '*_tests.rs'`

## Deterministic Placeholder Waits Removed

These tests were still using short sleeps only as thread handshakes or incidental ordering helpers. They were rewritten to use direct synchronization or explicit state injection:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty_tests.rs`
  `join_reader_thread_waits_for_completion` now coordinates the blocked reader and the join helper with channels instead of a fixed `25ms` wait.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach_tests.rs`
  `attach_output_worker_shutdown_aborts_before_joining` now uses an explicit shutdown channel instead of polling an atomic flag every `10ms`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty_tests.rs`
  `reader_channel_is_unbounded_for_output_events` now waits for the reader thread via a join-result channel instead of looping on `is_finished()` plus `park_timeout`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client_tests.rs`
  `test_call_with_config_times_out_over_real_socket` now holds the server stream open with a release channel instead of sleeping for `200ms`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session_tests.rs`
  `test_resolve_without_active_falls_back_to_most_recent_running_session` and `test_kill_promotes_most_recent_remaining_running_session_to_active` now inject explicit `created_at` ordering rather than sleeping between spawns.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process_tests.rs`
  `test_check_process_treats_zombie_as_not_found` no longer burns an up-front fixed `300ms` sleep before polling for zombie state.

## Remaining OS-Bound Integration Probes

The surviving timing-based tests are not unresolved `codex-rs` findings. They are waiting for real external state changes that are owned by the kernel, filesystem, or the lock-backoff algorithm itself.

### `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process_tests.rs`

- `test_check_process_treats_zombie_as_not_found`
  Polls `ps` until the child is reported as zombie (`Z`). This is a real kernel-observable process-state transition, not a local async timer.

### `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/lock_helpers_tests.rs`

- `test_acquire_lock_with_simple_mutex`
- `test_lock_timeout_with_held_mutex`
- `test_acquire_session_lock_succeeds_after_contention`
- `test_acquire_session_lock_timeout_returns_none_under_contention`

These tests intentionally exercise the actual lock backoff and timeout behavior implemented in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/lock_helpers.rs`. Replacing them with fake virtual time would stop testing the real retry windows and jitter/backoff interaction that the helper is responsible for.

### `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session_tests.rs`

- `wait_for_file_contents`
  Polls until the JSONL session store becomes visible on disk. This is a filesystem visibility probe used by persistence-focused tests.
- `test_startup_cleanup_kills_persisted_session_process_group`
- `test_cleanup_stale_sessions_appends_remove_events_when_unknown_records_exist`

These tests poll for real process termination after cleanup logic targets a live process group. The wait is for the OS to expose the exit, not for an in-process timer to fire.

## Conclusion

The post-remediation timing inventory is now narrow and intentional:

- deterministic placeholder waits were removed;
- the remaining real-time polling is confined to OS-bound integration probes or direct lock-backoff behavior;
- no new open findings were created from this follow-up pass.
