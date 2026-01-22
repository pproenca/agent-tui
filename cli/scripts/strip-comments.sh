#!/usr/bin/env bash
#
# Strip all comments from Rust source files using ast-grep.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
CLI_DIR="$(dirname "$SCRIPT_DIR")"
readonly CLI_DIR

if ! command -v sg &>/dev/null; then
  echo "Error: ast-grep (sg) not installed" >&2
  echo "Install with: brew install ast-grep" >&2
  exit 1
fi

if [[ ! -d "$CLI_DIR/src" ]]; then
  echo "Error: Source directory not found: $CLI_DIR/src" >&2
  exit 1
fi

RULE=$(mktemp)
trap 'rm -f "$RULE"' EXIT

cat >"$RULE" <<'EOF'
id: remove-all-comments
language: rust
rule:
  kind: line_comment
fix: ""
EOF

echo "Stripping ALL comments from $CLI_DIR/src..."
find "$CLI_DIR/src" -name "*.rs" -print0 | xargs -0 sg scan --rule "$RULE" --update-all

echo "Running cargo fmt..."
cargo fmt --manifest-path "$CLI_DIR/Cargo.toml"

echo "Done. Run 'cargo clippy && cargo test' to verify."
