#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"
scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent" "$scratch/status" \
  "$scratch/repo/docs/orchestration/tasks/active/pause" \
  "$scratch/repo/docs/orchestration/tasks/active/check" \
  "$scratch/repo/docs/orchestration/tasks/done"
echo '# T1' >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md"
echo '# T2' >"$scratch/repo/docs/orchestration/tasks/active/pause/T2-hold.md"
echo '# T3' >"$scratch/repo/docs/orchestration/tasks/active/check/T3-qa.md"
echo '# T4' >"$scratch/repo/docs/orchestration/tasks/done/T4-old.md"
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m tasks
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: true
    alias_legacy: none
    exceptions: [Toy-keep]
EOF
export WT_REPOS_YAML="$scratch/repos.yaml" WT_STATUS_DIR="$scratch/status"

assert_eq "$(wt_task_state Toy 1)" "active"
assert_eq "$(wt_task_state Toy 2)" "pause"
assert_eq "$(wt_task_state Toy 3)" "check"
assert_eq "$(wt_task_state Toy 4)" "done"
assert_eq "$(wt_task_state Toy 99)" "none"

"$HERE/../wt-new.sh" Toy 1 demo
# detached at a commit already on master — naive --merged would say yes
git -C "$scratch/repo" worktree add --detach "$scratch/parent/Toy-wt-t9" HEAD

"$HERE/../wt-status.sh"
st="$scratch/status/STATUS.md"
grep -q 'name: Toy-wt-t1' "$st"
grep -q 'task: active' "$st"
if grep -q "name: repo$" "$st"; then
  echo FAIL primary listed; exit 1
fi
# extract the t9 block
python3 - "$st" <<'PY'
import sys
text = open(sys.argv[1]).read().split("- path:")
block = next(b for b in text if "name: Toy-wt-t9" in b)
assert "branch: detached" in block, block
assert "merged: no" in block, block
PY
