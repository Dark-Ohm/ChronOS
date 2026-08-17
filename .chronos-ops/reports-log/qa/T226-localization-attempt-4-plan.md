# T226 — Localization Attempt #4 Plan (2026-08-04)

**Status:** ready for execution — binary built, scripts prepared, user restarts ChronOS when ready.

## Infrastructure gaps closed (since attempt #3)

| Gap | Status | Commit |
|-----|--------|--------|
| `expand-left` IPC (open left panel, dock chat, focus composer) | ✅ implemented | unstaged diff |
| `select-tab:<id>` IPC (switch right panel to tab) | ✅ implemented | unstaged diff |
| `preview-target:<path>` IPC (set PreviewTarget, switch to Editor) | ✅ implemented | unstaged diff |
| `wtype` confirmed working for Wayland input | ✅ tool available | — |
| `wf-recorder` confirmed working for video clips | ✅ tool available | — |
| `grim -g "X,Y WxH"` with exact layer geometry | ✅ methodology | — |

**Release binary:** `target/release/chronos` — 26 MB, stripped, built 2026-08-04 15:21, compiles clean.

**Socket path:** `$XDG_RUNTIME_DIR/chronos.sock` (typically `/run/user/1000/chronos.sock`).

## Known limitation: Editor tab keyboard focus

`active_tab_focus()` in `view.rs:453-465` only returns a `FocusHandle` for `TabContent::Terminal`. For `Preview`/Editor, it returns `None`. This means:

- **Terminal tab**: `select-tab:terminal` → 50ms deferred `window.focus()` → `wtype` lands in PTY ✅
- **Editor tab**: `select-tab:preview` → panel opens, file loads, but **no keyboard focus** → `wtype` goes to the previously focused surface ❌

To test the Editor tab, one of:
- Click into the editor area manually (mouse) before `wtype`
- Or add a `FocusHandle` return for `TabContent::Preview` in `active_tab_focus()` — but this requires knowing which internal widget to focus (Edit mode's Input vs View mode's passive surface)

**Recommendation:** run Phase 1 (composer) and Phase 2 (terminal) first. If the bug reproduces in either, localization is done — the Editor phase becomes optional. If not, expand `active_tab_focus` for Preview or use manual click.

## Execution script

**Script:** `/tmp/t226-localize-4.sh`
**Test file:** `/tmp/t226-test-file.md` (42 lines, markdown with code blocks for gutter visibility)

### Pre-conditions
1. Restart ChronOS with `target/release/chronos` (the new binary)
2. Verify socket: `test -S $XDG_RUNTIME_DIR/chronos.sock && echo "ready"`
3. For terminal capture: switch to an empty workspace on DP-1 (the `desktop-terminal` background layer is covered by any window above it — but the **right-panel Terminal tab** is a separate surface at overlay level, so this may not be needed for the tab test)

### Phase 1: Composer (left panel)
```bash
echo -n "expand-left" | nc -U $XDG_RUNTIME_DIR/chronos.sock
# Panel expands, composer gets focus
# wf-recorder captures the whole side_panel_left surface
# wtype types: 123abc, abc123, 1a2b3c (EN + RU)
```

### Phase 2: Terminal (right panel)
```bash
echo -n "select-tab:terminal" | nc -U $XDG_RUNTIME_DIR/chronos.sock
# Panel opens at Terminal's preferred width (560)
# 50ms deferred focus lands on PTY
# wtype types into the terminal
```

### Phase 3: Editor (right panel) — LIMITED
```bash
echo -n "preview-target:/tmp/t226-test-file.md" | nc -U $XDG_RUNTIME_DIR/chronos.sock
echo -n "select-tab:preview" | nc -U $XDG_RUNTIME_DIR/chronos.sock
# Panel opens with file in View mode
# ⚠️ No keyboard focus — wtype won't land here without manual click
```

### Output
All artifacts in `/tmp/t226-attempt4/`:
- `composer-en.mp4`, `composer-ru.mp4` — video clips
- `composer-en-123abc.png`, `composer-ru-123abc.png` — static frames
- `terminal-en.mp4`, `terminal-en-123abc.png`
- `editor-en.mp4`, `editor-ru.mp4`, `editor-en-123abc.png`, `editor-ru-123abc.png`

## What changed from attempt #3

| Problem | Fix |
|---------|-----|
| No way to expand left panel with composer focused | `expand-left` IPC |
| No way to set PreviewTarget | `preview-target:<path>` IPC |
| No way to switch tabs | `select-tab:<id>` IPC |
| `ydotool click` erratic on layer-shell | Use `wtype` for keyboard input |
| `grim` captured wrong window | Use `hyprctl layers` for exact geometry |
| Static frames miss temporal bug | `wf-recorder` video clips during typing |
| Terminal capture got covered windows | Right-panel Terminal tab is overlay-level, not background |

## Next

User restarts ChronOS → runs `/tmp/t226-localize-4.sh` → reviews clips for digits vanishing → reports which location(s) reproduce.
