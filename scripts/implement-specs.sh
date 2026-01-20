#!/usr/bin/env bash
# Orchestrates spec implementation with judge loop
#
# Usage: ./scripts/implement-specs.sh [--spec <name>] [--reset] [--dry-run]
#
# Options:
#   --spec <name>   Process only the specified spec file
#   --reset         Reset progress state and start fresh
#   --dry-run       Show what would be processed without executing

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SPECS_DIR="$PROJECT_ROOT/docs/specs"
STATE_FILE="$PROJECT_ROOT/.claude/spec-progress.json"
LOG_DIR="$PROJECT_ROOT/.claude/logs"
LOG_FILE="$LOG_DIR/implement-specs.log"
MAX_ATTEMPTS=3
MAX_TURNS=50
JUDGE_MAX_TURNS=15

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Spec processing order (based on dependencies and missing features)
SPEC_ORDER=(
  "07-STATE_QUERY.spec.md"
  "03-ELEMENT_INTERACTION.spec.md"
  "05-KEYBOARD_MOUSE.spec.md"
  "08-FORM_ELEMENTS.spec.md"
  "09-SEMANTIC_LOCATORS.spec.md"
  "10-RECORDING.spec.md"
  "01-SESSION_LIFECYCLE.spec.md"
  "04-SNAPSHOT.spec.md"
  "06-WAITING.spec.md"
  "11-SESSION_MANAGEMENT.spec.md"
  # 02-NAVIGATION is skipped (N/A for TUI)
)

log_info() {
  echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
  echo -e "${GREEN}✅${NC} $1"
}

log_warning() {
  echo -e "${YELLOW}⚠️${NC} $1"
}

log_error() {
  echo -e "${RED}❌${NC} $1"
}

# Format stream-json output for human readability
# Extracts text content, tool uses, and results from the JSON stream
format_stream_json() {
  local show_tools="${1:-true}"

  while IFS= read -r line; do
    # Skip empty lines
    [[ -z "$line" ]] && continue

    # Try to parse as JSON
    local msg_type
    msg_type=$(echo "$line" | jq -r '.type // empty' 2>/dev/null) || continue

    case "$msg_type" in
      content_block_start)
        local block_type
        block_type=$(echo "$line" | jq -r '.content_block.type // empty' 2>/dev/null)
        if [[ "$block_type" == "tool_use" && "$show_tools" == "true" ]]; then
          local tool_name
          tool_name=$(echo "$line" | jq -r '.content_block.name // empty' 2>/dev/null)
          echo -e "${BLUE}▶ Tool: ${tool_name}${NC}"
        fi
        ;;
      content_block_delta)
        local delta_type
        delta_type=$(echo "$line" | jq -r '.delta.type // empty' 2>/dev/null)
        if [[ "$delta_type" == "text_delta" ]]; then
          local text
          text=$(echo "$line" | jq -r '.delta.text // empty' 2>/dev/null)
          # Print text without newline to allow streaming effect
          printf '%s' "$text"
        fi
        ;;
      message_stop)
        # Ensure we end with a newline after message completes
        echo ""
        ;;
      result)
        local is_error cost_usd duration_ms
        is_error=$(echo "$line" | jq -r '.is_error // false' 2>/dev/null)
        cost_usd=$(echo "$line" | jq -r '.cost_usd // 0' 2>/dev/null)
        duration_ms=$(echo "$line" | jq -r '.duration_ms // 0' 2>/dev/null)

        echo ""
        if [[ "$is_error" == "true" ]]; then
          local error_msg
          error_msg=$(echo "$line" | jq -r '.error // "Unknown error"' 2>/dev/null)
          echo -e "${RED}━━━ Error: ${error_msg} ━━━${NC}"
        else
          # Format cost and duration nicely
          local duration_sec
          duration_sec=$(awk "BEGIN {printf \"%.1f\", $duration_ms/1000}")
          echo -e "${GREEN}━━━ Completed in ${duration_sec}s | Cost: \$${cost_usd} ━━━${NC}"
        fi
        ;;
    esac
  done
}

