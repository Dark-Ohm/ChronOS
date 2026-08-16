#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
for t in "$HERE"/test_*.sh; do
  echo "== $(basename "$t")"
  if bash "$t"; then
    echo "OK"
  else
    echo "FAIL $t"
    fail=1
  fi
done
exit "$fail"
