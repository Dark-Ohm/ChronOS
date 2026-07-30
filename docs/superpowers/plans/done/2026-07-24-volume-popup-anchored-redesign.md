# Volume Popup — Anchored Redesign Implementation Plan

> **For agentic workers:** REQUIRED workflow: **superpowers:subagent-driven-development**
> (one implementer per task + task review) **or** **superpowers:executing-plans**.
> Checkboxes `- [ ]` track steps.
>
> **ChronOS-specific:** Architect does **not** spawn Claude/Grok subagents.
> "Implementer" = local minion via `docs/orchestration/tasks/active/T121-*.md`
> (**no personal names in the brief**). Report →
> `docs/orchestration/tasks/report/T121-*-report.md`. Named `git add`, no AI
> trailers. Architect accepts after greps / build / live smoke.

**Goal:** Bring `volume_popup` to the same popup discipline as
`updates_popup` (post-T117) and `notifications/history_popup` (T120):
anchored to the bar volume widget, mockup-faithful chrome from
`docs/design/Volume Popup.dc.html` (dark + light C), interactive volume/mic
sliders (not fill-only ±5% rows), footer dual mute, device menus as in
mockup.

**Architecture:** Copy anchor lifecycle from updates; redesign view with
**rsx chrome + builder live controls** (gpui-rsx consumer skill). Audio
backend already has volume/mute/default-device — prefer UI work, not new
service surface unless a gap appears.

**Tech stack / fork facts (gpui-fork-start-here):**
- This is **gpui-ce chronos edition** (`../Source`), not crates.io 0.2.2.
- `WindowKind::AnchoredPopup` → `gpui/src/platform/popup.rs`, skill
  **anchored-popups**.
- Vendored **`gpui-rsx`** (verbatim, MIT) → skill **gpui-rsx-markup**;
  ChronOS already depends `gpui-rsx.workspace = true`. Macro expands to
  **this fork's** `div()` — import `use gpui::div` always.
- Do not edit `gpui-rsx` source for consumer type errors; use
  `rsx_expand!` to debug.

**Design canon:** `docs/design/Volume Popup.dc.html`  
**Lifecycle canon:** `crates/app/src/updates_popup/mod.rs` +
`bar/widgets/updates.rs`  
**rsx canon:** `.claude/skills/gpui-rsx/SKILL.md` +
`side_panel_right/{header,permission}.rs`

