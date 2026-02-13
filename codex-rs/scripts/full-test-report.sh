#!/usr/bin/env bash

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

declare -A STATUSES
declare -A LOG_FILES

cleanup() {
  for log_file in "${LOG_FILES[@]}"; do
    [[ -n "$log_file" ]] && rm -f "$log_file"
  done
}

trap cleanup EXIT

run_step() {
  local name="$1"
  local cmd="$2"
  local log_file
  log_file="$(mktemp -t codex-full-test-report.${name}.XXXXXX.log)"
  LOG_FILES["$name"]="$log_file"

  echo "Running: ${cmd}" >&2

  if (cd "$ROOT_DIR" && bash -lc "$cmd" >"$log_file" 2>&1); then
    STATUSES["$name"]=0
  else
    STATUSES["$name"]=$?
  fi
}

print_problem_summary() {
  local name="$1"
  local cmd="$2"
  local log_file="${LOG_FILES[$name]}"
  local status="${STATUSES[$name]}"

  if [[ "$status" == "0" ]]; then
    return
  fi

  echo "=== ${name} FAILED (exit ${status}) ==="
  echo "Command: ${cmd}"

  if grep -q '^failures:$' "$log_file"; then
    echo "Failed tests:"
    awk '
      /^failures:$/ { count++; if (count==2) in_failures=1; next }
      in_failures && NF==0 { exit }
      in_failures { print "  " $0 }
    ' "$log_file"
    echo
    echo "Failure details:"
    awk '
      /^failures:$/ { count++; if (count==1) in_details=1; next }
      in_details && /^failures:$/ { exit }
      in_details { print "  " $0 }
    ' "$log_file"
  else
    echo "Error tail:"
    tail -n 30 "$log_file" | sed 's/^/  /'
  fi

  if grep -q '^test result: FAILED' "$log_file"; then
    echo "Result line:"
    grep '^test result: FAILED' "$log_file" | tail -n 1 | sed 's/^/  /'
  fi
  echo
}

run_step "fmt" "just fmt"
run_step "tui_tests" "cargo test -p codex-tui"
run_step "all_features_tests" "RUST_TEST_THREADS=${RUST_TEST_THREADS:-1} cargo test --all-features"

has_failures=0

print_problem_summary "fmt" "just fmt"
if [[ "${STATUSES[fmt]}" != "0" ]]; then
  has_failures=1
fi

print_problem_summary "tui_tests" "cargo test -p codex-tui"
if [[ "${STATUSES[tui_tests]}" != "0" ]]; then
  has_failures=1
fi

print_problem_summary "all_features_tests" "RUST_TEST_THREADS=${RUST_TEST_THREADS:-1} cargo test --all-features"
if [[ "${STATUSES[all_features_tests]}" != "0" ]]; then
  has_failures=1
fi

if [[ "$has_failures" == "0" ]]; then
  echo "Проблем не найдено."
  exit 0
fi

echo "Обнаружены проблемы."
exit 1
