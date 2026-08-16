#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

wt_status_emit_one() {
  local repo="$1" path="$2" head="$3" branch_line="$4" detached="$5"
  local root branch_def rr rp name ticket dirty ahead_n merged exc task
  root="$(wt_repo_get "$repo" root)"
  branch_def="$(wt_repo_get "$repo" default_branch)"
  rp="$(realpath "$path")"
  rr="$(realpath "$root")"
  [[ "$rp" == "$rr" ]] && return 0
  name="$(basename "$path")"
  ticket="$(wt_ticket_from_name "$repo" "$name")"
  if wt_is_exception "$repo" "$name"; then exc=yes; else exc=no; fi
  if [[ -n "$(git -C "$path" status --short)" ]]; then dirty=yes; else dirty=no; fi
  ahead_n="$(git -C "$path" rev-list --count "${branch_def}..HEAD" 2>/dev/null || printf '0')"
  if [[ "$detached" == "1" ]]; then
    branch_line=detached
    merged=no
  else
    branch_line="${branch_line#refs/heads/}"
    if [[ "$ahead_n" == "0" ]] && git -C "$root" merge-base --is-ancestor "$head" "$branch_def"; then
      merged=yes
    else
      merged=no
    fi
  fi
  if [[ -n "$ticket" ]]; then
    task="$(wt_task_state "$repo" "$ticket")"
  else
    task=none
  fi
  printf -- '- path: %s\n' "$path"
  printf '  name: %s\n' "$name"
  printf '  ticket: %s\n' "$ticket"
  printf '  branch: %s\n' "$branch_line"
  printf '  dirty: %s\n' "$dirty"
  printf '  ahead: %s\n' "$ahead_n"
  printf '  merged: %s\n' "$merged"
  printf '  exception: %s\n' "$exc"
  printf '  task: %s\n' "$task"
  printf '\n'
}

wt_status_emit_repo() {
  local repo="$1" path="" head="" branch="" detached=0 line
  printf '## %s\n' "$repo"
  while IFS= read -r line; do
    case "$line" in
      worktree\ *)
        if [[ -n "$path" ]]; then
          wt_status_emit_one "$repo" "$path" "$head" "$branch" "$detached"
        fi
        path="${line#worktree }"
        head="" branch="" detached=0
        ;;
      HEAD\ *) head="${line#HEAD }" ;;
      branch\ *) branch="${line#branch }" ;;
      detached) detached=1 ;;
    esac
  done < <(git -C "$(wt_repo_get "$repo" root)" worktree list --porcelain)
  if [[ -n "$path" ]]; then
    wt_status_emit_one "$repo" "$path" "$head" "$branch" "$detached"
  fi
  printf '\n'
}

{
  printf '# worktree status\n'
  printf 'generated: %s\n\n' "$(date -Iseconds)"
  while IFS= read -r repo; do
    [[ -n "$repo" ]] || continue
    wt_status_emit_repo "$repo"
  done < <(wt_repo_keys)
} | wt_atomic_write "$(wt_status_dir)/STATUS.md"

cat "$(wt_status_dir)/STATUS.md"
