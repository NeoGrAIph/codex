#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || true)"

if [ -z "$REPO" ]; then
  echo "Failed to determine repository root from scripts/2_codex-fork-install-binary.sh" >&2
  exit 1
fi

SOURCE="$REPO/codex-rs/target/release/codex"
CLI_CODEX="$(command -v codex 2>/dev/null || true)"
CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
MANAGED_CODEX="$CODEX_HOME/packages/standalone/current/codex"
NPM_CODEX="$HOME/.nvm/versions/node/v22.22.0/bin/codex"
VENDOR_CODEX="$HOME/.nvm/versions/node/v22.22.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"
DRY_RUN=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--dry-run]

Install the fork release binary into the local Codex CLI and app-server runtime targets.

Source:
  $SOURCE

Targets:
  command -v codex
  $MANAGED_CODEX
  $NPM_CODEX
  $VENDOR_CODEX

Run scripts/codex-fork-build.sh first so the release binary is current.
This script does not stop, kill, start, or restart existing app-server processes.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
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

if [ -z "$CLI_CODEX" ]; then
  echo "Could not resolve codex in PATH." >&2
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

log() {
  echo "codex-fork install binary: $*" >&2
}

install_targets() {
  printf '%s\n' "$CLI_CODEX" "$MANAGED_CODEX" "$NPM_CODEX" "$VENDOR_CODEX" | awk 'NF && !seen[$0]++'
}

install_target() {
  local target="$1"
  local target_dir tmp

  target_dir="$(dirname -- "$target")"
  if [ ! -d "$target_dir" ]; then
    log "target directory is missing; skipping: $target_dir"
    return 0
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    if [ -f "$target" ] && cmp -s "$SOURCE" "$target"; then
      log "already current: $target"
    else
      log "would install fork binary to: $target"
    fi
    return 0
  fi

  if [ -f "$target" ] && cmp -s "$SOURCE" "$target"; then
    log "already current: $target"
    return 0
  fi

  tmp="$(mktemp "$target_dir/.codex-fork-install.XXXXXX")"
  cp "$SOURCE" "$tmp"
  chmod 0755 "$tmp"
  mv -f "$tmp" "$target"
  log "installed fork binary to: $target"
}

log "source: $SOURCE"

while IFS= read -r target; do
  [ -n "$target" ] || continue
  install_target "$target"
done < <(install_targets)

if [ "$DRY_RUN" -eq 1 ]; then
  log "dry run: no files changed"
fi
