#!/bin/bash
#
# Install git hooks from scripts/hooks/ to .git/hooks/
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOKS_DIR="$SCRIPT_DIR/hooks"
GIT_HOOKS_DIR="$SCRIPT_DIR/../.git/hooks"

if [ ! -d "$GIT_HOOKS_DIR" ]; then
    echo "ERROR: .git/hooks directory not found. Are you in a git repository?"
    exit 1
fi

echo "Installing git hooks..."

for hook in "$HOOKS_DIR"/*; do
    if [ -f "$hook" ]; then
        hook_name=$(basename "$hook")
        target="$GIT_HOOKS_DIR/$hook_name"
        cp "$hook" "$target"
        chmod +x "$target"
        echo "  Installed: $hook_name"
    fi
done

echo "Git hooks installed successfully!"
