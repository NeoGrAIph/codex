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
TERM_TIMEOUT_SECONDS=10
KILL_TIMEOUT_SECONDS=5

usage() {
  cat <<EOF
Usage: $(basename "$0") [--source PATH] [--dry-run] [--force]

Synchronize the managed app-server daemon runtime with the fork release binary.

Defaults:
  --source     $SOURCE

This updates the managed local app-server binary and stops already running
app-server processes. It does not explicitly restart the daemon; the next client
request should start it through the normal app-server path.
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
NPM_CODEX="$HOME/.nvm/versions/node/v22.22.0/bin/codex"
VENDOR_CODEX="$HOME/.nvm/versions/node/v22.22.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"
PID_FILE="$CODEX_HOME/app-server-daemon/app-server.pid"
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

daemon_status() {
  local output
  if ! output="$("$SOURCE" app-server daemon version 2>/dev/null)"; then
    echo "unknown"
    return 0
  fi
  python3 -c 'import json, sys; print(json.load(sys.stdin).get("status", "unknown"))' <<<"$output" 2>/dev/null || echo "unknown"
}

daemon_backend() {
  local output
  if ! output="$("$SOURCE" app-server daemon version 2>/dev/null)"; then
    echo ""
    return 0
  fi
  python3 -c 'import json, sys; print(json.load(sys.stdin).get("backend") or "")' <<<"$output" 2>/dev/null || echo ""
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
  all_runtime_targets_current || return 1
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

socket_owner_pids() {
  local socket_path="$1"
  python3 - "$socket_path" <<'PY' 2>/dev/null || true
import os
import sys

socket_path = os.path.realpath(sys.argv[1])
inodes = set()

try:
    with open("/proc/net/unix", encoding="utf-8", errors="replace") as proc_net_unix:
        next(proc_net_unix, None)
        for line in proc_net_unix:
            parts = line.split()
            if len(parts) < 8:
                continue
            path = parts[-1]
            if path and os.path.realpath(path) == socket_path:
                inodes.add(parts[6])
except OSError:
    pass

if not inodes:
    raise SystemExit

pids = set()
for proc_name in os.listdir("/proc"):
    if not proc_name.isdigit():
        continue
    fd_dir = os.path.join("/proc", proc_name, "fd")
    try:
        fd_names = os.listdir(fd_dir)
    except OSError:
        continue
    for fd_name in fd_names:
        try:
            target = os.readlink(os.path.join(fd_dir, fd_name))
        except OSError:
            continue
        if target.startswith("socket:[") and target[8:-1] in inodes:
            pids.add(int(proc_name))
            break

for pid in sorted(pids):
    print(pid)
PY
}

probe_app_server_socket() {
  "$SOURCE" app-server daemon version >/dev/null 2>&1
}

wait_for_socket_to_clear() {
  local deadline="$1"
  local now

  while probe_app_server_socket; do
    now="$(date +%s)"
    if [ "$now" -ge "$deadline" ]; then
      return 1
    fi
    sleep 0.2
  done

  return 0
}

terminate_pids() {
  local signal="$1"
  shift
  local pid

  for pid in "$@"; do
    if is_pid_running "$pid"; then
      kill "-$signal" "$pid" 2>/dev/null || true
    fi
  done
}

hard_stop_unmanaged_app_server() {
  local pids deadline

  mapfile -t pids < <(socket_owner_pids "$CODEX_HOME/app-server-control/app-server-control.sock")
  if [ "${#pids[@]}" -eq 0 ]; then
    log "unmanaged app-server socket is reachable but no socket owner pid was found"
    return 1
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    log "would terminate unmanaged app-server socket owner pid(s): ${pids[*]}"
    return 0
  fi

  log "terminating unmanaged app-server socket owner pid(s): ${pids[*]}"
  terminate_pids TERM "${pids[@]}"
  deadline="$(($(date +%s) + TERM_TIMEOUT_SECONDS))"
  if wait_for_socket_to_clear "$deadline"; then
    return 0
  fi

  mapfile -t pids < <(socket_owner_pids "$CODEX_HOME/app-server-control/app-server-control.sock")
  if [ "${#pids[@]}" -gt 0 ]; then
    log "force killing unmanaged app-server socket owner pid(s): ${pids[*]}"
    terminate_pids KILL "${pids[@]}"
  fi

  deadline="$(($(date +%s) + KILL_TIMEOUT_SECONDS))"
  if ! wait_for_socket_to_clear "$deadline"; then
    log "failed to clear unmanaged app-server socket after TERM and KILL"
    return 1
  fi

  return 0
}

stop_current_app_server_processes() {
  if [ "$DRY_RUN" -eq 1 ]; then
    log "would stop current app-server processes with: pkill -f 'rg codex app-server|/codex app-server'"
    return 0
  fi

  log "stopping current app-server processes"
  pkill -f 'rg codex app-server|/codex app-server' 2>/dev/null || true
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
pid="$(daemon_pid)"

if can_fast_exit "$source_state" "$managed_state" "$pid"; then
  exit 0
fi

status="$(daemon_status)"
backend="$(daemon_backend)"
takeover=0

if [ "$status" = "running" ] && [ "$backend" != "pid" ]; then
  hard_stop_unmanaged_app_server
  takeover=1
  if [ "$DRY_RUN" -eq 1 ]; then
    log "would stop current app-server processes without restarting daemon"
    update_runtime_targets
    exit 0
  fi
  status="$(daemon_status)"
  backend="$(daemon_backend)"
fi

if [ "$status" != "running" ]; then
  if [ "$DRY_RUN" -eq 1 ]; then
    if [ "$takeover" -eq 1 ]; then
      log "would stop current app-server processes without restarting daemon"
    else
      log "would not start managed app-server daemon"
    fi
    update_runtime_targets
    exit 0
  fi

  if [ "$takeover" -eq 1 ]; then
    stop_current_app_server_processes
  else
    log "managed app-server daemon is not running; leaving it stopped"
  fi
  update_runtime_targets
  write_state "$source_state" "$(managed_stamp)" "$(daemon_pid)"
  exit 0
fi

needs_restart=0
if [ "$FORCE" -eq 1 ] || [ "$(state_value SOURCE_STAMP)" != "$source_state" ] || ! all_runtime_targets_current; then
  needs_restart=1
elif running_daemon_needs_restart; then
  needs_restart=1
fi

if [ "$needs_restart" -eq 0 ]; then
  write_state "$source_state" "$(managed_stamp)" "$(daemon_pid)"
  exit 0
fi

if [ "$DRY_RUN" -eq 1 ]; then
  log "would stop current app-server processes without restarting daemon"
  update_runtime_targets
  exit 0
fi

stop_current_app_server_processes
update_runtime_targets
write_state "$source_state" "$(managed_stamp)" "$(daemon_pid)"
