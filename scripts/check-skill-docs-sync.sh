#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

files=(
  "skills/agent-tui/SKILL.md"
  "skills/agent-tui/references/command-atlas.md"
  "skills/agent-tui/references/output-contract.md"
  "skills/agent-tui/references/use-cases.md"
  "skills/agent-tui/references/flows.md"
  "skills/agent-tui/references/recovery.md"
  "skills/agent-tui/references/decision-tree.md"
  "skills/agent-tui/references/test-plan.md"
)

invalid_patterns=(
  'agent-tui[[:space:]]+scroll'
  '--verbose'
)

failed=0
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

if [[ "$failed" -ne 0 ]]; then
  echo "Skill docs are out of sync with the current CLI surface."
  exit 1
fi

echo "Skill docs command references are in sync."
