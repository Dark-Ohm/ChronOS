#!/usr/bin/env bash
# check-proofs.sh — validate every `file:line` proof reference across the
# repo's skill vault.
#
# Skills are supposed to carry evidence ("answer from the skill, proof from
# the tree"), so a proof is valid only when its `file:line` (or `file:start-end`)
# reference resolves to an existing file whose line range exists and is
# non-empty. This keeps the vault honest and reproducible — run after any
# SKILL.md / reference / eval edit, before committing.
#
# Scope (default): every `SKILL.md`, every `references/*.md`, every
# `*.eval.md` under `skills/` (excluding .obsidian / _notes / assets / eval).
# Pass explicit files to check a subset.
#
# Usage:  ./skills/check-proofs.sh [file …]
# Exit:   0 when every proof resolves, 1 otherwise (broken list on stdout).
#
# Path resolution: repo root via `git rev-parse`; the gpui fork is assumed to
# be the *sibling* `../Source` of the repo (ChronOS worktree rule — see
# skills/chronos-shell/SKILL.md). Runs from any cwd. Notes:
#   - `Source/…` refs resolve against the fork root.
#   - `crates/…`, `docs/…`, `packaging/…`, `scripts/…`, `skills/…`,
#     `reference/…` (donor/upstream snapshots) resolve against the repo root.
#   - Short refs (e.g. `div.rs:1429`) are tried against known fork/repo
#     prefixes in order; the first hit wins.
#   - References to OUT-OF-TREE code (e.g. Zed-upstream paths in `zed-*`
#     skills) legitimately do not resolve here — the script reports them as
#     MISS-FILE, which is the expected signal for "this points outside our
#     tree", not necessarily a defect.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/..")"
SRC="$(dirname "$REPO")/Source"
[ -d "$SRC" ] || SRC="$REPO/../Source"

if [ "$#" -gt 0 ]; then
  FILES=("$@")
else
  mapfile -t FILES < <(
    find "$REPO/skills" \
      \( -name 'SKILL.md' -o -name '*.eval.md' -o -path '*/references/*.md' \) \
      -not -path '*/.obsidian/*' -not -path '*/_notes/*' -not -path '*/assets/*' \
      -not -path '*/eval/*' -not -path '*/.git/*' \
      | sort
  )
fi

python3 - "$SRC" "$REPO" "${FILES[@]}" <<'PYEOF'
import re, os, sys

SRC, REPO, FILES = sys.argv[1], sys.argv[2], sys.argv[3:]

# Short-ref prefixes, fork-first. `crates/…`-style prefixes come last because
# full-path refs are handled by their own branch below.
PREFIXES = [
    'gpui/src/elements/', 'gpui/src/platform/', 'gpui/src/app/',
    'gpui/src/', 'gpui/examples/', 'gpui/',
    'gpui_macros/src/', 'gpui_linux/src/', 'gpui_scheduler/src/', 'gpui_tokio/src/',
    'gpui_platform/src/',
    # ChronOS app crates — short refs like `volume_popup/view.rs:199`
    'crates/app/src/volume_popup/', 'crates/app/src/side_panel_right/',
    'crates/app/src/side_panel_left/', 'crates/app/src/desktop_terminal/',
    'crates/app/src/launcher/', 'crates/app/src/notifications/',
    'crates/app/src/osd/', 'crates/app/src/bar/', 'crates/app/src/ipc/',
    'crates/app/src/', 'crates/ui/', 'crates/services/', 'crates/',
    'docs/', 'packaging/', 'scripts/', 'skills/', 'reference/',
]

def resolve(ref):
    if ref.startswith('Source/'):          # explicit fork path
        cand = os.path.join(SRC, ref[len('Source/'):])
        return cand if os.path.isfile(cand) else None
    if re.match(r'^(crates|docs|packaging|scripts|skills|reference)/', ref):  # explicit repo path
        cand = os.path.join(REPO, ref)
        return cand if os.path.isfile(cand) else None
    for pfx in PREFIXES + ['']:
        root = REPO if pfx.startswith(('crates', 'docs', 'packaging', 'scripts', 'skills', 'reference')) else SRC
        cand = os.path.join(root, pfx + ref)
        if os.path.isfile(cand):
            return cand
    return None

pat = re.compile(r'`([^`]+?):(\d+)(?:-(\d+))?`')
ok = bad = 0
for f in FILES:
    if not os.path.isfile(f):
        print(f'MISS-EVAL-FILE {f}')
        bad += 1
        continue
    lines = open(f).read().split('\n')
    for ln, line in enumerate(lines, 1):
        for m in pat.finditer(line):
            ref, start, end = m.group(1), int(m.group(2)), int(m.group(3) or m.group(2))
            if ':line' in ref:             # ``file.rs:line:NN`` prose style, skip
                continue
            if not re.search(r'[/.]', ref):  # not path-like (e.g. CSS `flex:1`), skip
                continue
            found = resolve(ref)
            if found is None:
                print(f'MISS-FILE {os.path.relpath(f, REPO)}:{ln}: {ref}')
                bad += 1
                continue
            fl = open(found).read().split('\n')
            if end > len(fl) or fl[start-1].strip() == '':
                print(f'BAD-LINE {os.path.relpath(f, REPO)}:{ln}: {ref}:{start}-{end} '
                      f'-> {os.path.relpath(found, SRC)} ({len(fl)} lines)')
                bad += 1
                continue
            ok += 1

print('=' * 66)
print(f'files: {len(FILES)}, proofs checked: {ok}, BROKEN: {bad}')
sys.exit(1 if bad else 0)
PYEOF
