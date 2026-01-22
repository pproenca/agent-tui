#!/bin/bash
# release.sh - Update versions, commit, and tag for release
#
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0

set -euo pipefail

err() {
  echo "$*" >&2
}

sed_inplace() {
  if [[ "${OSTYPE}" == darwin* ]]; then
    sed -i '' "$@"
  else
    sed -i "$@"
  fi
}

main() {
  if [[ -z "${1:-}" ]]; then
    err "Usage: $0 <version>"
    err "Example: $0 0.2.0"
    exit 1
  fi

  local -r version="$1"
  local -r tag="v${version}"

  if ! [[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    err "Error: Invalid version format. Use semver (e.g., 0.2.0 or 0.2.0-beta.1)"
    exit 1
  fi

  if ! git diff --quiet || ! git diff --cached --quiet; then
    err "Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
  fi

  if git rev-parse "${tag}" >/dev/null 2>&1; then
    err "Error: Tag ${tag} already exists"
    exit 1
  fi

  echo "Releasing version ${version}..."

  local -r script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  local -r root_dir="$(cd "${script_dir}/.." && pwd)"

  echo "Updating package.json..."
  sed_inplace "s/\"version\": \"[^\"]*\"/\"version\": \"${version}\"/" "${root_dir}/package.json"

  echo "Updating cli/Cargo.toml..."
  sed_inplace "s/^version = \"[^\"]*\"/version = \"${version}\"/" "${root_dir}/cli/Cargo.toml"

  echo
  echo "Version updates:"
  grep '"version"' "${root_dir}/package.json" | head -1
  grep '^version' "${root_dir}/cli/Cargo.toml" | head -1

  echo
  echo "Staging changes..."
  git add "${root_dir}/package.json" "${root_dir}/cli/Cargo.toml"

  echo "Committing..."
  git commit -m "chore: bump version to ${version}"

  echo "Creating tag ${tag}..."
  git tag -a "${tag}" -m "Release ${version}"

  echo
  echo "Done! Release ${version} prepared."
  echo
  echo "To publish, run:"
  echo "  git push && git push --tags"
}

main "$@"
