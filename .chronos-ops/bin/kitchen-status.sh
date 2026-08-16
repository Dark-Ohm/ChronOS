#!/usr/bin/env bash
# kitchen-status.sh — one-glance queue snapshot of a .chronos-ops kitchen.
# Pure bash, no LLM, no cron. Run from anywhere — resolves the kitchen
# root from this script's own location (.chronos-ops/bin/..), not cwd.
#
# Writes checkpoint/STATUS.md (atomic tmp+mv) and prints the same to stdout.
# Counts only ticket files (T<digits>-*.md). Role pointer files
# (BACKEND.md, FRONTEND.md, QA.md, RECON.md, DESIGN.md, ...) are excluded
# by construction — the T-prefix glob never matches them.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

ticket_files() {
  # $1 = directory; prints basenames of T###-*.md files in it, one per line
  local dir="$1"
  [[ -d "$dir" ]] || return 0
  local f
  for f in "$dir"/T[0-9]*.md; do
    [[ -e "$f" ]] || continue
    basename "$f"
  done
}

count() { ticket_files "$1" | grep -c . || true; }

roles() {
  find "$root/active" -mindepth 1 -maxdepth 1 -type d ! -name hold -exec basename {} \; | sort
}

atomic_write() {
  local dest="$1" dir tmp
  dir="$(dirname "$dest")"
  mkdir -p "$dir"
  tmp="$(mktemp "$dir/.tmp.XXXXXX")"
  cat >"$tmp"
  mv -f "$tmp" "$dest"
}

{
  printf '# kitchen status\n'
  printf 'generated: %s\n\n' "$(date -Iseconds)"

  printf '## active — by role\n\n'
  while IFS= read -r role; do
    [[ -n "$role" ]] || continue
    n="$(count "$root/active/$role")"
    printf '### %s (%s)\n' "$role" "$n"
    ticket_files "$root/active/$role" | sed 's/^/- /'
    printf '\n'
  done < <(roles)

  hn="$(count "$root/active/hold")"
  printf '## hold (%s)\n\n' "$hn"
  ticket_files "$root/active/hold" | sed 's/^/- /'
  printf '\n'

  rf="$(count "$root/reports-fresh")"
  printf '## reports-fresh — inbox, awaiting triage (%s)\n\n' "$rf"
  ticket_files "$root/reports-fresh" | sed 's/^/- /'
  printf '\n'

  printf '## rework — by role\n\n'
  while IFS= read -r role; do
    [[ -n "$role" ]] || continue
    n="$(count "$root/rework/$role")"
    printf -- '- %s: %s\n' "$role" "$n"
  done < <(roles)
  printf '\n'

  printf '## reject — by role\n\n'
  while IFS= read -r role; do
    [[ -n "$role" ]] || continue
    n="$(count "$root/reject/$role")"
    printf -- '- %s: %s\n' "$role" "$n"
  done < <(roles)
  printf '\n'

  printf '## done / reports-log — by role (accepted totals)\n\n'
  while IFS= read -r role; do
    [[ -n "$role" ]] || continue
    dn="$(count "$root/done/$role")"
    rn="$(count "$root/reports-log/$role")"
    printf -- '- %s: done=%s reports-log=%s\n' "$role" "$dn" "$rn"
  done < <(roles)
} | atomic_write "$root/checkpoint/STATUS.md"

cat "$root/checkpoint/STATUS.md"
