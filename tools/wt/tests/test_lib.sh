#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=helpers.sh
source "$HERE/helpers.sh"
# shellcheck source=../lib.sh
source "$HERE/../lib.sh"

keys="$(wt_repo_keys | tr '\n' ' ')"
assert_eq "$keys" "ChronOS Chronos-FM Chronos-Engine Chronos-lm Source " "repo keys order"

assert_eq "$(wt_repo_get ChronOS default_branch)" "master"
assert_eq "$(wt_repo_get Chronos-Engine default_branch)" "chronos-main"
assert_eq "$(wt_repo_get Chronos-FM worktree_parent)" \
  "/home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees"
assert_eq "$(wt_repo_get ChronOS name_pattern)" "ChronOS-wt-t{ticket}"
assert_eq "$(wt_expand "$(wt_repo_get ChronOS branch_pattern)" 266 blur)" \
  "feat/t266-blur"
assert_eq "$(wt_expand "ChronOS-wt-t{ticket}" 266)" "ChronOS-wt-t266"
assert_fail wt_repo_get NoSuchRepo root

tmpdir="$(mktemp -d)"
printf 'hello\n' | wt_atomic_write "$tmpdir/out.md"
assert_eq "$(cat "$tmpdir/out.md")" "hello"
rm -rf "$tmpdir"
