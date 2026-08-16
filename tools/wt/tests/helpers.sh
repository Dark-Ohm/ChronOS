#!/usr/bin/env bash
set -euo pipefail
assert_eq() {
  local got="$1" want="$2" msg="${3:-}"
  if [[ "$got" != "$want" ]]; then
    echo "FAIL ${msg}: got=$(printf %q "$got") want=$(printf %q "$want")" >&2
    return 1
  fi
}
assert_fail() {
  if "$@"; then
    echo "FAIL expected failure: $*" >&2
    return 1
  fi
}
