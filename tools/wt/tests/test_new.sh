#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent"
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: null
    alias_bare: false
    alias_legacy: none
    exceptions: []
EOF
export WT_REPOS_YAML="$scratch/repos.yaml"

NEW="$HERE/../wt-new.sh"

assert_fail "$NEW" Nope 1 foo
"$NEW" Toy 1 demo
assert_eq "$(git -C "$scratch/repo" worktree list | wc -l | tr -d ' ')" "2"
[[ -d "$scratch/parent/Toy-wt-t1" ]]
[[ -d "$scratch/parent/Toy-wt-t1-target" ]]
[[ -f "$scratch/parent/Toy-wt-t1/.envrc" ]]
grep -q CARGO_TARGET_DIR "$scratch/parent/Toy-wt-t1/.envrc"
assert_eq "$(git -C "$scratch/parent/Toy-wt-t1" branch --show-current)" "feat/t1-demo"
# stacked commit not on master must fail
git -C "$scratch/repo" checkout -q -b side
echo x >>"$scratch/repo/README"
git -C "$scratch/repo" commit -q -am side
side="$(git -C "$scratch/repo" rev-parse HEAD)"
git -C "$scratch/repo" checkout -q master
assert_fail "$NEW" Toy 2 stacked --base "$side"
