# T323 — Full-surface UI audit (QA) — report

- **Ticket:** T323 (`.chronos-ops/active/qa/T323-full-ui-audit.md`)
- **Role:** QA executor (worker; acceptance is the Architect's)
- **Date:** 2026-08-20/21
- **Binary:** `./target/release/chronos`, pid 3113178 (never restarted), commit `9ffc2eaa`
- **Log:** `/tmp/t323.log`. **Frames:** `/tmp/t323/frames/` (162 PNGs, 49 MB, full-res grim of DP-1). **Config backups:** `/tmp/t323/config-backup/` (sha256-verified restored).
- **Environment caveat:** owner was actively working on HDMI-A-1 throughout. Clicks aimed at DP-1 chrome; a few focus steals and one mpris track-advance were unavoidable side effects of exercising click actions.

## Method

Live mouse via `ydotool` (coords verified with `hyprctl cursorpos`), `grim -o DP-1` frames, layer geometry via `hyprctl layers`, and — after the session's image-read budget was exhausted mid-audit — **quantitative vision**: per-pixel probes / row profiles (`magick txt:`), bright-pixel ratios, `tesseract` OCR attempts, plus log correlation. Every visual claim is backed by a frame on disk; where I could not *see*, I say so and the frame is left for the Architect's eye.

## Findings

### F1 — BAR: workspace dots are click-dead (functional defect, new)
Three precise clicks on dot centers (dots at x≈90-96/102-108/114-120, y≈21; cursor confirmed at 104,20 and 116,20), under both focus conditions (HDMI focused and DP-1 focused): **no workspace change on either monitor**, no dispatch trace in the log. The exact socket line the widget sends (`hl.dsp.focus({ workspace = 2 })`, `hyprland.rs:61`) executed manually switches DP-1 correctly. The compositor path works; the defect is ChronOS-side (handler not firing or dispatch swallowed — note `workspaces.rs` uses `let _ = …dispatch(...)`, the repo's banned pattern). Frames: `07-ws2.png`, `07b-ws-switched.png`, `07c-ws2-manual.png`, `07d-dots-after-manual.png`.

### F2 — BAR: active-dot mapping off (visual, new)
With DP-1 on workspace 2 (switched via socket), the **third** dot lit, not the second (pixel scan: blue at x114-120; ws1 → first dot). Hyprland only has ws 1/11/12, yet the bar shows three dots on DP-1 — the dot list is not the live compositor workspace list, and its active mapping is wrong. Frame `07d-dots-after-manual.png`.

### F3 — THEME: `surface_alpha` applied inconsistently (consistency, new)
alpha 0.7 over white wallpaper: bar blends (`(73,73,83)` at y10) but the wrap plate (`(24,24,37)`) and the volume popup (`(27,27,43)`) stay opaque. Frames `15b-alpha07-bar.png`, `15-alpha07-popup.png`, `15-alpha07-noblur.png`. (Popup-on-light readability is fine precisely because it stays opaque — the finding is the inconsistency.)

### F4 — FRAME: T312 1px seam still alive in `normal` (residual confirmed)
On white wallpaper, `normal` shows a 1px wallpaper hairline under the bar (y41 = `(253,253,253)`); `wrapped` has none. Wrap side gaps: none on white — T309 fix holds. Frames `16-frame-normal.png`, `16-frame-wrapped-white.png`.

### F5 — THEME: schemes render; picker click unconfirmed this session
Hot-reload of `scheme` works; palettes verified by probe: Light `(236,238,250)`, Solarized Dark `(88,110,117)`, Mocha `(24,20,26)`. Light-on-black = bright slab (known TBD critique reproduced, `17-light-on-black.png`). Blind clicks at estimated swatch positions did **not** change `scheme` — I could not see the picker, so picker clickability is *unconfirmed here* (live-verified by the Architect in T313; flag for re-check, not a claimed regression). `toggle-theme` IPC cycles Default↔Light (known limitation). Frames `14-*.png`.

### F6 — INCIDENT (restored): wallpaper daemon died during wallpaper-IPC sequence
`awww-daemon` (owner's wallpaper renderer) was dead after `wallpaper-next` / `wallpaper-refresh` / `wallpaper-gallery` + `waytrogen --restore` (a defunct `waytrogen` zombie remained); DP-1 fell to flat Hyprland bg (`23-final-baseline.png`, std 0.039). I relaunched `awww-daemon` and re-sent the owner's gif (`/home/neo/Pictures/кфт/musely_pixel_art.gif`, recovered from awww's per-output cache) to both outputs; `awww query` confirms both monitors display it (`24-wallpaper-restored.png`, std 0.14). Exact trigger unknown — chronos `wallpaper_ctl` → waytrogen interaction is the suspect; worth a ticket (chronos should not be able to kill the user's wallpaper daemon).

### F7 — LEFT PANEL: opens, rail switches tabs; fresh-shell chat is a blank slab
`toggle-side-panel-left` opens rail+content pinned; `expand-left` upgrades peek→pinned; rail tab select works (log: Chat→Sessions→Chat→Plan). Content paints panel bg; on a fresh shell the transcript area is empty; ~1% bright pixels at top and bottom rows show some chrome renders, but OCR could not read it. Visual quality needs the Architect's eye on `09-left-rail.png` / `10-left-expanded.png`. ACP client connects (log).

### F8 — RIGHT PANEL: all 22 tab IDs switch and paint distinct content
Log-verified per-tab widths (Launcher 410, Media 400, System 400, Updates 420, Notifications 420, Files 440, Editor 560, Terminal 560, Inspector 320, Build 640, Source control 440, Library 480, Scenes 400, Captures 320, ACP 320, MCP 320, LSP 320, API providers 320, System settings 800, Hyprland binds 320, Display 440). Content paints as a right-aligned column inside the fixed 920px canvas; left remainder transparent (T276 design, architect-ruled). Bright-ratio varies 6.9–17.5% per tab → each paints different content. Frames `12-tab-*.png`. Empty-state explanation still absent (TBD critique stands).

### F9 — Popups / tray / toast / OSD
Volume popup opens with slider + device rows (`01-volume-popup.png`). Calendar (`02-calendar.png`), updates (`03-updates.png`), bell toast (`04-toast.png`) open. Tray menu lists 6 items incl. steam/vivaldi/kate (`05-tray.png`); WARN `tray: Activate failed … UnknownMethod` for steam (steam's SNI quirk, minor, log-only). OSD layer appears on audio change (layer evidence).

### F10 — DOCK: hover tooltip OK; right-click menu still absent (T309 residual)
Hover tooltip renders (`06-dock-hover.png`). Right-click on a dock icon produced **no** context menu (log shows `pult display resolved` spam instead) — matches T309's known gap, unchanged at this commit. Left-click launch not tested (owner session).

### F11 — Start menu / launcher / edit mode / workspace modes
Start menu: layer 720x520 at (0,42) + click-catcher (`18-start-menu.png`). Launcher: centered client window 720x560, paints (`18b-launcher2.png`). Edit mode toggles (log + `19-edit-mode.png`). Workspace modes Gamer/Developer switch (log; rail frames `19-gamer-rails.png` / `19-dev-rails.png` left for visual diff).

### F12 — IPC: every command in the brief exercised
All returned success with expected layer/log effect, incl. `preview-target:` (markdown loaded, 6802 B, Preview painted) and `compose-and-send:` (dispatched; hermes stderr `nemo_relay` warnings are agent-backend side, same class T309 ruled not-a-finding).

### F13 — Stability
0 panics, 0 GPUI protocol errors across ~45 min; chronos pid unchanged; git tree clean.

## Not done / honest caveats
- No human-eye verification after mid-audit (image budget exhausted); render claims are pixel/layer/log-based. Frames on disk are the evidence package.
- Not tested: dock left-click launch, keyboard-layout widget click (owner typing), picker swatch click (F5), gaming-mode rail visual diff, alpha+blur on the animated gif wallpaper (flats used instead).
- Owner side effects: one mpris track advance; brief focus steals; wallpaper incident (F6) — fully restored.

## Restoration proof
- `sha256sum -c` on all 11 chronos config files: **OK** (byte-identical).
- Wallpaper: `awww query` shows owner's gif on both outputs.
- Panels closed; `frame.toml` back to `wrapped`; `theme.toml` original (Default, alpha 1.0, blur on).

