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
