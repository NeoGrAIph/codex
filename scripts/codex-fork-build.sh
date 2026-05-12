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
BUILD_HASH_FILE="$REPO/scripts/.codex-build-hash"

compute_workspace_hash() {
  python3 - "$REPO" <<'PY'
import hashlib
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
head = subprocess.check_output([
    "git", "-C", str(repo), "rev-parse", "HEAD"
], text=True).strip()
tracked_diff = subprocess.check_output([
    "git", "-C", str(repo), "diff", "--binary", "--no-ext-diff", "HEAD", "--", ".", ":(exclude)scripts/.codex-build-hash"
])
untracked = subprocess.check_output([
    "git", "-C", str(repo), "ls-files", "--others", "--exclude-standard", "-z"
])
paths = [Path(p.decode()) for p in untracked.split(b"\0") if p]
if not tracked_diff and not paths:
    print(head)
    raise SystemExit
h = hashlib.sha256()
h.update(tracked_diff)
for rel_path in sorted(paths):
    h.update(b"\0UNTRACKED\0")
    h.update(rel_path.as_posix().encode())
    with (repo / rel_path).open("rb") as fh:
        while True:
            chunk = fh.read(65536)
            if not chunk:
                break
            h.update(chunk)
print(f"{head}-dirty-{h.hexdigest()}")
PY
}

echo "$(compute_workspace_hash)" > "$BUILD_HASH_FILE"

echo "Built: $REPO/codex-rs/target/release/codex"
echo "Build hash recorded: $BUILD_HASH_FILE"
