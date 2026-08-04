#!/usr/bin/env bash
# check-proofs.sh — validate every `file:line` proof reference in the
# chronos-gpui eval files.
#
# Each eval item must carry evidence from the fork sources (`../Source/`) or
# the repo (`crates/…`). A proof is valid when the file resolves AND the
# referenced line range exists and is non-empty. This keeps "answer from the
# skill, proof from the tree" honest and reproducible — run it after any
# SKILL.md / eval edit, before committing.
#
# Usage:  ./check-proofs.sh [eval-file …]        (default: all evals/*.eval.md)
# Exit:   0 when every proof resolves, 1 otherwise (broken list on stdout).
#
# Path resolution: the repo root is found via `git rev-parse`; the gpui fork
# is assumed to be the *sibling* `../Source` of the repo (ChronOS worktree
# rule — see skills/chronos-shell/SKILL.md). Runs from any cwd.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/../..")"
SRC="$(dirname "$REPO")/Source"
[ -d "$SRC" ] || SRC="$REPO/../Source"

if [ "$#" -gt 0 ]; then
  EVALS=("$@")
else
  EVALS=("$SCRIPT_DIR"/*.eval.md)
fi

python3 - "$SRC" "$REPO" "${EVALS[@]}" <<'PYEOF'
import re, os, sys

SRC, REPO, EVALS = sys.argv[1], sys.argv[2], sys.argv[3:]

# Candidate prefixes for short refs like `div.rs:4063` or `grid_cols` docs.
PREFIXES = [
    'gpui/src/elements/', 'gpui/src/platform/', 'gpui/src/app/',
    'gpui/src/', 'gpui/examples/', 'gpui/',
    'gpui_macros/src/', 'gpui_linux/src/', 'gpui_scheduler/src/', 'gpui_tokio/src/',
    'gpui_platform/src/',
    'crates/app/', 'crates/ui/src/', 'crates/services/',
]

def resolve(ref):
    if ref.startswith('Source/'):           # explicit fork path, e.g. Source/gpui/src/window.rs
        cand = os.path.join(SRC, ref[len('Source/'):])
        return cand if os.path.isfile(cand) else None
    if ref.startswith('crates/'):           # explicit repo path, e.g. crates/ui/src/elevation.rs
        cand = os.path.join(REPO, ref)
        return cand if os.path.isfile(cand) else None
    for pfx in PREFIXES + ['']:             # short refs under the fork / repo
        root = REPO if pfx.startswith('crates') else SRC
        cand = os.path.join(root, pfx + ref)
        if os.path.isfile(cand):
            return cand
    return None

pat = re.compile(r'`([^`]+?):(\d+)(?:-(\d+))?`')
ok = bad = 0
for ef in EVALS:
    if not os.path.isfile(ef):
        print(f'MISS-EVAL-FILE {ef}')
        bad += 1
        continue
    lines = open(ef).read().split('\n')
    for ln, line in enumerate(lines, 1):
        for m in pat.finditer(line):
            ref, start, end = m.group(1), int(m.group(2)), int(m.group(3) or m.group(2))
            if ':line' in ref:              # ``file.rs:line:NN`` prose style, skip
                continue
            found = resolve(ref)
            if found is None:
                print(f'MISS-FILE {os.path.basename(ef)}:{ln}: {ref}')
                bad += 1
                continue
            fl = open(found).read().split('\n')
            if end > len(fl) or fl[start-1].strip() == '':
                print(f'BAD-LINE {os.path.basename(ef)}:{ln}: {ref}:{start}-{end} '
                      f'-> {os.path.relpath(found, SRC)} ({len(fl)} lines)')
                bad += 1
                continue
            ok += 1

print('=' * 66)
print(f'proofs checked: {ok}, BROKEN: {bad}')
sys.exit(1 if bad else 0)
PYEOF
