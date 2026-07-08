#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

files=(
  "README.md"
  "skills/agent-tui/SKILL.md"
  "skills/agent-tui/agents/openai.yaml"
  "skills/agent-tui/references/command-atlas.md"
  "skills/agent-tui/references/output-contract.md"
  "skills/agent-tui/references/use-cases.md"
  "skills/agent-tui/references/flows.md"
  "skills/agent-tui/references/session-lifecycle.md"
  "skills/agent-tui/references/clarifications.md"
  "skills/agent-tui/references/recovery.md"
  "skills/agent-tui/references/decision-tree.md"
  "skills/agent-tui/references/demo.md"
  "skills/agent-tui/references/test-plan.md"
)

invalid_patterns=(
  'agent-tui[[:space:]]+scroll'
  '--verbose'
)

failed=0
check_absent() {
  local file="$1"
  local pattern="$2"
  local message="$3"
  local full_path="$ROOT/$file"
  if grep -nE "$pattern" "$full_path" >/dev/null 2>&1; then
    echo "$message"
    grep -nE "$pattern" "$full_path" || true
    failed=1
  fi
}

check_present() {
  local file="$1"
  local pattern="$2"
  local message="$3"
  local full_path="$ROOT/$file"
  if ! grep -nE "$pattern" "$full_path" >/dev/null 2>&1; then
    echo "$message"
    failed=1
  fi
}

for file in "${files[@]}"; do
  full_path="$ROOT/$file"
  for pattern in "${invalid_patterns[@]}"; do
    if grep -nE "$pattern" "$full_path" >/dev/null 2>&1; then
      echo "Invalid command reference found in $file (pattern: $pattern)"
      grep -nE "$pattern" "$full_path" || true
      failed=1
    fi
  done
done

check_absent "skills/agent-tui/SKILL.md" '`agent-tui kill`' \
  "Skill quick-start cleanup must use \`agent-tui kill --yes\` for automation-safe cleanup."
check_absent "skills/agent-tui/agents/openai.yaml" 'cleaning up with kill\.' \
  "Agent prompt must use kill --yes for automation-safe cleanup."
check_absent "skills/agent-tui/references/flows.md" '`agent-tui --session <id> kill`|`agent-tui kill`|Ctrl-P Ctrl-Q' \
  "Flow references must use kill --yes and the current Ctrl-P Ctrl-B detach sequence."
check_absent "skills/agent-tui/references/demo.md" '`agent-tui kill`' \
  "Demo cleanup must use \`agent-tui kill --yes\` for automation-safe cleanup."
check_absent "skills/agent-tui/references/use-cases.md" '-> `kill`\.' \
  "Use-case references must use kill --yes for automation-safe cleanup."
check_absent "skills/agent-tui/references/session-lifecycle.md" 'Ctrl-P Ctrl-Q' \
  "Session lifecycle docs must use the current Ctrl-P Ctrl-B detach sequence."
check_absent "README.md" 'Ctrl-P Ctrl-Q' \
  "README must use the current Ctrl-P Ctrl-B detach sequence."
check_present "skills/agent-tui/references/command-atlas.md" 'reserved/currently rejected|currently rejected' \
  "Command atlas must document screenshot --region as reserved/currently rejected."
check_present "skills/agent-tui/references/command-atlas.md" 'exit code 75' \
  "Command atlas must document wait --assert timeout exit code 75."
check_present "skills/agent-tui/references/output-contract.md" 'only present when `--include-cursor`|conditional on `--include-cursor`' \
  "Output contract must mark screenshot cursor JSON as conditional on --include-cursor."
check_present "skills/agent-tui/references/clarifications.md" 'daemon stop --yes' \
  "Clarifications must explain daemon-backed live preview needs daemon stop --yes to stop the daemon."

if [[ "$failed" -ne 0 ]]; then
  echo "Skill docs are out of sync with the current CLI surface."
  exit 1
fi

echo "Skill docs command references are in sync."
