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

This only manages the local app-server daemon. It does not stop active TUI
sessions, resume sessions, npm vendor binaries, or unrelated Codex processes.
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
MANAGED_DIR="$(dirname -- "$MANAGED_CODEX")"
PID_FILE="$CODEX_HOME/app-server-daemon/app-server.pid"
STATE_FILE="$CODEX_HOME/app-server-daemon/codex-fork-runtime-sync.env"
BUILD_HASH_FILE="$REPO/scripts/.codex-build-hash"

log() {
  echo "codex-fork runtime sync: $*" >&2
}

file_stamp() {
  stat -c '%s:%Y' "$1"
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

daemon_status() {
  local output
  if ! output="$("$SOURCE" app-server daemon version 2>/dev/null)"; then
    echo "unknown"
    return 0
  fi
  python3 -c 'import json, sys; print(json.load(sys.stdin).get("status", "unknown"))' <<<"$output" 2>/dev/null || echo "unknown"
}

daemon_pid() {
  if [ -f "$PID_FILE" ]; then
    python3 - "$PID_FILE" <<'PY' 2>/dev/null || sed -n '1p' "$PID_FILE" 2>/dev/null || true
import json
import sys

path = sys.argv[1]
text = open(path, encoding="utf-8").read().strip()
try:
    value = json.loads(text)
except json.JSONDecodeError:
    print(text)
else:
    print(value.get("pid", ""))
PY
  fi
}

is_pid_running() {
  local pid="$1"
  [ -n "$pid" ] && [ -d "/proc/$pid" ]
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
  local pid="$3"

  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi

  mkdir -p "$(dirname -- "$STATE_FILE")"
  tmp_state="$(mktemp "$(dirname -- "$STATE_FILE")/.codex-fork-runtime-sync.XXXXXX")"
  {
    printf 'SOURCE_STAMP=%s\n' "$source_state"
    printf 'MANAGED_STAMP=%s\n' "$managed_state"
    printf 'DAEMON_PID=%s\n' "$pid"
  } >"$tmp_state"
  mv -f "$tmp_state" "$STATE_FILE"
}

can_fast_exit() {
  local source_state="$1"
  local managed_state="$2"
  local pid="$3"

  [ "$FORCE" -eq 0 ] || return 1
  [ -n "$pid" ] || return 1
  is_pid_running "$pid" || return 1
  [ "$(state_value SOURCE_STAMP)" = "$source_state" ] || return 1
  [ "$(state_value MANAGED_STAMP)" = "$managed_state" ] || return 1
  [ "$(state_value DAEMON_PID)" = "$pid" ] || return 1
  return 0
}

running_daemon_needs_restart() {
  local pid exe_link

  pid="$(daemon_pid)"
  if ! is_pid_running "$pid"; then
    return 1
  fi

  exe_link="$(readlink "/proc/$pid/exe" 2>/dev/null || true)"
  if [ -z "$exe_link" ]; then
    return 0
  fi

  if [[ "$exe_link" == *" (deleted)" ]] || [[ "$exe_link" == *"(deleted)" ]]; then
    return 0
  fi

  if ! cmp -s "/proc/$pid/exe" "$SOURCE"; then
    return 0
  fi

  return 1
}

if [ ! -e "$MANAGED_CODEX" ]; then
  log "managed standalone binary is missing; skipping daemon runtime sync: $MANAGED_CODEX"
  exit 0
fi

if [ ! -d "$MANAGED_DIR" ]; then
  log "managed standalone directory is missing; skipping daemon runtime sync: $MANAGED_DIR"
  exit 0
fi

source_state="$(source_stamp)"
managed_state="$(file_stamp "$MANAGED_CODEX")"
pid="$(daemon_pid)"

if can_fast_exit "$source_state" "$managed_state" "$pid"; then
  exit 0
fi

status="$(daemon_status)"
copied=0

if [ "$FORCE" -eq 1 ] || ! cmp -s "$SOURCE" "$MANAGED_CODEX"; then
  if [ "$DRY_RUN" -eq 1 ]; then
    log "would update managed app-server binary: $MANAGED_CODEX"
  else
    tmp="$(mktemp "$MANAGED_DIR/.codex-fork-managed.XXXXXX")"
    cleanup_tmp() {
      rm -f "$tmp"
    }
    trap cleanup_tmp EXIT
    cp "$SOURCE" "$tmp"
    chmod 0755 "$tmp"
    mv -f "$tmp" "$MANAGED_CODEX"
    trap - EXIT
    copied=1
    managed_state="$(file_stamp "$MANAGED_CODEX")"
    log "updated managed app-server binary: $MANAGED_CODEX"
  fi
fi

if [ "$status" != "running" ]; then
  write_state "$source_state" "$managed_state" ""
  exit 0
fi

needs_restart=0
if [ "$copied" -eq 1 ]; then
  needs_restart=1
elif running_daemon_needs_restart; then
  needs_restart=1
fi

if [ "$needs_restart" -eq 0 ]; then
  write_state "$source_state" "$managed_state" "$(daemon_pid)"
  exit 0
fi

if [ "$DRY_RUN" -eq 1 ]; then
  log "would restart managed app-server daemon"
  exit 0
fi

log "restarting managed app-server daemon"
"$SOURCE" app-server daemon restart >&2
write_state "$source_state" "$(file_stamp "$MANAGED_CODEX")" "$(daemon_pid)"
