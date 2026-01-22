#!/bin/bash
# release.sh - Update versions, commit, and tag for release
#
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0

set -euo pipefail

sed_inplace() {
  if [[ "$OSTYPE" == darwin* ]]; then
    sed -i '' "$@"
  else
    sed -i "$@"
  fi
}

if [[ -z "$1" ]]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.2.0"
  exit 1
fi

VERSION="$1"
TAG="v${VERSION}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Error: Invalid version format. Use semver (e.g., 0.2.0 or 0.2.0-beta.1)"
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: You have uncommitted changes. Please commit or stash them first."
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: Tag $TAG already exists"
  exit 1
fi

echo "Releasing version $VERSION..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Updating package.json..."
sed_inplace "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$ROOT_DIR/package.json"

echo "Updating cli/Cargo.toml..."
sed_inplace "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" "$ROOT_DIR/cli/Cargo.toml"

echo ""
echo "Version updates:"
grep '"version"' "$ROOT_DIR/package.json" | head -1
grep '^version' "$ROOT_DIR/cli/Cargo.toml" | head -1

echo ""
echo "Staging changes..."
git add "$ROOT_DIR/package.json" "$ROOT_DIR/cli/Cargo.toml"

echo "Committing..."
git commit -m "chore: bump version to $VERSION"

echo "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $VERSION"

echo ""
echo "Done! Release $VERSION prepared."
echo ""
echo "To publish, run:"
echo "  git push && git push --tags"