# Log a section header to the unified log file
log_section() {
  local phase="$1"
  local spec_name="$2"
  local attempt="${3:-}"
  local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

  {
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    if [[ -n "$attempt" ]]; then
      echo "[$timestamp] $phase: $spec_name (attempt $attempt/$MAX_ATTEMPTS)"
    else
      echo "[$timestamp] $phase: $spec_name"
    fi
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
  } >> "$LOG_FILE"
}

# Initialize state file
init_state() {
  mkdir -p "$(dirname "$STATE_FILE")"
  mkdir -p "$LOG_DIR"

  if [[ ! -f "$STATE_FILE" ]]; then
    echo '{"completed":[],"in_progress":null,"attempts":{},"gaps":{}}' > "$STATE_FILE"
    log_info "Initialized state file: $STATE_FILE"
  fi

  # Initialize unified log file with session header
  local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
  {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║       agent-tui Spec Implementation Session                  ║"
    echo "║       Started: $timestamp                        ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
  } >> "$LOG_FILE"
  log_info "Logging to: $LOG_FILE"
}

# Reset state file
reset_state() {
  echo '{"completed":[],"in_progress":null,"attempts":{},"gaps":{}}' > "$STATE_FILE"
  log_info "Reset state file"
}

# Get state value using jq
get_state() {
  local key="$1"
  jq -r "$key" "$STATE_FILE" 2>/dev/null || echo ""
}

# Update state file
update_state() {
  local tmp_file=$(mktemp)
  jq "$1" "$STATE_FILE" > "$tmp_file" && mv "$tmp_file" "$STATE_FILE"
}

# Check if spec is completed
is_completed() {
  local spec_name="$1"
  local completed=$(get_state '.completed[]' 2>/dev/null)
  echo "$completed" | grep -q "^${spec_name}$" && return 0 || return 1
}

# Get attempt count for spec
get_attempts() {
  local spec_name="$1"
  local attempts=$(get_state ".attempts[\"$spec_name\"] // 0")
  echo "$attempts"
}

# Increment attempt count
increment_attempts() {
  local spec_name="$1"
  update_state ".attempts[\"$spec_name\"] = ((.attempts[\"$spec_name\"] // 0) + 1)"
}

# Mark spec as in progress
mark_in_progress() {
  local spec_name="$1"
  update_state ".in_progress = \"$spec_name\""
}

# Mark spec as complete
mark_complete() {
  local spec_name="$1"
  update_state ".completed += [\"$spec_name\"] | .in_progress = null | del(.gaps[\"$spec_name\"])"
}

# Save gaps for next iteration
save_gaps() {
  local spec_name="$1"
  local gaps="$2"
  # Escape the gaps for JSON
  local escaped_gaps=$(echo "$gaps" | jq -Rs '.')
  update_state ".gaps[\"$spec_name\"] = $escaped_gaps"
}

# Get next incomplete spec in order
get_next_spec() {
  for spec_name in "${SPEC_ORDER[@]}"; do
    if ! is_completed "$spec_name"; then
      local spec_path="$SPECS_DIR/$spec_name"
      if [[ -f "$spec_path" ]]; then
        echo "$spec_path"
        return
      fi
    fi
  done
}

# Build implementation prompt
build_implement_prompt() {
  local spec="$1"
  local spec_name=$(basename "$spec")
  local spec_contents=$(cat "$spec")
  local gaps=$(get_state ".gaps[\"$spec_name\"] // \"None\"")

  # Read template
  local template=$(cat "$SCRIPT_DIR/prompts/implement.md")

  # Substitute placeholders
  template="${template//\{\{SPEC_NAME\}\}/$spec_name}"

  # Use heredoc for multi-line content substitution
  echo "$template" | sed "s|{{SPEC_CONTENTS}}|$(echo "$spec_contents" | sed 's/|/\\|/g; s/&/\\&/g; s/$/\\n/' | tr -d '\n')|g" | \
    sed "s|{{GAPS}}|$(echo "$gaps" | sed 's/|/\\|/g; s/&/\\&/g')|g"
}

