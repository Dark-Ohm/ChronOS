#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
source "$HERE/../lib.sh"

scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
make_git_repo "$scratch/repo" master
mkdir -p "$scratch/parent" "$scratch/status" \
  "$scratch/repo/docs/orchestration/tasks/active"
echo '# T1 no scope' >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md"
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m brief
cat >"$scratch/repos.yaml" <<EOF
repos:
  Toy:
    root: $scratch/repo
    default_branch: master
    worktree_parent: $scratch/parent
    name_pattern: "Toy-wt-t{ticket}"
    branch_pattern: "feat/t{ticket}-{slug}"
    task_glob: docs/orchestration/tasks/active/*.md
    alias_bare: false
    alias_legacy: none
    exceptions: []
EOF
export WT_REPOS_YAML="$scratch/repos.yaml" WT_STATUS_DIR="$scratch/status"

called="$scratch/omni-called"
rm -f "$called"
printf '#!/bin/sh\ntouch %q\ncat >/dev/null\necho SHOULD-NOT-RUN\n' "$called" >"$scratch/fake-omni"
chmod +x "$scratch/fake-omni"
export WT_OMNI_CURL="$scratch/fake-omni"

"$HERE/../wt-new.sh" Toy 1 demo
"$HERE/../wt-drift.sh"
grep -q 'no scope declared' "$scratch/status/DRIFT.md"
if [[ -e "$called" ]]; then
  echo FAIL omni called without scope; exit 1
fi

# now add scope + a denied file on the worktree
cat >"$scratch/repo/docs/orchestration/tasks/active/T1-demo.md" <<'MD'
# T1
## Scope (machine)
allow:
  - README
deny:
  - secret.txt
base: master
## Verification
do not include this heading in the scope block
MD
block="$(wt_scope_block "$scratch/repo/docs/orchestration/tasks/active/T1-demo.md")"
printf '%s\n' "$block" | grep -q '^## Scope (machine)'
if printf '%s\n' "$block" | grep -q '^## Verification'; then
  echo FAIL scope block leaked next heading; exit 1
fi
git -C "$scratch/repo" add docs && git -C "$scratch/repo" commit -q -m scope
# worktree is its own branch: commit deny file there
echo leak >"$scratch/parent/Toy-wt-t1/secret.txt"
git -C "$scratch/parent/Toy-wt-t1" add secret.txt
git -C "$scratch/parent/Toy-wt-t1" commit -q -m leak

printf '#!/bin/sh\ncat >/dev/null\necho DRIFT HIT\n' >"$scratch/fake-omni"
"$HERE/../wt-drift.sh"
grep -q 'DRIFT HIT' "$scratch/status/DRIFT.md"
grep -q '## T1 (Toy)' "$scratch/status/DRIFT.md"

assert_eq "$(wt_extract_scope_base "$scratch/repo/docs/orchestration/tasks/active/T1-demo.md")" "master"
