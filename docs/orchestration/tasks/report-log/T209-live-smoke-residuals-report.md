# T209 report — live smoke residuals

**Binary:** `target/release/chronos` · mtime `2026-08-03 01:29:42` ·
`git rev-parse --short HEAD` = **`9435cc0`** (code tree clean; only docs dirty).
**Env:** DP-1 2560×1440 (pult) + HDMI-A-1 1920×1200 · Hyprland **0.56.1** ·
Hermes ACP **yes** (connected, live turns executed) · ydotool **yes**
(absolute = screen/2 confirmed; note ydotool warns "disable mouse speed
acceleration", first calls drifted, later calls landed exactly — every click
in this run is verified by `hyprctl cursorpos` printed next to it).
**Artifacts:** `/tmp/t209-smoke/20260803-0250/` (grim PNGs + `chronos.log`,
`chronos2.log`, `chronos3.log`).

**Verdict: FAIL** — three P0 failures (S2, F1, and a new right-panel
unreachable-after-drag regression). Everything else PASSes, most of it cleanly.

---

## Matrix

| ID | result | evidence |
|---|---|---|
| P0–P6 | PASS | HEAD `9435cc0`; binary newer than HEAD; old shell killed; started `RUST_LOG=info`; `hyprctl layers` shows `bar` + `side_panel_hover_strip` on DP-1 |
| **B1** height live | PASS | `[appearance] height 30→44` via `bar.toml` → layer `bar 0,0 2560x44`, windows repushed to y=44, **no** recreate (resize path). grim `B1-height44.png` |
| **B2** exclusive+floating | PASS *(+note)* | log `bar: appearance floating=true forces exclusive=false`; bar to `0,10 2560x40`, clients back to y=10. grim `B2-floating.png` · **note:** `margin.x=16` has **no** horizontal effect on a full-width floating bar (layer stays `x=0 w=2560`); `margin.y` works |
| **B3** presets | PASS | clicked "Bottom full" chip in System settings → `bar 0,1410`, app rewrote `bar.toml` as v2 with sorted keys; "Top full" restored. grim `B3-presets.png`, `B3-after-preset.png` |
| **B4** edge bottom | PASS | `bar 384,1400 1792x40`, one recreate ≈130 ms, exactly one `bar` layer (no ghost). grim `B4-edge-bottom.png` |
| **B5** fraction width | PASS | `width="fraction:0.7"` + center → `bar 384,0 1792x40` = 2560×0.7 centred, radius 12 pill visible, no compositor stretch. grim `B5-fraction70.png` |
| **B6** cold-start | PASS w/ residual | boot log: exactly one `bar: closed for recreate` → `bar: recreated window …` — the documented T207 single flicker, no loop |
| **B7/B8** agent tools / dogfood | SKIP | agent-side path (agent edits `bar.toml` via its own file tools); the shell-side apply it depends on is already proven by B1–B5 |
| **R1** rail-only, no gray lip | PASS | rail `2520,30 40x1410`, dark, flush to screen edge, no gray/white lip. grim `R1-rail-only.png`, `R6-rail-only-chrome.png` |
| **R2** expand via handle | PASS w/ residual | log `handle grab expanded rail → content width=400.0`; drag then tracks monotonically. **Residual worse than documented:** the edge follows the pointer at ≈**half** rate (ptr Δ200 → edge Δ105), plus the ~360 px offset from the rail→content snap. Never converges under the cursor. grim `R2-mid-drag.png`, `R2-drag-expanded.png` |
| **R3** shrink | PASS w/ residual | ptr 1834→2540, edge 1834→2196, monotonic, no jump, no stuck — same half-rate. grim `R3-shrunk.png` |
| **R4** one-frame jank | PASS | no continuous thrash observed across ~15 drag samples |
| **R5** left panel resize | SKIP | not exercised (left panel used only for Follow) |
| **R6** hairline only when open | PASS | rail-only has no border chrome; open panel shows the hairline. grim pair above |
| **R7** *(new, unplanned)* | **FAIL P0** | see Failures §1 |
| **E1** md View | PASS | `preview: loaded kind=Markdown … README.md`, rendered, default **View** not Edit; **T180 network guard holds** — badges render as "remote image, not loaded", zero fetches. grim `E1-md-view.png` |
| **E2** Preview\|Edit only md | PASS | md rows show View+Edit in Files; `.toml/.lock/LICENSE/NOTICE` show none; `.toml` opened via ACP tab logs `Edit intent on non-editable kind, forcing View` |
| **E3** themed buffer | PASS | dark buffer, no white glare. grim `E3-edit-mode.png` |
| **E4** gutter | PASS | 1-based line numbers in gutter, same frame |
| **E5** Ln/Col live | PASS | click → `Ln 30, Col 14`; arrows → `Ln 32, Col 65`, no typing needed (T208 observe errata works). grim `E5-lncol-after-click.png` / `-after-arrows.png` |
| **E6** soft wrap | PASS | Wrap on = wrapped; off = clipped/h-scroll; button state tracks the buffer. grim `E6-wrap-on.png` / `E6-wrap-off.png` |
| **E7** Save/dirty | PASS | typed char → `• unsaved` + blue **Save**; click Save → log `editor: saved path=…/README.md`, mtime bumped, `git status` clean (byte-identical, undo had restored content). grim `E7-dirty.png`, `E7-after-save.png` |
| **E8** terminal drawer | PASS | drawer opens under editor, `terminal: shell spawned … grid reconciled cols=64 rows=10`; typed text reached the PTY and Enter executed (zsh replied). grim `E8-terminal-drawer.png`, `E8-terminal-input.png` |
| **E9** light theme buffer | PASS w/ note | readable, not inverted — **but** panel chrome goes light while the **editor buffer stays dark** and the **rail stays dark**. grim `E9-light-files2.png`, `E9-light-edit.png` |
| **F1** Follow toggle UI | **FAIL P0** | see Failures §2 |
| **F2** Follow ON opens path | PASS | Hermes `write_file /tmp/t209-follow-probe.txt` → auto-approve → right panel loads that exact path within 400 ms (`preview: loaded … path=/tmp/t209-follow-probe.txt`). grim `F2-follow-editor.png` |
| **F3** Follow OFF quiet | PASS | Follow off, agent wrote `/tmp/t209-follow-off.txt`, tool Completed, **no** `preview: loaded` — right panel did not move |
| **F4** clear last_tool | SKIP | no observable signal (no log, no strip UI) |
| **F5** activity strip | SKIP | documented deferred — absent as expected |
| **S1** System settings | PASS | Bar page with live `[appearance] · top · 30px`, Presets, Edge/Height/Width/Floating/Radius/Elevation/Exclusive, Theme, Hypr modules, About. grim `S1-settings.png` |
| **S2** theme toggle | **FAIL P0** | see Failures §3 |
| **S3** hypr modules | PASS | 8 real files listed; click `00-monitors` → `preview: loaded … /home/neo/.config/hypr/modules/00-monitors.lua` |
| **S4** About version | PASS | UI `0.1.0` == `crates/app/Cargo.toml` `version = "0.1.0"` |
| **S5** ACP agents list | PASS | "1 agent(s) · agents.toml", Hermes + `built-in` badge, `hermes acp --accept-hooks`. grim `S5-acp-agents.png` |
| **S6** Open agents.toml | PASS w/ note + sub-FAIL | with the file present: correct path opens (forced View, `.toml` not editable — so "Edit agents.toml to add/remove agents" is a **dead end in-app**). **Sub-FAIL:** when the file does **not** exist, the Editor renders a fully blank surface — no path, no buttons, no error. grim `S6-open-agents-toml.png` (blank), `S6b-agents-toml-editor.png` (ok) |
| **S7** reload after edit | **FAIL** | added a `[[agents]]` stub on disk → list still "1 agent(s)"; no Reload control exists anywhere in the tab. After a shell restart it reads correctly ("2 agent(s)", `T209 Probe` + command). So parsing is fine, **live reload is missing**. grim `S7-acp-after-add.png`, `S7b-acp-after-restart.png` |
| **S8** inline CRUD | SKIP | documented deferred — absent as expected |
| **X1** agent chat send | PASS | `composer: send …` → `ACP streaming reply complete`, no CPU pin |
| **X2** notifications | PASS | `notify-send` → `notifications` layer appears |
| **X3** no panic | PASS (runs 2–3) / FAIL (run 1) | `chronos2.log`, `chronos3.log`: 0 panics. `chronos.log`: 4 — all from the S2 crash |

