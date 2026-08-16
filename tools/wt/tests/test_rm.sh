#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
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
"$HERE/../wt-new.sh" Toy 3 x
echo dirt >"$scratch/parent/Toy-wt-t3/x"
if "$HERE/../wt-rm.sh" Toy 3; then echo FAIL expected refuse; exit 1; fi
[[ -d "$scratch/parent/Toy-wt-t3" ]]
WT_RM_CONFIRM=YES "$HERE/../wt-rm.sh" Toy 3 --force
[[ ! -d "$scratch/parent/Toy-wt-t3" ]]
[[ ! -d "$scratch/parent/Toy-wt-t3-target" ]]
"$HERE/../wt-new.sh" Toy 4 y
"$HERE/../wt-rm.sh" Toy 4
[[ ! -d "$scratch/parent/Toy-wt-t4" ]]
