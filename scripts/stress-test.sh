#!/usr/bin/env bash
set -uo pipefail

runs="${1:-1}"
passes=0
failures=0
failed_runs=""

for run in $(seq 1 "$runs"); do
  printf '\n=== Stress run %s/%s ===\n' "$run" "$runs"

  if cargo test 2>&1 | trunc; then
    passes=$((passes + 1))
    printf 'Result: PASS\n'
  else
    failures=$((failures + 1))
    failed_runs="${failed_runs} ${run}"
    printf 'Result: FAIL\n'
  fi
done

printf '\n=== Summary ===\n'
printf 'Total runs: %s\n' "$runs"
printf 'Passed: %s\n' "$passes"
printf 'Failed: %s\n' "$failures"

if [ -n "$failed_runs" ]; then
  printf 'Failed runs:%s\n' "$failed_runs"
fi

if [ "$failures" -eq 0 ]; then
  printf 'Pass rate: 100%%\n'
else
  awk -v passes="$passes" -v runs="$runs" 'BEGIN { printf "Pass rate: %.1f%%\n", (passes / runs) * 100 }'
fi

if [ "$failures" -eq 0 ]; then
  exit 0
fi

exit 1
