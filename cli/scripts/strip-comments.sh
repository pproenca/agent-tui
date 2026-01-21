#!/usr/bin/env bash
# Strip comments from Rust source files
# Keeps: /// doc comments in commands.rs (used for --help text)
# Removes: ALL other comments

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_DIR="$(dirname "$SCRIPT_DIR")"

# Check if ast-grep is installed
if ! command -v sg &> /dev/null; then
    echo "Error: ast-grep (sg) not installed"
    echo "Install with: cargo install ast-grep"
    exit 1
fi

# Rule to remove ALL comments (including doc comments)
RULE_ALL=$(mktemp)
cat > "$RULE_ALL" << 'EOF'
id: remove-all-comments
language: rust
rule:
  kind: line_comment
fix: ""
EOF

# Rule to remove only regular comments (keep doc comments)
RULE_REGULAR=$(mktemp)
cat > "$RULE_REGULAR" << 'EOF'
id: remove-regular-comments
language: rust
rule:
  kind: line_comment
  not:
    regex: "^//[/!]"
fix: ""
EOF

echo "Stripping ALL comments from $CLI_DIR/src (except commands.rs)..."
find "$CLI_DIR/src" -name "*.rs" ! -name "commands.rs" -exec sg scan --rule "$RULE_ALL" --update-all {} +

echo "Stripping regular comments from commands.rs (keeping doc comments for --help)..."
sg scan --rule "$RULE_REGULAR" --update-all "$CLI_DIR/src/commands.rs"

rm "$RULE_ALL" "$RULE_REGULAR"

echo "Running cargo fmt..."
cd "$CLI_DIR" && cargo fmt

echo "Done. Run 'cargo clippy && cargo test' to verify."