# Build judge prompt
build_judge_prompt() {
  local spec="$1"
  local spec_name=$(basename "$spec")
  local spec_contents=$(cat "$spec")

  # Extract relevant code snippets (if files exist)
  local commands_snippet=""
  local protocol_snippet=""
  local server_snippet=""

  if [[ -f "$PROJECT_ROOT/cli/src/commands.rs" ]]; then
    commands_snippet=$(cat "$PROJECT_ROOT/cli/src/commands.rs" | head -200)
  fi

  if [[ -f "$PROJECT_ROOT/cli/src/protocol.rs" ]]; then
    protocol_snippet=$(cat "$PROJECT_ROOT/cli/src/protocol.rs" | head -150)
  fi

  if [[ -f "$PROJECT_ROOT/cli/src/daemon/server.rs" ]]; then
    server_snippet=$(cat "$PROJECT_ROOT/cli/src/daemon/server.rs" | head -200)
  fi

  # Read template and output with substitutions
  cat "$SCRIPT_DIR/prompts/judge.md" | \
    sed "s|{{SPEC_NAME}}|$spec_name|g"

  echo ""
  echo "## Spec Requirements"
  echo '```markdown'
  echo "$spec_contents"
  echo '```'
  echo ""
  echo "## Current Implementation"
  echo "### commands.rs"
  echo '```rust'
  echo "$commands_snippet"
  echo '```'
  echo ""
  echo "### protocol.rs"
  echo '```rust'
  echo "$protocol_snippet"
  echo '```'
  echo ""
  echo "### server.rs"
  echo '```rust'
  echo "$server_snippet"
  echo '```'
}

# Implementation phase using claude
implement_spec() {
  local spec="$1"
  local attempt="$2"
  local spec_name=$(basename "$spec")

  log_info "Starting implementation phase for $spec_name"
  log_section "IMPLEMENT" "$spec_name" "$attempt"

  # Build the prompt
  local prompt_file=$(mktemp)
  cat "$SCRIPT_DIR/prompts/implement.md" > "$prompt_file"

  # Append spec contents
  echo "" >> "$prompt_file"
  echo "## Spec Contents" >> "$prompt_file"
  echo '```markdown' >> "$prompt_file"
  cat "$spec" >> "$prompt_file"
  echo '```' >> "$prompt_file"

  # Append gaps if any
  local gaps=$(get_state ".gaps[\"$spec_name\"]" | sed 's/^"//;s/"$//')
  if [[ "$gaps" != "null" && "$gaps" != "None" && -n "$gaps" ]]; then
    echo "" >> "$prompt_file"
    echo "## Previous Gaps to Address" >> "$prompt_file"
    echo "$gaps" >> "$prompt_file"
  fi

  # Run claude with the implementation prompt
  # Raw JSON goes to log file and output_file, formatted output goes to console
  cd "$PROJECT_ROOT"
  local output_file=$(mktemp)
  claude -p "$(cat "$prompt_file")" \
    --allowedTools "Edit,Write,Read,Bash,Glob,Grep" \
    --max-turns "$MAX_TURNS" \
    --output-format stream-json \
    --verbose \
    2>&1 | tee -a "$LOG_FILE" | tee "$output_file" | format_stream_json

  local exit_code=${PIPESTATUS[0]}
  rm -f "$prompt_file"

  # Check if implementation was successful (look for promise tag in raw JSON output)
  if grep -q "IMPLEMENTED" "$output_file"; then
    rm -f "$output_file"
    return 0
  else
    rm -f "$output_file"
    return 1
  fi
}

