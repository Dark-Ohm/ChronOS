#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

assert_eq "$(wt_ticket_from_name ChronOS ChronOS-wt-t266)" "266"
assert_eq "$(wt_ticket_from_name ChronOS wt-t285)" "285"
assert_eq "$(wt_ticket_from_name ChronOS ChronOS-wt-t265A)" "265A"
assert_eq "$(wt_ticket_from_name Chronos-FM FM-wt-t051)" "051"
assert_eq "$(wt_ticket_from_name Chronos-FM t051-dnd)" "051"
assert_eq "$(wt_ticket_from_name ChronOS t051-dnd)" ""
assert_eq "$(wt_ticket_from_name Source Source-wt-t12)" "12"
assert_eq "$(wt_is_exception ChronOS ChronOS-wt-measure && echo yes)" "yes"
assert_eq "$(wt_is_exception ChronOS ChronOS-wt-t266 && echo yes || echo no)" "no"

# alias_bare / alias_legacy — свойство yaml, не хардкод пути
scratch="$(mktemp -d)"
cat >"$scratch/repos.yaml" <<EOF
repos:
  Bare:
    root: /x
    default_branch: master
    worktree_parent: /scratch/parent
    name_pattern: "Bare-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}"
    task_glob: null
    alias_bare: true
    alias_legacy: none
    exceptions: []
  Inside:
    root: /y
    default_branch: main
    worktree_parent: /scratch/inside/.worktrees
    name_pattern: "In-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}"
    task_glob: null
    alias_bare: false
    alias_legacy: t-slug
    exceptions: []
EOF
WT_REPOS_YAML="$scratch/repos.yaml"
assert_eq "$(wt_ticket_from_name Bare wt-t9)" "9"
assert_eq "$(wt_ticket_from_name Inside wt-t9)" ""
assert_eq "$(wt_ticket_from_name Inside t051-dnd)" "051"
assert_eq "$(wt_ticket_from_name Bare t051-dnd)" ""
rm -rf "$scratch"
