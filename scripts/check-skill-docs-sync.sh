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
  "docs/ops/release-channels.md"
)

invalid_patterns=(
  '--verbose'
)

current_guidance_files=(
  "README.md"
  "skills/agent-tui/agents/openai.yaml"
  "skills/agent-tui/references/use-cases.md"
  "skills/agent-tui/references/flows.md"
  "skills/agent-tui/references/test-plan.md"
)

legacy_current_patterns=(
  'agent-tui[[:space:]]+input'
  'agent-tui[[:space:]]+action'
  'agent-tui[[:space:]]+screenshot[^[:alnum:]\n]+-[ea]'
  'agent-tui[[:space:]]+wait[^[:alnum:]\n]+-e'
  'agent-tui[[:space:]]+scroll-into-view'
)

failed=0
check_absent() {
  local file="$1"
  local pattern="$2"
  local message="$3"
  local full_path="$ROOT/$file"
  if grep -nE -- "$pattern" "$full_path" >/dev/null 2>&1; then
    echo "$message"
    grep -nE -- "$pattern" "$full_path" || true
    failed=1
  fi
}

check_present() {
  local file="$1"
  local pattern="$2"
  local message="$3"
  local full_path="$ROOT/$file"
  if ! grep -nE -- "$pattern" "$full_path" >/dev/null 2>&1; then
    echo "$message"
    failed=1
  fi
}

for file in "${files[@]}"; do
  full_path="$ROOT/$file"
  for pattern in "${invalid_patterns[@]}"; do
    if grep -nE -- "$pattern" "$full_path" >/dev/null 2>&1; then
      echo "Invalid command reference found in $file (pattern: $pattern)"
      grep -nE -- "$pattern" "$full_path" || true
      failed=1
    fi
  done
done

for file in "${current_guidance_files[@]}"; do
  for pattern in "${legacy_current_patterns[@]}"; do
    check_absent "$file" "$pattern" \
      "Current guidance must not present legacy compatibility commands as the preferred path ($file, pattern: $pattern)."
  done
done

check_absent "skills/agent-tui/SKILL.md" '`agent-tui kill`' \
  "Skill quick-start cleanup must use \`agent-tui kill --yes\` for automation-safe cleanup."
check_absent "README.md" '^agent-tui kill$' \
  "README quick-start cleanup must use \`agent-tui kill --yes\` for automation-safe cleanup."
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
check_present "skills/agent-tui/SKILL.md" 'run -> screenshot -> press/type/scroll -> wait -> kill --yes' \
  "Skill must teach the current run -> screenshot -> press/type/scroll -> wait -> kill --yes loop first."
check_present "skills/agent-tui/SKILL.md" '## Legacy migration' \
  "Skill must include a legacy migration section."
for pattern in \
  'agent-tui input' \
  'agent-tui action' \
  'screenshot -e' \
  'screenshot -a' \
  'wait -e' \
  'scroll-into-view'
do
  check_present "skills/agent-tui/SKILL.md" "$pattern" \
    "Skill legacy migration must cover $pattern."
done
check_present "skills/agent-tui/SKILL.md" 'stderr.*next-major|next-major.*stderr|stderr.*next major|next major.*stderr' \
  "Skill must explain deprecation notices on stderr and the next-major removal window."
check_present "README.md" 'Release channel verification' \
  "README install docs must include release channel verification guidance."
for channel in github-releases install-script npm crates-io source-install homebrew; do
  check_present "README.md" "--channel $channel" \
    "README install docs must identify the $channel release channel and its verification command."
done
check_present "docs/ops/release-channels.md" '## Release notes guidance' \
  "Release channel docs must include release notes guidance."
check_present "docs/ops/release-channels.md" 'compatibility window' \
  "Release notes guidance must mention the compatibility window."
check_present "docs/ops/release-channels.md" 'next-major|next major' \
  "Release notes guidance must mention the next-major deprecation plan."

if [[ "$failed" -ne 0 ]]; then
  echo "Skill docs are out of sync with the current CLI surface."
  exit 1
fi

echo "Skill docs command references are in sync."