# Judge phase using claude
judge_spec() {
  local spec="$1"
  local attempt="$2"
  local spec_name=$(basename "$spec")

  log_info "Starting judge phase for $spec_name"
  log_section "JUDGE" "$spec_name" "$attempt"

  # Build judge prompt
  local prompt_file=$(mktemp)
  build_judge_prompt "$spec" > "$prompt_file"

  # Run claude with stream-json output format
  # Raw JSON goes to log file and output_file, formatted output goes to console
  cd "$PROJECT_ROOT"
  local output_file=$(mktemp)
  claude -p "$(cat "$prompt_file")" \
    --allowedTools "Read,Bash,Glob,Grep" \
    --max-turns "$JUDGE_MAX_TURNS" \
    --output-format stream-json \
    --verbose \
    2>&1 | tee -a "$LOG_FILE" | tee "$output_file" | format_stream_json

  rm -f "$prompt_file"

  # Extract all text content from stream-json format into a single file
  local text_file=$(mktemp)
  # Extract text from delta.text fields and concatenate
  jq -r 'select(.type == "content_block_delta") | select(.delta.type == "text_delta") | .delta.text // empty' "$output_file" 2>/dev/null | tr -d '\n' > "$text_file" || true

  # Also try the older format where text appears directly
  if [[ ! -s "$text_file" ]]; then
    grep -o '"text":"[^"]*"' "$output_file" 2>/dev/null | sed 's/"text":"//g; s/"$//g' | tr -d '\n' >> "$text_file" || true
  fi

  # Convert escaped newlines to actual newlines
  sed -i.bak 's/\\n/\n/g' "$text_file" 2>/dev/null || sed 's/\\n/\n/g' "$text_file" > "${text_file}.tmp" && mv "${text_file}.tmp" "$text_file"
  rm -f "${text_file}.bak"

  # Check for VERDICT: PASS
  if grep -q "VERDICT: PASS" "$text_file"; then
    rm -f "$output_file" "$text_file"
    echo "PASS"
  else
    # Extract gaps - look for lines starting with "- " after "GAPS:"
    local gaps
    gaps=$(awk '/GAPS:/{found=1; next} found && /^[^-]/{exit} found && /^- /{print}' "$text_file" || true)

    # If no gaps found with awk, try a simpler grep after GAPS:
    if [[ -z "$gaps" ]]; then
      gaps=$(sed -n '/GAPS:/,/VERDICT:/p' "$text_file" | grep -E '^- ' || true)
    fi

    # Check if judge hit max turns without producing a verdict
    local max_turns_error=""
    if grep -q '"subtype":"error_max_turns"' "$output_file"; then
      max_turns_error="yes"
    fi

    rm -f "$output_file" "$text_file"

    if [[ -n "$gaps" ]]; then
      echo "GAPS"
      echo "$gaps"
    elif [[ -n "$max_turns_error" ]]; then
      echo "GAPS"
      echo "- [ ] Judge hit max turns limit before producing verdict (increase JUDGE_MAX_TURNS)"
    else
      # Fallback: return generic gap if no specific gaps found
      echo "GAPS"
      echo "- [ ] Judge returned FAIL but no specific gaps provided"
    fi
  fi
}

# Process a single spec through implement/judge loop
process_spec() {
  local spec="$1"
  local spec_name=$(basename "$spec")

  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  log_info "Processing: $spec_name"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  local attempts=$(get_attempts "$spec_name")

  while [[ $attempts -lt $MAX_ATTEMPTS ]]; do
    increment_attempts "$spec_name"
    attempts=$((attempts + 1))

    log_info "Attempt $attempts of $MAX_ATTEMPTS"
    mark_in_progress "$spec_name"

    # Implementation phase
    if implement_spec "$spec" "$attempts"; then
      log_success "Implementation phase completed"
    else
      log_warning "Implementation phase did not emit IMPLEMENTED promise"
    fi

    # Judge phase
    local judge_result=$(judge_spec "$spec" "$attempts")

    if [[ "$judge_result" == "PASS" ]]; then
      mark_complete "$spec_name"
      log_success "$spec_name PASSED"
      return 0
    else
      save_gaps "$spec_name" "$judge_result"
      log_warning "$spec_name has gaps, will retry (attempt $attempts/$MAX_ATTEMPTS)"
      echo "$judge_result"
    fi
  done

  log_error "$spec_name failed after $MAX_ATTEMPTS attempts - needs human review"
  return 1
}

