#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

usage() { echo "usage: wt-new.sh <repo> <ticket> [slug] [--base <sha>]" >&2; }

repo="" ticket="" slug="" base=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base) base="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      if [[ -z "$repo" ]]; then repo="$1"
      elif [[ -z "$ticket" ]]; then ticket="$1"
      elif [[ -z "$slug" ]]; then slug="$1"
      else usage; exit 1
      fi
      shift
      ;;
  esac
done
[[ -n "$repo" && -n "$ticket" ]] || { usage; exit 1; }

if ! root="$(wt_repo_get "$repo" root)"; then
  echo "wt-new: unknown repo '$repo' — add it to $WT_REPOS_YAML" >&2
  exit 1
fi
branch_def="$(wt_repo_get "$repo" default_branch)"
parent="$(wt_repo_get "$repo" worktree_parent)"
name="$(wt_expand "$(wt_repo_get "$repo" name_pattern)" "$ticket" "$slug")"
branch="$(wt_expand "$(wt_repo_get "$repo" branch_pattern)" "$ticket" "$slug")"
path="$parent/$name"

if [[ -z "$base" ]]; then
  base="$(git -C "$root" rev-parse --verify "$branch_def")"
fi
if ! git -C "$root" merge-base --is-ancestor "$base" "$branch_def"; then
  echo "wt-new: $base is not an ancestor of $branch_def (v1: no stack on unmerged tickets; use git worktree add by hand)" >&2
  exit 1
fi
if [[ -e "$path" ]]; then
  echo "wt-new: $path already exists" >&2
  exit 1
fi
mkdir -p "$parent"
git -C "$root" worktree add -b "$branch" "$path" "$base"
sidecar="${path}-target"
mkdir -p "$sidecar"
printf 'export CARGO_TARGET_DIR=%q\n' "$sidecar" >"$path/.envrc"
exclude="$root/.git/info/exclude"
mkdir -p "$(dirname "$exclude")"
if ! grep -qxF '.envrc' "$exclude" 2>/dev/null; then
  printf '.envrc\n' >>"$exclude"
fi
echo "created $path (branch $branch, target $sidecar)"
