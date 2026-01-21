#!/usr/bin/env bash
# Strip non-doc comments from Rust source files
# Keeps: /// and //! (doc comments)
# Removes: // (regular comments)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_DIR="$(dirname "$SCRIPT_DIR")"

# Check if ast-grep is installed
if ! command -v sg &> /dev/null; then
    echo "Error: ast-grep (sg) not installed"
    echo "Install with: cargo install-dev-tools"
    exit 1
fi

# Create temp rule file
RULE_FILE=$(mktemp)
cat > "$RULE_FILE" << 'EOF'
id: remove-non-doc-comments
language: rust
rule:
  kind: line_comment
  not:
    regex: "^//[/!]"
fix: ""
EOF

echo "Stripping non-doc comments from $CLI_DIR/src..."
sg scan --rule "$RULE_FILE" --update-all "$CLI_DIR/src/"

rm "$RULE_FILE"

echo "Running cargo fmt..."
cd "$CLI_DIR" && cargo fmt

echo "Done. Run 'cargo clippy && cargo test' to verify."
