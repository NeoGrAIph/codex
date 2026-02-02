#!/usr/bin/env bash
set -euo pipefail

REPO="/home/neograiph/repo/AGENTS/codex"
cd "$REPO/codex-rs"

# Build release binary for fast local runs.
cargo build -p codex-cli --release

# Record build hash to detect stale binaries.
BUILD_HASH_FILE="$REPO/codex-rs/target/release/.codex-build-hash"
HEAD_HASH=$(git -C "$REPO" rev-parse HEAD)
if [ -n "$(git -C "$REPO" status --porcelain)" ]; then
  HEAD_HASH="${HEAD_HASH}-dirty"
fi
echo "$HEAD_HASH" > "$BUILD_HASH_FILE"

echo "Built: $REPO/codex-rs/target/release/codex"
echo "Build hash recorded: $BUILD_HASH_FILE"