---

## Failures (root-cause guess — not fixed here)

### 1. P0 — right panel becomes unreachable via hover after an interrupted handle-drag *(new, T206 area)*

Repro: panel open as **peek** (hover, not pinned) → grab the 4 px handle →
drag left past the panel's own edge → the peek-leave logic closes the panel
mid-drag (`side_panel_right: closed`, 0.75 s after `handle grab expanded`).
From that moment the hover strip **never opens the panel again** — cursor
parked exactly on `side_panel_hover_strip 2556,30 4x1410`, verified by
`hyprctl cursorpos` (2558,800 / 2558,1000), slow multi-step approach, and a
neutral click elsewhere to clear any stuck implicit grab. **Zero** new log
lines on every attempt.

Not a stale handle: `open_window` early-returns when `handle.is_some()` and
would log nothing — but the IPC path `toggle-side-panel-right` **does** open
the panel immediately afterwards (`opened (pinned)`), which proves
`state.handle` was correctly cleared by `close()`. So the dead component is
the **hover strip's enter handler**, not the panel state machine. Two bugs
really: (a) an active handle-drag must suppress peek-close; (b) whatever the
strip loses on that close path never comes back.

Impact: for a user who never binds the IPC toggle, the right panel is gone
until the shell restarts.

### 2. P0 — Follow toggle has literally no visual state

