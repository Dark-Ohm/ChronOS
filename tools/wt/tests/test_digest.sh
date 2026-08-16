#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"

scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT
cat >"$scratch/STATUS.md" <<'MD'
# worktree status
- path: /x
  name: demo
MD
cat >"$scratch/DRIFT.md" <<'MD'
# Drift
- none
MD

fake="$(mktemp)"
printf '#!/bin/sh\ncat >/dev/null\necho digest-ok\n' >"$fake"
chmod +x "$fake"

out="$(WT_STATUS_DIR="$scratch" WT_OMNI_CURL="$fake" \
  bash "$HERE/../wt-digest.sh")"
assert_eq "$(cat "$scratch/DIGEST.md")" "digest-ok"

# missing input → exit 1 without calling omni
rm -f "$scratch/DRIFT.md"
if WT_STATUS_DIR="$scratch" WT_OMNI_CURL="$fake" \
  bash "$HERE/../wt-digest.sh" 2>/dev/null; then
  echo FAIL expected exit 1; exit 1
fi
rm -f "$fake"