**Prerequisite:** T120 accepted or at least not conflicting (different
files). Ship after / in parallel with T120 only if file zones stay
disjoint — **T121 does not touch notifications/**.

---

## Global Constraints

### Scope
- **In:** `volume_popup/`, `bar/widgets/volume.rs`, Theme tokens / light
  decorations if needed for Light C watermark/glow (same pattern as
  updates if `Theme.is_light` already exists).
- **Out:** `updates_popup`, `history_popup`, `system_popup` (except reading
  brightness fill-bar as anti-pattern to improve), MPRIS card, app-stream
  mute list (`ToggleStreamMute` — not in mockup), AUR, side panels.

### Mockup → UI (literal)
| Element | Spec |
|---|---|
| Width | **360px** (now 300) |
| Header | title **«Sound»** + close ✕ (22×22, hover) |
| Sections | **Volume** + **Microphone** (not «Speakers») |
| Row | mute icon \| name + device subtitle + chevron \| mono `%` / `Muted` |
| Control | **horizontal slider** 0–100 (track ~4px, thumb ~13) — drag + click |
| Device menu | absolute under title row, ~220px, checkmark on selected |
| Footer | two outlined buttons: `Mute output` / `Mute mic` (toggle labels) |
| Dark | no watermark/glow; Light C: watermark + top glow + elevated shadow |

### Fork / API
- Anchor: `PopupAnchor::BottomRight` + `PopupGravity::BottomLeft`,
  `SLIDE_X | FLIP_X`, `offset y=4`, `grab: true`, fallback
  `PopupNotSupportedError` → LayerShell TOP|RIGHT.
- Trigger: **`on_mouse_down(Left)`** + canvas bounds + **`.relative()`**
  wrapper (T117 lesson). Keep **scroll-to-adjust volume** on the bar
  widget (existing); do not break it.
- Dismiss: re-toggle bar, header ✕. No focus-loss close.
- `close` / `close_this` reentrancy discipline — copy updates (ghost-window).

### rsx vs builder map (mandatory in report)

| Piece | Approach | Why |
|---|---|---|
| Outer card shell, header «Sound» + ✕ | `rsx!` | static chrome from HTML |
| Light watermark / glow line | `rsx!` gated `theme.is_light` | static decoration |
| Footer dual mute buttons | `rsx!` or thin builder | simple `onClick` |
| Volume/Mic section chrome (labels) | `rsx!` OK if listeners stay thin | structure |
| **Slider track + thumb + drag** | **builder `div()`** | `on_mouse_down` / drag_move / hit geometry |
| Device dropdown rows | **builder** | dynamic list + `key`/`id` per device |
| Expanded state (`volumeMenuOpen` / `micMenuOpen`) | view fields + `cx.listener` | mutual exclusive menus |

Flagship: **rsx static, div live meters and listeners.** Rollback from rsx
to div is data, not failure — document in report.

**Blood facts (compile):**
1. `use gpui::div` even with only `rsx!` → else E0425.
2. `hover={|s| …}` is `StyleRefinement`, not `Div` → else E0631.
3. Stateful interactive needs `.id(...)`.
4. Confusing macro errors → `rsx_expand!` (gpui-rsx-markup).

### Backend
Existing `AudioCommand` is enough for mockup:
- `SetSinkVolume` / `SetSourceVolume`
- `ToggleSinkMute` / `ToggleSourceMute`
- `SetDefaultSink` / `SetDefaultSource`
- `EndpointState::{volume, muted, name, available}`

**Do not** invent a second audio stack. Slider maps percent 0–100 ↔
`f64` volume with existing `clamp_volume` (allow >100% only if product
already does on bar — bar clamps to 150% display; mockup max 100 —
**cap slider UI at 100%**, backend may still report boost; document).

### Quality
- `unsafe_code = deny`; no new unwrap/expect in prod paths.
- No silent `let _ = fallible` — `.log_err()` or match.
- Commits: `area : what`, named add, no AI trailers.
- Live grim mandatory (Task 5). Unit ≠ done for UX.

---

### Task 1: Bar volume widget — anchor capture

**Files:** `crates/app/src/bar/widgets/volume.rs`,  
signatures in `crates/app/src/volume_popup/mod.rs` (open/toggle).

**Reference:** `bar/widgets/updates.rs` (canvas + relative + mouse_down).

- [ ] **Step 1:** Widget holds `Rc<Cell<Bounds<Pixels>>>` (or same type as updates).
- [ ] **Step 2:** Wrapper `.relative()` + full-size absolute canvas writing bounds.
- [ ] **Step 3:** Replace `on_click` open with `on_mouse_down(Left)` →  
  `volume_popup::toggle(anchor_rect, parent, window, cx)`.
- [ ] **Step 4:** Preserve scroll-wheel volume adjust on the widget.
- [ ] **Step 5:** Unit tests for `describe` / `format_percent` still pass.
- [ ] **Step 6:** Commit  
  `bar : anchor capture for volume widget`

---

### Task 2: Window — AnchoredPopup + fallback + 360 width

**Files:** `crates/app/src/volume_popup/mod.rs`

**Reference:** `updates_popup/mod.rs` (`window_options`, fallback, open match).

- [ ] **Step 1:** `POPUP_WIDTH = 360.`
- [ ] **Step 2:** `open(cx, anchor_rect, parent)` / `toggle(anchor_rect, parent, window, cx)`.
- [ ] **Step 3:** Anchored options + LayerShell fallback on `PopupNotSupportedError`.
- [ ] **Step 4:** Revisit `estimate_popup_height` for mockup layout (header +
  2 sections + footer; extra when device menu open — grow window or
  overlay menu inside fixed height; **prefer grow window** like today so
  menu is not clipped).
- [ ] **Step 5:** Keep `resize_to_fit` / watcher notify; log update failures.
- [ ] **Step 6:** `cargo build -p chronos`
- [ ] **Step 7:** Commit  
  `volume_popup : anchored open + 360 width`

---

### Task 3: View chrome — mockup shell (rsx) + footer mutes

**Files:** `crates/app/src/volume_popup/view.rs` (and optional
`view_chrome.rs` if split keeps file smaller).

- [ ] **Step 1:** Header **Sound** + ✕ close_this (rsx preferred).
- [ ] **Step 2:** Section titles **Volume** / **Microphone** with device
  subtitle from `ep.name`, chevron, mute icon button (toggle mute).
- [ ] **Step 3:** Mono label: `Muted` or `{n}%` (JetBrains / `theme.font_mono`).
- [ ] **Step 4:** Footer two outlined buttons — labels from mockup  
  (`Mute output` / `Unmute output`, `Mute mic` / `Unmute mic`); accent
  when muted state.
- [ ] **Step 5:** Remove old −5% / +5% primary path (scroll on bar remains;
  optional: keep wheel on slider later — not required).
- [ ] **Step 6:** Light C decorations if `theme.is_light` exists; else dark-only
  and note debt.
- [ ] **Step 7:** Build; commit  
  `volume_popup : Sound chrome + footer mutes (rsx)`

---

### Task 4: Interactive sliders + device menus (builder)

**Files:** `view.rs` (builder)

**Slider behavior (acceptance):**
- Track full width of content column; click sets volume to click fraction.
- Drag thumb / drag on track updates continuously; dispatch
  `SetSinkVolume` / `SetSourceVolume` (debounce optional; prefer direct
  dispatch with service already async).
- Changing volume while muted **unmutes** (mockup `onVolumeChange` sets
  muted false) — match mockup.
- Visual: track bg mockup `#313244` / token; fill optional; thumb circle
  ~13px in thumb color.

**Device menu:**
- Click title row (not only mute icon) toggles that menu; opening one
  closes the other.
- Rows: device name + check when `is_default` / selected; click →
  `SetDefaultSink` / `SetDefaultSource`.
- Empty list: muted «No devices found».
- Cap rows (~8) or scroll inside menu if many — document choice.

- [ ] **Step 1:** Implement sink slider hit-testing + drag state on view.
- [ ] **Step 2:** Source slider same.
- [ ] **Step 3:** Device menus + resize_to_fit when expanded.
- [ ] **Step 4:** Wire icons (existing `icons/*` speaker/mic if present;
  svg path strings already used on bar — reuse).
- [ ] **Step 5:** `cargo build --release -p chronos`
- [ ] **Step 6:** Commit  
  `volume_popup : sliders + device menus`

---

### Task 5: Live smoke + report

- [ ] Kill stale chronos; run release.
- [ ] Click volume widget → popup under/near icon (not fixed wrong corner).
- [ ] Drag sink slider → `wpctl get-volume @DEFAULT_SINK@` matches.
- [ ] Mute output footer + icon; unmute.
- [ ] Mic slider + mute mic.
- [ ] Expand device list if multi-device; select default.
- [ ] grim: dark open, menu open, muted state; light if switchable.
- [ ] Report  
  `docs/orchestration/tasks/report/T121-volume-popup-anchored-redesign-report.md`  
  with **rsx vs div map**, commits, live evidence, residual debt.
- [ ] Architect accepts — minion does not self-accept.

---

## Execution notes

| Task | Hint |
|------|------|
| 1–2 | standard model, mechanical after updates pattern |
| 3 | standard, rsx careful with hover/id |
| 4 | stronger — drag geometry + audio race |
| 5 | operator + grim |

**Order:** 1 → 2 → 3 → 4 → 5. Optional merge 1+2 one commit if cleaner.

**Progress ledger (multi-session):**  
`.superpowers/sdd/progress.md` → `Task N: complete (commits …)`.
