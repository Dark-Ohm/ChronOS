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

make_git_repo() {
  local d="$1" branch="${2:-master}"
  mkdir -p "$d"
  git -C "$d" init -q -b "$branch"
  git -C "$d" config user.email t@t
  git -C "$d" config user.name t
  echo ok >"$d/README"
  git -C "$d" add README
  git -C "$d" commit -q -m init
}
