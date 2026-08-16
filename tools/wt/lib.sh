#!/usr/bin/env bash
# Shared helpers for ChronOS/tools/wt. Source only.

wt_lib_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${WT_REPOS_YAML:=$wt_lib_dir/repos.yaml}"
: "${WT_STATUS_DIR:=/home/neo/projects/chronos-ecosystem/.wt-status}"
: "${WT_OMNI_URL:=http://127.0.0.1:20128/v1/chat/completions}"
: "${WT_OMNI_MODEL:=cron}"

wt_status_dir() { printf '%s\n' "$WT_STATUS_DIR"; }

wt_repo_keys() {
  awk '
    /^repos:[[:space:]]*$/ { in_repos=1; next }
    in_repos && /^  [A-Za-z0-9_.-]+:[[:space:]]*$/ {
      k=$1; sub(/:$/, "", k); print k
    }
  ' "$WT_REPOS_YAML"
}

wt_repo_get() {
  local repo="$1" field="$2"
  local val
  val="$(
    awk -v repo="$repo" -v field="$field" '
      /^  [A-Za-z0-9_.-]+:[[:space:]]*$/ {
        k=$1; sub(/:$/, "", k); cur=k
      }
      cur==repo && $1==field":" {
        $1=""; sub(/^[[:space:]]+/, "", $0)
        print $0
        found=1
        exit
      }
      END { if (!found) exit 2 }
    ' "$WT_REPOS_YAML"
  )" || return 2
  if [[ "$val" == "null" ]]; then
    printf '\n'
    return 0
  fi
  # strip wrapping quotes
  val="${val#\"}"
  val="${val%\"}"
  printf '%s\n' "$val"
}

wt_expand() {
  local pat="$1" ticket="$2" slug="${3:-}"
  local out="$pat"
  out="${out//\{ticket\}/$ticket}"
  out="${out//\{slug\}/$slug}"
  # trailing hyphen if slug empty: feat/t266- → feat/t266
  out="${out%-}"
  printf '%s\n' "$out"
}

wt_atomic_write() {
  local dest="$1"
  local dir tmp
  dir="$(dirname "$dest")"
  mkdir -p "$dir"
  tmp="$(mktemp "$dir/.tmp.XXXXXX")"
  cat >"$tmp"
  mv -f "$tmp" "$dest"
}

wt_is_exception() {
  local repo="$1" name="$2"
  local raw item
  raw="$(wt_repo_get "$repo" exceptions)" || return 1
  raw="${raw#[}"
  raw="${raw%]}"
  IFS=',' read -ra items <<<"$raw"
  for item in "${items[@]}"; do
    item="${item#"${item%%[![:space:]]*}"}"
    item="${item%"${item##*[![:space:]]}"}"
    [[ "$item" == "$name" ]] && return 0
  done
  return 1
}

wt_ticket_from_name() {
  local repo="$1" name="$2"
  local pat ticket bare legacy
  pat="$(wt_repo_get "$repo" name_pattern)"
  pat="${pat//\{ticket\}/__T__}"
  pat="${pat//\{slug\}/}"
  local re
  re="$(printf '%s' "$pat" | python3 -c '
import re,sys
p=sys.stdin.read()
parts=p.split("__T__")
print("".join(re.escape(a)+r"([0-9]+[A-Za-z0-9]*)"*(i<len(parts)-1) for i,a in enumerate(parts)))
')"
  if ticket="$(printf '%s' "$name" | sed -nE "s/^${re}\$/\\1/p")" && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  bare="$(wt_repo_get "$repo" alias_bare)"
  if [[ "$bare" == "true" ]] \
     && ticket="$(printf '%s' "$name" | sed -nE 's/^wt-t([0-9]+[A-Za-z0-9]*)$/\1/p')" \
     && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  legacy="$(wt_repo_get "$repo" alias_legacy)"
  if [[ "$legacy" == "t-slug" ]] \
     && ticket="$(printf '%s' "$name" | sed -nE 's/^t([0-9]+[A-Za-z0-9]*)(-.*)?$/\1/p')" \
     && [[ -n "$ticket" ]]; then
    printf '%s\n' "$ticket"
    return 0
  fi
  printf '\n'
}
