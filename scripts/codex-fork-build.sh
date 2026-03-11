#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || true)"

if [ -z "$REPO" ]; then
  echo "Failed to determine repository root from scripts/codex-fork-build.sh" >&2
  exit 1
fi

CODEX_RS_DIR="$REPO/codex-rs"
if [ ! -d "$CODEX_RS_DIR" ]; then
  echo "Expected codex-rs directory at: $CODEX_RS_DIR" >&2
  exit 1
fi

cd "$CODEX_RS_DIR"

# Build release binary for fast local runs.
cargo build -p codex-cli --release

# Record build hash to detect stale binaries.
BUILD_HASH_FILE="$REPO/codex-rs/target/release/.codex-build-hash"
HEAD_HASH="$(git -C "$REPO" rev-parse HEAD)"
status_porcelain="$(git -C "$REPO" status --porcelain)"

if [ -n "$status_porcelain" ]; then
  # Keep hash format in sync with callers that detect stale dirty builds.
  status_hash="$(printf '%s' "$status_porcelain" | sha256sum | awk '{print $1}')"
  HEAD_HASH="${HEAD_HASH}-dirty-${status_hash}"
fi

echo "$HEAD_HASH" > "$BUILD_HASH_FILE"

echo "Built: $REPO/codex-rs/target/release/codex"
echo "Build hash recorded: $BUILD_HASH_FILE"
