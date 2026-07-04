#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || true)"

if [ -z "$REPO" ]; then
  echo "Failed to determine repository root from scripts/3_codex-fork-restore-upstream-runtime.sh" >&2
  exit 1
fi

DRY_RUN=0
SKIP_NPM_INSTALL=0
NPM_PACKAGE="@openai/codex"

CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
MANAGED_CODEX="$CODEX_HOME/packages/standalone/current/codex"
NPM_CODEX="$HOME/.nvm/versions/node/v22.22.0/bin/codex"
VENDOR_CODEX="$HOME/.nvm/versions/node/v22.22.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--dry-run] [--skip-npm-install]

Restore local Codex app-server runtime targets to the upstream binary installed
by npm install -g @openai/codex.

This script stops current app-server processes, installs the upstream npm
package unless --skip-npm-install is passed, then copies the upstream npm vendor
binary to:
  $MANAGED_CODEX
  $NPM_CODEX
  $VENDOR_CODEX

It leaves the fork build artifact at $REPO/codex-rs/target/release/codex unchanged.
It does not explicitly start or restart the app-server daemon.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --skip-npm-install)
      SKIP_NPM_INSTALL=1
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

log() {
  echo "codex-fork upstream restore: $*" >&2
}

runtime_targets() {
  printf '%s\n' "$MANAGED_CODEX" "$NPM_CODEX" "$VENDOR_CODEX"
}

stop_current_app_server_processes() {
  if [ "$DRY_RUN" -eq 1 ]; then
    log "would stop current app-server processes with: pkill -f 'rg codex app-server|/codex app-server'"
    return 0
  fi

  log "stopping current app-server processes"
  pkill -f 'rg codex app-server|/codex app-server' 2>/dev/null || true
}

npm_global_root() {
  npm root -g
}

upstream_vendor_codex() {
  local root
  root="$(npm_global_root)"
  printf '%s\n' "$root/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex"
}

install_upstream_package() {
  if [ "$SKIP_NPM_INSTALL" -eq 1 ]; then
    log "skipping npm install"
    return 0
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    log "would run: npm install -g $NPM_PACKAGE"
    return 0
  fi

  log "installing upstream npm package: $NPM_PACKAGE"
  npm install -g "$NPM_PACKAGE"
}

install_runtime_target() {
  local source="$1"
  local target="$2"
  local target_dir tmp

  target_dir="$(dirname -- "$target")"
  if [ ! -d "$target_dir" ]; then
    log "runtime target directory is missing; skipping: $target_dir"
    return 0
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    log "would replace runtime target with upstream binary: $target"
    return 0
  fi

  tmp="$(mktemp "$target_dir/.codex-upstream-runtime.XXXXXX")"
  cp "$source" "$tmp"
  chmod 0755 "$tmp"
  mv -f "$tmp" "$target"
  log "updated runtime target: $target"
}

install_runtime_targets() {
  local source="$1"
  local target

  while IFS= read -r target; do
    [ -n "$target" ] || continue
    install_runtime_target "$source" "$target"
  done < <(runtime_targets)
}

stop_current_app_server_processes
install_upstream_package

SOURCE="$(upstream_vendor_codex)"
if [ "$DRY_RUN" -eq 1 ]; then
  log "would use upstream vendor binary: $SOURCE"
else
  if [ ! -f "$SOURCE" ]; then
    echo "Upstream vendor binary not found after npm install: $SOURCE" >&2
    exit 1
  fi
  if [ ! -x "$SOURCE" ]; then
    echo "Upstream vendor binary is not executable: $SOURCE" >&2
    exit 1
  fi
fi

install_runtime_targets "$SOURCE"

if [ "$DRY_RUN" -eq 1 ]; then
  log "dry run: no files changed"
else
  log "upstream runtime restore complete"
fi
