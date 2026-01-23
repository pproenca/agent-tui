#!/bin/bash
# release.sh - Update versions, commit, and tag for release
#
# Usage: ./scripts/release.sh <version|bump-type>
# Examples:
#   ./scripts/release.sh 0.2.0    # explicit version
#   ./scripts/release.sh patch    # 0.2.0 → 0.2.1
#   ./scripts/release.sh minor    # 0.2.0 → 0.3.0
#   ./scripts/release.sh major    # 0.2.0 → 1.0.0

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

get_current_version() {
  local -r root_dir="$1"
  grep '^version = ' "${root_dir}/Cargo.toml" | head -1 | sed 's/version = "\([^"]*\)"/\1/'
}

bump_version() {
  local -r current="$1"
  local -r bump_type="$2"

  local major minor patch
  IFS='.' read -r major minor patch <<< "${current%%-*}"

  case "${bump_type}" in
    major)
      echo "$((major + 1)).0.0"
      ;;
    minor)
      echo "${major}.$((minor + 1)).0"
      ;;
    patch)
      echo "${major}.${minor}.$((patch + 1))"
      ;;
  esac
}

main() {
  if [[ -z "${1:-}" ]]; then
    err "Usage: $0 <version|bump-type>"
    err "Examples:"
    err "  $0 0.2.0    # explicit version"
    err "  $0 patch    # bump patch version"
    err "  $0 minor    # bump minor version"
    err "  $0 major    # bump major version"
    exit 1
  fi

  local -r script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  local -r root_dir="$(cd "${script_dir}/.." && pwd)"
  local -r arg="$1"
  local version

  if [[ "${arg}" =~ ^(major|minor|patch)$ ]]; then
    local -r current_version="$(get_current_version "${root_dir}")"
    version="$(bump_version "${current_version}" "${arg}")"
    echo "Bumping ${arg}: ${current_version} → ${version}"
  else
    version="${arg}"
    if ! [[ "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
      err "Error: Invalid version format. Use semver (e.g., 0.2.0 or 0.2.0-beta.1)"
      exit 1
    fi
  fi

  local -r tag="v${version}"

  if ! git diff --quiet || ! git diff --cached --quiet; then
    err "Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
  fi

  if git rev-parse "${tag}" >/dev/null 2>&1; then
    err "Error: Tag ${tag} already exists"
    exit 1
  fi

  echo "Releasing version ${version}..."

  echo "Updating package.json..."
  sed_inplace "s/\"version\": \"[^\"]*\"/\"version\": \"${version}\"/" "${root_dir}/package.json"

  echo "Updating Cargo.toml (workspace)..."
  sed_inplace "s/^version = \"[^\"]*\"/version = \"${version}\"/" "${root_dir}/Cargo.toml"

  echo
  echo "Version updates:"
  grep '"version"' "${root_dir}/package.json" | head -1
  grep '^version' "${root_dir}/Cargo.toml" | head -1

  echo
  echo "Staging changes..."
  git add "${root_dir}/package.json" "${root_dir}/Cargo.toml"

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
