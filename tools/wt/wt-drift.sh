#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$here/lib.sh"

wt_find_live_brief() {
  local repo="$1" ticket="$2" root tasks f
  root="$(wt_repo_get "$repo" root)"
  tasks="$root/docs/orchestration/tasks"
  for f in \
    "$tasks/active/T${ticket}-"*.md \
    "$tasks/active/pause/T${ticket}-"*.md \
    "$tasks/active/check/T${ticket}-"*.md
  do
    [[ -e "$f" ]] || continue
    printf '%s\n' "$f"
    return 0
  done
  return 1
}

sections=""
while IFS= read -r repo; do
  [[ -n "$repo" ]] || continue
  root="$(wt_repo_get "$repo" root)"
  rr="$(realpath "$root")"
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    [[ "$(realpath "$path")" == "$rr" ]] && continue
    name="$(basename "$path")"
    wt_is_exception "$repo" "$name" && continue
    ticket="$(wt_ticket_from_name "$repo" "$name")"
    if [[ -z "$ticket" ]]; then
      sections+="## ${name} (${repo})"$'\\n'"no ticket parsed"$'\\n\\n'
      continue
    fi
    brief=""
    if ! brief="$(wt_find_live_brief "$repo" "$ticket")"; then
      sections+="## T${ticket} (${repo})"$'\\n'"no scope declared"$'\\n\\n'
      continue
    fi
    scope="$(wt_scope_block "$brief")"
    if ! printf '%s\n' "$scope" | grep -q '^## Scope (machine)'; then
      sections+="## T${ticket} (${repo})"$'\\n'"no scope declared"$'\\n\\n'
      continue
    fi
    base="$(wt_extract_scope_base "$brief")"
    [[ -n "$base" ]] || base="$(wt_repo_get "$repo" default_branch)"
    names="$(git -C "$path" diff --name-only "$base"..HEAD || true)"
    prompt="$(
      cat "$here/prompts/drift.txt"
      printf '\\n%s\\n\\n# diff --name-only\\n%s\\n' "$scope" "$names"
    )"
    ans="$(printf '%s' "$prompt" | "$here/wt-omni.sh")" || exit 1
    sections+="## T${ticket} (${repo})"$'\\n'"${ans}"$'\\n\\n'
  done < <(git -C "$root" worktree list --porcelain | awk '/^worktree /{print substr($0,10)}')
done < <(wt_repo_keys)

[[ -n "${sections//[$' \\t\\r\\n']/}" ]] || { echo "drift: nothing to write" >&2; exit 1; }
printf '%s' "$sections" | wt_atomic_write "$(wt_status_dir)/DRIFT.md"
