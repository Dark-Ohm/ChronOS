#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/lib.sh"

force=0
repo="" ticket=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --force) force=1; shift ;;
    *)
      if [[ -z "$repo" ]]; then repo="$1"
      elif [[ -z "$ticket" ]]; then ticket="$1"
      else echo "usage: wt-rm.sh <repo> <ticket> [--force]" >&2; exit 1
      fi
      shift
      ;;
  esac
done
[[ -n "$repo" && -n "$ticket" ]] || { echo "usage: wt-rm.sh <repo> <ticket> [--force]" >&2; exit 1; }

root="$(wt_repo_get "$repo" root)" || { echo "unknown repo" >&2; exit 1; }
parent="$(wt_repo_get "$repo" worktree_parent)"
canon="$parent/$(wt_expand "$(wt_repo_get "$repo" name_pattern)" "$ticket")"
path=""
if [[ -d "$canon" ]]; then
  path="$canon"
else
  while IFS= read -r line; do
    [[ "$line" == worktree\ * ]] || continue
    p="${line#worktree }"
    [[ "$(realpath "$p")" == "$(realpath "$root")" ]] && continue
    t="$(wt_ticket_from_name "$repo" "$(basename "$p")")"
    if [[ "$t" == "$ticket" ]]; then path="$p"; break; fi
  done < <(git -C "$root" worktree list --porcelain)
fi
[[ -n "$path" && -d "$path" ]] || { echo "wt-rm: no worktree for $repo $ticket" >&2; exit 1; }

branch_def="$(wt_repo_get "$repo" default_branch)"
dirty="$(git -C "$path" status --short)"
ahead="$(git -C "$path" log --oneline "$branch_def..HEAD" || true)"
if [[ -n "$dirty" || -n "$ahead" ]]; then
  if [[ "$force" -ne 1 ]]; then
    echo "wt-rm: dirty or unmerged commits; pass --force" >&2
    echo "$dirty" >&2
    echo "$ahead" >&2
    exit 1
  fi
  echo "WILL LOSE:"
  echo "$dirty"
  echo "$ahead"
  if [[ "${WT_RM_CONFIRM:-}" != "YES" ]]; then
    printf 'type YES: '
    read -r ans
    [[ "$ans" == "YES" ]] || exit 1
  fi
fi
if [[ "$force" -eq 1 ]]; then
  git -C "$root" worktree remove --force "$path"
else
  git -C "$root" worktree remove "$path"
fi
git -C "$root" worktree prune
rm -rf "${path}-target"
echo "removed $path"