`magick compare -metric AE` between the ON and OFF frames (cursor moved away,
no hover): **0 differing pixels**. Root cause is visible in the source —
`crates/app/src/side_panel_left/panel.rs:236-252` renders the control as the
**color-emoji glyph `👁`** and switches `text_color` between
`theme.accent.primary` and `theme.text.muted`. `text_color` does not affect a
color-bitmap emoji glyph, so the accent/muted distinction cannot render. No
log line either. The state itself flips correctly — F2/F3 prove the behaviour
follows the toggle; only the affordance is invisible.

### 3. P0 — Theme "Toggle" in System settings crashes the whole shell

```
thread 'main' panicked at Source/gpui/src/app.rs:1872:32:
no state of type chronos_ui::theme::Theme exists
thread 'main' panicked at Source/gpui_linux/src/linux/wayland/client.rs:336:14:
The pointer should always be valid when dispatching in wayland
panic in a destructor during cleanup — thread caused non-unwinding panic. aborting.
```

`theme.toml` **is** written first (`scheme = "Default"` → `"Light"`), then the
process aborts; bar, both panels and the dock die with it. Writing the same
value into `theme.toml` by hand hot-reloads cleanly with no crash, so theme
*application* is fine — the toggle handler reads/updates the `Theme` global
from a context where it isn't registered (side-panel window vs the app-level
global). `let _ =`-free code did not help here because it is a `global()`
panic, not a swallowed `Err`.

### 4. Non-blocking, but real

- **S7:** no reload control for `agents.toml`; edits need a shell restart.
- **S6:** opening a missing file yields a completely blank Editor surface.
- **S6/E2:** `.toml` is non-editable, so the ACP tab's own "edit agents.toml"
  instruction cannot be carried out inside ChronOS.
- **E9:** light theme is partial — chrome light, editor buffer and rail dark.
- **B2:** `appearance.margin.x` is a no-op for a full-width floating bar.
- **S1/B3:** at the 320 px settings width the Presets row and several controls
  are clipped by the panel edge with no scroll affordance.

---

## Residuals confirmed still true (allowed, per spec §6)

| residual | status |
|---|---|
| Cold-start single bar recreate (T207) | confirmed, exactly one |
| No activity strip UI (T195) | confirmed absent |
| No inline ACP add/remove (T196) | confirmed absent |
| Recreate flash instead of live `set_anchor` (T207) | confirmed, ≈130 ms |
| Highlight current line deferred (T205) | confirmed absent |
| start_x offset after expand (T206) | confirmed — **and worse**: ≈half-rate tracking, see R2 |

---

## Environment restored after the run

`bar.toml` restored from backup (v1, byte-identical to pre-run);
`theme.toml` back to `Default`; probe `agents.toml` and `/tmp/t209-*` files
removed; both side panels closed; keyboard layout switched back to Russian;
`README.md` untouched on disk (`git status` clean). Shell left running on
`chronos3.log` with 0 panics.

**Tooling note:** `lean-ctx allow` was extended additively with `socat`,
`magick`, `notify-send` (config `~/.config/lean-ctx/config.toml`).

---

## Next

Three thin fix tasks, none of them "re-accept the old T-ID as done live":

1. **T210** — peek-close must not fire during an active handle-drag; hover
   strip must survive a close that happens mid-drag (P0, T206 area).
2. **T211** — theme toggle context fix (P0, T196 area) + Follow toggle
   affordance: drop the emoji for a tintable icon (P0, T195 area).
3. **T212** — settings surface honesty: `agents.toml` reload control, blank-
   editor-on-missing-file, `.toml` editability, light-theme buffer/rail.

Panel drag half-rate tracking (R2/R3) rides along with T210.
