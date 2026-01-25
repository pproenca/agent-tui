#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-.}"
SRC_DIR="$ROOT_DIR/crates/agent-tui/src"

fail() {
  echo "Architecture check failed: $1" >&2
  exit 1
}

rg -n "crate::daemon::|crate::ipc::|crate::terminal::|crate::core::|crate::commands::|crate::handlers::|crate::presenter::|crate::error::|crate::attach::" "$SRC_DIR" \
  && fail "legacy shim paths detected"

rg -n "std::process::exit" "$SRC_DIR" \
  | rg -v "/main\\.rs:" \
  && fail "std::process::exit is only allowed in main.rs"

rg -n "serde_json" "$SRC_DIR/domain" "$SRC_DIR/usecases" \
  && fail "serde_json usage detected in domain/usecases"

rg -n --pcre2 "crate::infra::(?!daemon::test_support)" "$SRC_DIR/usecases" --glob '!**/ports/errors.rs' \
  && fail "infra dependency detected in usecases"

echo "Architecture checks passed."
