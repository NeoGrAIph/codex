#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || true)"

if [ -z "$REPO" ]; then
  echo "Failed to determine repository root from scripts/codex-fork-sync-runtime.sh" >&2
  exit 1
fi

SOURCE="$REPO/codex-rs/target/release/codex"
DRY_RUN=0
FORCE=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [--source PATH] [--dry-run] [--force]

Synchronize the managed app-server daemon runtime with the fork release binary.

Defaults:
  --source     $SOURCE

This updates the managed local app-server binary targets. It does not stop,
kill, or restart already running app-server processes; running processes keep
using their current executable until they exit naturally.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source)
      SOURCE="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --force)
      FORCE=1
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

if [ -z "$SOURCE" ]; then
  echo "Source binary path cannot be empty." >&2
  exit 1
fi

if [ ! -f "$SOURCE" ]; then
  echo "Fork release binary not found: $SOURCE" >&2
  exit 1
fi

if [ ! -x "$SOURCE" ]; then
  echo "Fork release binary is not executable: $SOURCE" >&2
  exit 1
fi

CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
MANAGED_CODEX="$CODEX_HOME/packages/standalone/current/codex"
NPM_CODEX="$HOME/.nvm/versions/node/v22.22.0/bin/codex"
VENDOR_CODEX="$HOME/.nvm/versions/node/v22.22.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"
STATE_FILE="$CODEX_HOME/app-server-daemon/codex-fork-runtime-sync.env"
BUILD_HASH_FILE="$REPO/scripts/.codex-build-hash"

log() {
  echo "codex-fork runtime sync: $*" >&2
}

file_stamp() {
  stat -c '%s:%Y' "$1"
}

managed_stamp() {
  if [ -f "$MANAGED_CODEX" ]; then
    file_stamp "$MANAGED_CODEX"
  else
    printf 'missing\n'
  fi
}

runtime_targets() {
  printf '%s\n' "$MANAGED_CODEX" "$NPM_CODEX" "$VENDOR_CODEX"
}

source_stamp() {
  local file_state
  file_state="$(file_stamp "$SOURCE")"
  if [ -f "$BUILD_HASH_FILE" ]; then
    printf '%s:%s\n' "$(cat "$BUILD_HASH_FILE")" "$file_state"
  else
    printf 'no-build-hash:%s\n' "$file_state"
  fi
}

state_value() {
  local key="$1"
  if [ -f "$STATE_FILE" ]; then
    grep -E "^${key}=" "$STATE_FILE" 2>/dev/null | tail -n 1 | cut -d= -f2- || true
  fi
}

write_state() {
  local source_state="$1"
  local managed_state="$2"

  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi

  mkdir -p "$(dirname -- "$STATE_FILE")"
  tmp_state="$(mktemp "$(dirname -- "$STATE_FILE")/.codex-fork-runtime-sync.XXXXXX")"
  {
    printf 'SOURCE_STAMP=%s\n' "$source_state"
    printf 'MANAGED_STAMP=%s\n' "$managed_state"
  } >"$tmp_state"
  mv -f "$tmp_state" "$STATE_FILE"
}

can_fast_exit() {
  local source_state="$1"
  local managed_state="$2"

  [ "$FORCE" -eq 0 ] || return 1
  [ "$(state_value SOURCE_STAMP)" = "$source_state" ] || return 1
  [ "$(state_value MANAGED_STAMP)" = "$managed_state" ] || return 1
  all_runtime_targets_current || return 1
  return 0
}

all_runtime_targets_current() {
  local target

  while IFS= read -r target; do
    [ -n "$target" ] || continue
    [ -f "$target" ] || return 1
    cmp -s "$SOURCE" "$target" || return 1
  done < <(runtime_targets)

  return 0
}

update_runtime_target() {
  local target="$1"
  local target_dir tmp

  target_dir="$(dirname -- "$target")"
  if [ ! -d "$target_dir" ]; then
    log "runtime target directory is missing; skipping: $target_dir"
    return 0
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    if [ "$FORCE" -eq 1 ] || ! [ -f "$target" ] || ! cmp -s "$SOURCE" "$target"; then
      log "would update runtime binary: $target"
    fi
    return 0
  fi

  if [ "$FORCE" -eq 0 ] && [ -f "$target" ] && cmp -s "$SOURCE" "$target"; then
    return 0
  fi

  tmp="$(mktemp "$target_dir/.codex-fork-runtime.XXXXXX")"
  cp "$SOURCE" "$tmp"
  chmod 0755 "$tmp"
  mv -f "$tmp" "$target"
  log "updated runtime binary: $target"
}

update_runtime_targets() {
  local target

  while IFS= read -r target; do
    [ -n "$target" ] || continue
    update_runtime_target "$target"
  done < <(runtime_targets)
}

source_state="$(source_stamp)"
managed_state="$(managed_stamp)"

if can_fast_exit "$source_state" "$managed_state"; then
  exit 0
fi

update_runtime_targets
write_state "$source_state" "$(managed_stamp)"
