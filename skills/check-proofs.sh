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
#   - `Source-wt-component/…` refs resolve against the sibling gpui-component
#     worktree (Longbridge toolkit wired into ChronOS).
#   - Short refs (e.g. `div.rs:1429`) are tried against known fork/repo
#     prefixes in order; the first hit wins.
#   - References to OUT-OF-TREE code (Zed upstream, Hermes checkout, philip,
#     fable worked-examples, writing-plans placeholders) legitimately do not
#     resolve here — the EXTERNAL allowlist below reports them as `EXT`
#     (informational) and does NOT fail the run. A `MISS-FILE`/`BAD-LINE` is
#     therefore a genuine defect: a stale line or a path that should resolve
#     against one of the known roots.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/..")"
# The gpui fork is a *sibling* worktree on dev machines. CI has no fork, so
# `CHECK_PROOFS_SRC` lets a job point elsewhere (or at nothing). When the fork
# is absent, fork-only references degrade to EXT (informational) below.
SRC="${CHECK_PROOFS_SRC:-$(dirname "$REPO")/Source}"

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

# Environment degradation (CI): some roots are simply not present on the
# runner, and references to them degrade to informational EXT instead of
# failing — repo-local proofs stay strict.
#   - SRC (the gpui fork `../Source`): when missing, any ref that is NOT
#     repo-local (i.e. not an explicit `crates/…`/`docs/…`/`packaging/…`/
#     `scripts/…`/`skills/…`/`reference/…` path) is fork-style and degrades.
#   - `reference/` (donor/upstream snapshots) is gitignored and never
#     committed — absent in a fresh CI checkout.
#   - `Source-wt-component/` (gpui-component worktree) is a dev-machine
#     sibling — absent in CI.
FORK_MISSING = not os.path.isdir(SRC)
REPO_PREFIX_RE = re.compile(r'^(crates|docs|packaging|scripts|skills|reference)/')

def is_fork_style(ref):
    return not REPO_PREFIX_RE.match(ref)

def env_missing(ref):
    if ref.startswith('reference/'):
        return not os.path.isdir(os.path.join(REPO, 'reference'))
    if ref.startswith('Source-wt-component/'):
        return not os.path.isdir(os.path.join(os.path.dirname(REPO), 'Source-wt-component'))
    return False

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
    if ref.startswith('Source-wt-component/'):  # gpui-component worktree (sibling of Source/)
        cand = os.path.join(os.path.dirname(REPO), ref)
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

# Known-external references — legitimately do not resolve in this tree, but
# are NOT defects. Reported as `EXT` (informational); never fail the run.
# Grouped by the external tree each belongs to:
#   - Zed upstream source (documentation-investigation/zed-ai-assistant-*):
#     crates/agent, crates/agent_ui, crates/language_model(_core), crates/acp_thread
#   - Hermes checkout (~/.hermes/hermes-agent): agent/tool_executor.py,
#     acp_adapter/, server.py
#   - philip project checkout: trajectory_compressor.py, registry.py,
#     agent/context_compressor.py
#   - fable-method worked examples (fictional files): useDashboard.ts, dates.ts
#   - writing-plans templates/placeholders: exact/path/to/…,
#     docs/brief/IMPLEMENTATION-PLAN.md
EXTERNAL = [
    'crates/agent/', 'crates/agent_ui/', 'crates/language_model/',
    'crates/language_model_core/', 'crates/acp_thread/',
    'agent/tool_executor.py', 'agent/context_compressor.py', 'acp_adapter/',
    'server.py', 'trajectory_compressor.py', 'registry.py',
    'useDashboard.ts', 'dates.ts',
    'exact/path/to/', 'docs/brief/IMPLEMENTATION-PLAN.md',
]

def is_external(ref):
    return any(ref.startswith(p) or ref == p for p in EXTERNAL)

pat = re.compile(r'`([^`]+?):(\d+)(?:-(\d+))?`')
ok = bad = ext = 0
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
                if is_external(ref):
                    print(f'EXT   {os.path.relpath(f, REPO)}:{ln}: {ref}')
                    ext += 1
                    continue
                if env_missing(ref):
                    print(f'EXT(env-missing) {os.path.relpath(f, REPO)}:{ln}: {ref}')
                    ext += 1
                    continue
                if FORK_MISSING and is_fork_style(ref):
                    print(f'EXT(fork-missing) {os.path.relpath(f, REPO)}:{ln}: {ref}')
                    ext += 1
                    continue
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
print(f'files: {len(FILES)}, proofs checked: {ok}, external (by design): {ext}, BROKEN: {bad}')
sys.exit(1 if bad else 0)
PYEOF
