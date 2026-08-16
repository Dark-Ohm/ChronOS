#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/helpers.sh"
OMNI="$HERE/../wt-omni.sh"

fake="$(mktemp)"
printf '#!/bin/sh\ncat >/dev/null\necho fake-ok\n' >"$fake"
chmod +x "$fake"
out="$(printf 'hi' | WT_OMNI_CURL="$fake" "$OMNI")"
assert_eq "$out" "fake-ok"

printf '#!/bin/sh\ncat >/dev/null\n' >"$fake"
if printf 'hi' | WT_OMNI_CURL="$fake" "$OMNI"; then
  echo FAIL expected empty fake to fail; exit 1
fi

# connection refused must exit 1 and not hang (timeout 2s)
if printf 'hi' | env -u WT_OMNI_CURL \
     WT_OMNI_URL=http://127.0.0.1:1/v1/chat/completions \
     WT_OMNI_MODEL=cron \
     WT_OMNI_TIMEOUT=2 \
     "$OMNI"; then
  echo FAIL expected refused; exit 1
fi
rm -f "$fake"
