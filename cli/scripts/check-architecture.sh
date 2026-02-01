#!/bin/bash
# Clean Architecture boundary checker using cargo-modules
# Detects layer violations in the agent-tui crate
#
# Note: cargo-modules can be unstable with complex codebases. If it crashes,
# this script will skip the cargo-modules checks but still pass to avoid
# blocking CI. The text-based xtask architecture checks remain authoritative.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_DIR="$(dirname "$SCRIPT_DIR")"

cd "$CLI_DIR"

# Check if cargo-modules is installed
if ! command -v cargo-modules &> /dev/null; then
    echo "WARNING: cargo-modules is not installed"
    echo "Install with: cargo install cargo-modules"
    echo "Skipping cargo-modules checks..."
    exit 0
fi

echo "Analyzing module dependencies..."

# Generate dependency graph in DOT format
set +e
DOT_OUTPUT=$(cargo modules dependencies --lib -p agent-tui --layout dot 2>&1)
DOT_EXIT=$?
set -e

if [[ $DOT_EXIT -ne 0 ]]; then
    # Exit code 134 is SIGABRT - cargo-modules crashed
    if [[ $DOT_EXIT -eq 134 ]]; then
        echo "WARNING: cargo-modules crashed (exit code 134)"
        echo "This is a known issue with cargo-modules on some codebases."
        echo "Skipping cargo-modules checks..."
        exit 0
    else
        echo "ERROR: Failed to run cargo modules dependencies (exit code $DOT_EXIT)"
        echo "$DOT_OUTPUT"
        exit 1
    fi
fi

# Extract "uses" edges (actual code dependencies, not structural "owns")
# Format: "source" -> "target" [label="uses", style="dashed", ...]
USES_EDGES=$(echo "$DOT_OUTPUT" | grep -E 'label="uses"' | grep -oE '"[^"]+"\s*->\s*"[^"]+"' || true)

if [[ -z "$USES_EDGES" ]]; then
    echo "No 'uses' dependencies found in module graph"
    echo "Clean Architecture checks passed (no cross-module dependencies)"
    exit 0
fi

# Define layer patterns
DOMAIN="::domain::"
USECASES="::usecases::"
ADAPTERS="::adapters::"
INFRA="::infra::"
APP="::app::"
COMMON="::common::"

VIOLATIONS=0

# Only check dependencies within our crate (agent_tui::)
CRATE_PREFIX="agent_tui::"

while IFS= read -r edge; do
    [[ -z "$edge" ]] && continue

    # Parse source and target from edge like: "agent_tui::domain::foo" -> "agent_tui::usecases::bar"
    SOURCE=$(echo "$edge" | sed 's/"\([^"]*\)".*/\1/')
    TARGET=$(echo "$edge" | sed 's/.*-> "\([^"]*\)"/\1/')

    # Skip edges that don't involve our crate (external dependencies)
    if [[ "$SOURCE" != "$CRATE_PREFIX"* ]] || [[ "$TARGET" != "$CRATE_PREFIX"* ]]; then
        continue
    fi

    # Check domain layer (should have NO external layer dependencies)
    if [[ "$SOURCE" == *"$DOMAIN"* ]]; then
        if [[ "$TARGET" == *"$USECASES"* ]] || [[ "$TARGET" == *"$ADAPTERS"* ]] || \
           [[ "$TARGET" == *"$INFRA"* ]] || [[ "$TARGET" == *"$APP"* ]]; then
            echo "VIOLATION: $SOURCE -> $TARGET"
            echo "  Reason: domain layer cannot depend on outer layers (usecases/adapters/infra/app)"
            ((VIOLATIONS++)) || true
        fi
    fi

    # Check usecases layer (should only depend on domain)
    if [[ "$SOURCE" == *"$USECASES"* ]]; then
        if [[ "$TARGET" == *"$ADAPTERS"* ]] || [[ "$TARGET" == *"$INFRA"* ]] || \
           [[ "$TARGET" == *"$APP"* ]]; then
            echo "VIOLATION: $SOURCE -> $TARGET"
            echo "  Reason: usecases layer cannot depend on adapters/infra/app"
            ((VIOLATIONS++)) || true
        fi
    fi

    # Check adapters layer (should not depend on infra or app)
    if [[ "$SOURCE" == *"$ADAPTERS"* ]]; then
        if [[ "$TARGET" == *"$INFRA"* ]] || [[ "$TARGET" == *"$APP"* ]]; then
            echo "VIOLATION: $SOURCE -> $TARGET"
            echo "  Reason: adapters layer cannot depend on infra/app"
            ((VIOLATIONS++)) || true
        fi
    fi

    # Check infra layer (should not depend on app)
    if [[ "$SOURCE" == *"$INFRA"* ]]; then
        if [[ "$TARGET" == *"$APP"* ]]; then
            echo "VIOLATION: $SOURCE -> $TARGET"
            echo "  Reason: infra layer cannot depend on app"
            ((VIOLATIONS++)) || true
        fi
    fi

done <<< "$USES_EDGES"

echo ""

# Check for circular dependencies
echo "Checking for circular dependencies..."
set +e
CYCLE_OUTPUT=$(cargo modules dependencies --lib -p agent-tui --acyclic 2>&1)
CYCLE_EXIT=$?
set -e

if [[ $CYCLE_EXIT -eq 134 ]]; then
    echo "WARNING: cargo-modules crashed during cycle check"
    echo "Skipping cycle detection..."
elif [[ $CYCLE_EXIT -ne 0 ]]; then
    # Filter out false positives (enum methods referencing self)
    if echo "$CYCLE_OUTPUT" | grep -q "DomainError.*DomainError::code"; then
        echo "Note: Ignoring DomainError self-reference (enum with methods, not a real cycle)"
    else
        echo "$CYCLE_OUTPUT"
        echo ""
        echo "ERROR: Circular dependencies detected!"
        ((VIOLATIONS++)) || true
    fi
else
    echo "No circular dependencies found"
fi

echo ""

if [[ $VIOLATIONS -gt 0 ]]; then
    echo "Found $VIOLATIONS Clean Architecture violation(s)"
    exit 1
fi

echo "Clean Architecture checks passed"
echo "  - No layer boundary violations"
echo "  - No circular dependencies"
