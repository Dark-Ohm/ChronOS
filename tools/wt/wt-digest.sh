#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$here/lib.sh"
st="$(wt_status_dir)"
[[ -f "$st/STATUS.md" && -f "$st/DRIFT.md" ]] || { echo "digest: need STATUS.md and DRIFT.md" >&2; exit 1; }
prompt="$(cat "$here/prompts/digest.txt"; echo; echo '# STATUS'; cat "$st/STATUS.md"; echo; echo '# DRIFT'; cat "$st/DRIFT.md")"
out="$(printf '%s' "$prompt" | "$here/wt-omni.sh")"
[[ -n "${out// }" ]] || { echo "digest: empty model output" >&2; exit 1; }
printf '%s\n' "$out" | wt_atomic_write "$st/DIGEST.md"