# Show current progress
show_progress() {
  echo ""
  echo "Current Progress:"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  for spec_name in "${SPEC_ORDER[@]}"; do
    local status="⬜ pending"
    if is_completed "$spec_name"; then
      status="${GREEN}✅ completed${NC}"
    elif [[ "$(get_state '.in_progress')" == "$spec_name" ]]; then
      status="${YELLOW}🔄 in progress${NC}"
    fi

    local attempts=$(get_attempts "$spec_name")
    if [[ $attempts -gt 0 ]]; then
      echo -e "  $spec_name: $status (attempts: $attempts)"
    else
      echo -e "  $spec_name: $status"
    fi
  done
  echo ""
}

# Main function
main() {
  local single_spec=""
  local reset=false
  local dry_run=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case $1 in
      --spec)
        single_spec="$2"
        shift 2
        ;;
      --reset)
        reset=true
        shift
        ;;
      --dry-run)
        dry_run=true
        shift
        ;;
      -h|--help)
        echo "Usage: $0 [--spec <name>] [--reset] [--dry-run]"
        echo ""
        echo "Options:"
        echo "  --spec <name>   Process only the specified spec file"
        echo "  --reset         Reset progress state and start fresh"
        echo "  --dry-run       Show what would be processed without executing"
        exit 0
        ;;
      *)
        log_error "Unknown option: $1"
        exit 1
        ;;
    esac
  done

  # Check dependencies
  if ! command -v claude &> /dev/null; then
    log_error "claude CLI not found. Please install claude-code."
    exit 1
  fi

  if ! command -v jq &> /dev/null; then
    log_error "jq not found. Please install jq."
    exit 1
  fi

  # Initialize or reset state
  init_state
  if [[ "$reset" == true ]]; then
    reset_state
  fi

  echo ""
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║          agent-tui Spec Implementation Loop                  ║"
  echo "╚══════════════════════════════════════════════════════════════╝"

  show_progress

  if [[ "$dry_run" == true ]]; then
    log_info "Dry run mode - no changes will be made"
    local next=$(get_next_spec)
    if [[ -n "$next" ]]; then
      log_info "Next spec to process: $(basename "$next")"
    else
      log_info "All specs completed!"
    fi
    exit 0
  fi

  # Process single spec if specified
  if [[ -n "$single_spec" ]]; then
    local spec_path="$SPECS_DIR/$single_spec"
    if [[ ! -f "$spec_path" ]]; then
      # Try with .spec.md suffix
      spec_path="$SPECS_DIR/${single_spec}.spec.md"
    fi
    if [[ ! -f "$spec_path" ]]; then
      log_error "Spec not found: $single_spec"
      exit 1
    fi
    process_spec "$spec_path"
    exit $?
  fi

  # Process all specs in order
  local failed_specs=()

  for spec_name in "${SPEC_ORDER[@]}"; do
    if is_completed "$spec_name"; then
      log_info "Skipping $spec_name (already completed)"
      continue
    fi

    local spec_path="$SPECS_DIR/$spec_name"
    if [[ ! -f "$spec_path" ]]; then
      log_warning "Spec not found: $spec_path"
      continue
    fi

    if ! process_spec "$spec_path"; then
      failed_specs+=("$spec_name")
    fi
  done

  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Final Results"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  show_progress

  if [[ ${#failed_specs[@]} -gt 0 ]]; then
    log_error "Failed specs requiring human review:"
    for spec in "${failed_specs[@]}"; do
      echo "  - $spec"
    done
    exit 1
  fi

  log_success "All specs implemented successfully!"

  # Final verification
  echo ""
  log_info "Running final verification..."
  cd "$PROJECT_ROOT/cli"

  if cargo clippy --all-targets --all-features -- -D warnings && cargo test; then
    log_success "Final verification passed!"
  else
    log_error "Final verification failed - please review manually"
    exit 1
  fi
}

main "$@"
