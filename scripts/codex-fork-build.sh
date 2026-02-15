#!/usr/bin/env bash
set -euo pipefail

REPO="/home/neograiph/repo/AGENTS/codex-multi-agent"
cd "$REPO/codex-rs"

# Build release binary for fast local runs.
cargo build -p codex-cli --release

# Record build hash to detect stale binaries.
BUILD_HASH_FILE="$REPO/codex-rs/target/release/.codex-build-hash"
HEAD_HASH=$(git -C "$REPO" rev-parse HEAD)
status_porcelain=$(git -C "$REPO" status --porcelain)
if [ -n "$status_porcelain" ]; then
  # Keep hash format in sync with ~/.local/bin/codex-fork so we don't rebuild on every run
  # when the worktree is dirty but unchanged.
  status_hash=$(printf '%s' "$status_porcelain" | sha256sum | awk '{print $1}')
  HEAD_HASH="${HEAD_HASH}-dirty-${status_hash}"
fi
echo "$HEAD_HASH" > "$BUILD_HASH_FILE"

echo "Built: $REPO/codex-rs/target/release/codex"
echo "Build hash recorded: $BUILD_HASH_FILE"
