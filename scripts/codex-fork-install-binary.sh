#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || true)"

if [ -z "$REPO" ]; then
  echo "Failed to determine repository root from scripts/codex-fork-install-binary.sh" >&2
  exit 1
fi

SOURCE="$REPO/codex-rs/target/release/codex"
TARGET="$(command -v codex 2>/dev/null || true)"
BACKUP_DIR="$REPO/scripts/.codex-binary-backups"
DRY_RUN=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--source PATH] [--target PATH] [--backup-dir PATH] [--dry-run]

Replace the currently resolved codex command with the release binary built from this fork.

Defaults:
  --source     $SOURCE
  --target     command -v codex
  --backup-dir $BACKUP_DIR

Run scripts/codex-fork-build.sh first so the release binary is current.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source)
      SOURCE="${2:-}"
      shift 2
      ;;
    --target)
      TARGET="${2:-}"
      shift 2
      ;;
    --backup-dir)
      BACKUP_DIR="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "$TARGET" ]; then
  echo "Could not resolve codex in PATH; pass --target explicitly." >&2
  exit 1
fi

if [ ! -f "$SOURCE" ]; then
  echo "Built fork binary not found: $SOURCE" >&2
  echo "Run scripts/codex-fork-build.sh first." >&2
  exit 1
fi

if [ ! -x "$SOURCE" ]; then
  echo "Built fork binary is not executable: $SOURCE" >&2
  exit 1
fi

TARGET_DIR="$(dirname -- "$TARGET")"
if [ ! -d "$TARGET_DIR" ]; then
  echo "Target directory does not exist: $TARGET_DIR" >&2
  exit 1
fi

timestamp="$(date +%Y%m%d-%H%M%S)"
backup="$BACKUP_DIR/codex.$timestamp"

echo "Source: $SOURCE"
echo "Target: $TARGET"
echo "Backup: $backup"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "Dry run: no files changed."
  exit 0
fi

mkdir -p "$BACKUP_DIR"

if [ -e "$TARGET" ] || [ -L "$TARGET" ]; then
  if [ -L "$TARGET" ]; then
    readlink "$TARGET" > "$backup.symlink-target"
    cp -aL "$TARGET" "$backup"
  else
    cp -a "$TARGET" "$backup"
  fi
  rm -f "$TARGET"
fi

install -m 0755 "$SOURCE" "$TARGET"

echo "Installed fork codex binary to: $TARGET"
echo "Previous target backed up at: $backup"
