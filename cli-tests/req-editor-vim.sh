#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

usage() {
  cat <<'EOF'
Usage: cli-tests/req-editor-vim.sh [--skip-build] [--keep]

Editor scenario:
1. build agent-tui (unless AGENT_TUI_BIN or --skip-build is used)
2. start an isolated daemon
3. open vim and enter multiple lines
4. resize the terminal and assert content survives

Examples:
  cli-tests/req-editor-vim.sh
  cli-tests/req-editor-vim.sh --skip-build
  cli-tests/req-editor-vim.sh --keep
EOF
}

scenario_bootstrap "editor-vim" usage "$@"

require_command vim

log_step "Starting isolated daemon"
"$BIN" daemon run >"$ARTIFACT_DIR/daemon.stdout.log" 2>"$ARTIFACT_DIR/daemon.stderr.log" &
DAEMON_PID=$!
for attempt in $(seq 1 100); do
  if "$BIN" --json sessions >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
  if [[ "$attempt" -eq 100 ]]; then
    echo "Timed out waiting for daemon readiness" >&2
    exit 1
  fi
done

log_step "Editor step: write text in vim"
vim_payload="$("$BIN" --json run --cols 120 --rows 40 -- vim -Nu NONE -n)"
vim_session_id="$(json_field "$vim_payload" session_id)"
if [[ -z "$vim_session_id" ]]; then
  echo "Failed to start vim session" >&2
  printf '%s\n' "$vim_payload" >&2
  exit 1
fi

"$BIN" --session "$vim_session_id" wait --stable -t 5000 >/dev/null

"$BIN" --session "$vim_session_id" type "ialpha line" >/dev/null
"$BIN" --session "$vim_session_id" press Enter >/dev/null
"$BIN" --session "$vim_session_id" type "beta line" >/dev/null
"$BIN" --session "$vim_session_id" press Esc >/dev/null

"$BIN" --session "$vim_session_id" wait --assert "alpha line" -t 5000 >/dev/null
"$BIN" --session "$vim_session_id" wait --assert "beta line" -t 5000 >/dev/null
"$BIN" --session "$vim_session_id" screenshot --strip-ansi >"$ARTIFACT_DIR/vim-before-resize.txt"

log_step "Editor step: resize and verify content remains"
"$BIN" --session "$vim_session_id" resize --cols 100 --rows 20 >/dev/null
"$BIN" --session "$vim_session_id" wait --stable -t 5000 >/dev/null
"$BIN" --session "$vim_session_id" screenshot --strip-ansi >"$ARTIFACT_DIR/vim-after-resize.txt"
assert_file_contains "$ARTIFACT_DIR/vim-after-resize.txt" "beta line"

"$BIN" --session "$vim_session_id" kill --yes >/dev/null 2>&1 || true

printf '\nEditor vim scenario passed.\n'
if [[ "$KEEP" -eq 1 ]]; then
  printf 'Artifacts: %s\n' "$ARTIFACT_DIR"
else
  printf 'Artifacts were temporary. Re-run with --keep to retain them.\n'
fi
