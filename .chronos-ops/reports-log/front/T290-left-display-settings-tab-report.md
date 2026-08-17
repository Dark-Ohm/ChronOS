# T290 — Left Display tab takes brightness and wallpapers — Report

**Date:** 2026-08-15
**Role:** FRONTEND. Zone: `crates/app/src/side_panel_left/**`, `crates/app/src/bar/widgets/system.rs`, `crates/app/src/{gaming_mode.rs,lib.rs,main.rs,power_controls.rs,scene.rs}`, `crates/app/src/side_panel_right/**`, icon asset.
**Worktree:** `wt-t290` @ `235185a5` (incl. T291-E) → commit `bb9790a`.

## Status

**Done (code + unit tests green).** Live release verification deferred — see
"Не сделано / Live".

## Симптом

Brightness and wallpapers were only reachable through the **right** System popup
(`system_popup`): clicking the bar's system widget (`bar/widgets/system.rs`)
called `system_popup::toggle`, and the wallpaper card lived in the right
`SystemTab`. There was no dedicated, always-on Display settings surface on the
left, and the popup machinery (`SystemPopupState`, `view`, `repaint_popup`) was
dead weight that T291-E had already outgrown (it repaints via `cx.refresh_windows()`,
not the popup). The left rail had a gap between Project and Sessions.

## Contract (implemented)

1. **New `LeftTab::Display`** in `side_panel_left/tabs/mod.rs`: NOT resizable,
   `preferred_panel_width = 440.0`, label `"Display"`, icon
   `icons/rail-display.svg`, inserted into `PRIMARY_TABS` **after Project, before
   Sessions**. Propagated through every `match LeftTab`, the inventory/label/icon
   tests, and the `workspace_view.rs` render arm. Content is a **live view**, not
   a "Coming later" shell.
2. **`workspace_view.rs`** handles `Display` like Project/Sessions via
   `ensure_display` (own `Entity<DisplayTab>`), NOT `ensure_shell`.
3. **Brightness** — same semantics as `system_popup/view.rs::brightness_block`:
   slider (click + drag), `%`, latest-wins / debounce via `AppState::brightness`,
   no per-sample `ddcutil` spawn.
4. **Wallpapers** — `render_wallpaper_card` moved out of the right `SystemTab`
   (single copy, now in `display.rs`); the card disappears from the right System.
5. **`bar/widgets/system.rs`** `on_click` →
   `side_panel_left::select_tab(LeftTab::Display)` instead of `system_popup::toggle`.
   **`system_popup/` deleted** (mod + init + close). No dangling `system_popup::`.
6. **`gaming_mode` relocated** to top-level `crates/app/src/gaming_mode.rs`;
   **T291-E `cx.refresh_windows()` preserved** in both `apply`/`revert`;
   `repaint_popup` + popup import deleted; `GamingModeState::init` re-homed as
   `gaming_mode::init`. Call sites updated (`power_controls.rs`, `scene.rs`,
   `side_panel_right/tab/system.rs`).

## Done

### 1. `crates/app/assets/icons/rail-display.svg` (NEW)

Monitor + brightness-glyph icon for the new Display tab rail button. Registered
in the `icons!` macro (`crates/app/src/assets.rs`) between `rail-captures.svg`
and `rail-editor.svg`.

### 2. `crates/app/src/side_panel_left/tabs/mod.rs`

- `pub(crate) mod display;` after `mod chat;`; `pub(crate) use display::DisplayTab;`
  re-export (mirrors `use shell::ShellTab;` — fixes `tabs::DisplayTab` path
  resolution in `workspace_view.rs`).
- Enum: `pub enum LeftTab { Project, Display, Sessions, Chat, Plan, Tools, Skills, ContextFiles, Archive }`.
- `preferred_panel_width`: `Self::Display => 440.0,`
- `label`: `Self::Display => "Display",`
- `icon_path`: `Self::Display => "icons/rail-display.svg",`
- `PRIMARY_TABS`: `&[Project, Display, Sessions, Chat, Plan, Tools, Skills, ContextFiles]`.
- `rail_view.rs` auto-iterates `PRIMARY_TABS` → the Display rail button renders
  with no extra match arm.
- Tests: `primary_tabs_in_fixed_order` (Display inserted), `resize_policy_matches_spec`
  (`!LeftTab::Display.is_resizable()`), `fixed_widths_match_spec`
  (`LeftTab::Display.preferred_panel_width(), 440.0`),
  `width_for_open_fixed_tabs_use_preferred_exactly` (Display in fixed list),
  `all_tabs_inventory_is_complete` (Display in all-variants array), and NEW
  `select_display_uses_fixed_preferred_width` (mirrors Project's test; asserts
  `r == (LeftTab::Display, 440.0, false)`).

### 3. `crates/app/src/side_panel_left/tabs/display.rs` (NEW — core deliverable)

- `DisplayTab` entity with **own** brightness + wallpaper subscriptions
  (`state::watch` on `AppState::brightness(cx).subscribe()` and
  `AppState::wallpaper(cx).subscribe()`), `dispatched_brightness: Option<u8>`,
  `track_bounds: Rc<Cell<Bounds<Pixels>>>`, `wallpaper: WallpaperState`,
  `waytrogen_available: bool`. Subscriptions retained in `_subs`.
- `DisplayTab::new(cx)` — called as `cx.new(|cx| tabs::DisplayTab::new(cx))`
  (the `E0061` fix: takes `&mut Context<Self>`).
- `render` → `div#display-tab` containing `brightness_block(...)` then
  `render_wallpaper_card(...)`.
- `brightness_block` — ported from `system_popup/view.rs` with identical
  semantics: click+drag via `brightness_frac_from_bounds` +
  `set_brightness_from_frac` → `AppState::brightness(cx).dispatch(BrightnessCommand::Set(value))`
  + `cx.notify()`; steppers use `STEP=5` absolute `Set` (not Step). `FALLBACK_TRACK_W = 352.0`
  as first-frame fallback (popup's `POPUP_WIDTH - …` replaced by a constant since
  there is no popup width). Unused `accent` binding dropped (silences warning).
- `render_wallpaper_card` — single copy, moved verbatim from
  `side_panel_right/wallpaper_card.rs`, using `crate::side_panel_right::surfaces::card`
  (`pub(crate)`), `crate::wallpaper_ctl::next`,
  `crate::wallpaper_ctl::open_waytrogen_gallery`, `WallpaperState`, `Theme`,
  `ElementId`, `SharedString`.

### 4. `crates/app/src/side_panel_left/workspace_view.rs`

- Field `display: Option<Entity<tabs::DisplayTab>>` added after `project`.
- `WorkspaceView::new` initializer: `display: None,`.
- `ensure_display` — mirrors `ensure_project`: lazily
  `cx.new(|cx| tabs::DisplayTab::new(cx))`, pushes observe subscription, stores &
  returns clone.
- Render arm: `tabs::LeftTab::Display => div().id("side-panel-left-product-clip").w(px(visible_w)).h_full().overflow_hidden().flex_none().child(self.ensure_display(cx)).into_any_element()`
  placed between Project and Plan arms.

### 5. `crates/app/src/bar/widgets/system.rs` (REWRITTEN)

- Removed `canvas` / bounds `Rc<Cell<Bounds>>` machinery and `system_popup::toggle`
  import. `SystemWidget` is now a unit struct (`pub struct SystemWidget;`, `new() -> Self { Self }`).
- `render` `on_mouse_down(MouseButton::Left, …)`: if `edit_mode::is_active(cx)` return;
  else `side_panel_left::select_tab(LeftTab::Display, cx);`. Keeps `hexagon-core.svg`.

### 6. `crates/app/src/gaming_mode.rs` (NEW — relocated from `system_popup/gaming_mode.rs`)

- Identical logic to the original **except**: dropped
  `use crate::system_popup::{SystemPopupState, view::SystemPopupView};`, deleted
  `fn repaint_popup`, added `pub fn init(cx: &mut App) { GamingModeState::init(cx); }`.
- **Kept `cx.refresh_windows();`** in both `apply` and `revert` — the T291-E line
  (only repaint mechanism for the gaming knob now that the popup is gone).
- Doc note added explaining `repaint_popup` removal. Test
  `on_payload_targets_all_three_options` retained (`animations`/`blur`/`allow_tearing`).
- Declared `pub(crate) mod gaming_mode;` in both `lib.rs` and `main.rs`.

### 7. `crates/app/src/side_panel_right/tab/system.rs` (EDITED)

- `use crate::system_popup::gaming_mode;` → `use crate::gaming_mode;`; other
  `gaming_mode::` usages in render still resolve.
- Removed `use crate::side_panel_right::wallpaper_card::render_wallpaper_card;`.
- Removed the `.child(render_wallpaper_card(&self.wallpaper, self.waytrogen_available, cx))`
  call from the scroll body (kept mpris / power profile / gaming cards).

### 8. `crates/app/src/side_panel_right/mod.rs` + `wallpaper_card.rs` (DELETED)

- `mod wallpaper_card;` declaration removed. `side_panel_right/wallpaper_card.rs`
  deleted — single copy now lives in `display.rs`.

### 9. `crates/app/src/system_popup/` (DELETED)

- `mod.rs`, `view.rs` (→ `display.rs`), `gaming_mode.rs` (→ `gaming_mode.rs`) all
  gone. No code-path references remain (only doc-comment mentions elsewhere, e.g.
  `power_controls.rs`, `display.rs`, `side_panel_right/mod.rs` — these are prose,
  not compile references).

### 10. Call-site rewiring (no `system_popup::` left)

- `lib.rs`: `pub mod system_popup;` → `mod gaming_mode;`
- `main.rs`: `mod system_popup;` → `mod gaming_mode;`; `system_popup::init(cx);` → `gaming_mode::init(cx);`
- `power_controls.rs`: `crate::system_popup::gaming_mode::toggle(cx);` → `crate::gaming_mode::toggle(cx);`
- `scene.rs`: `use crate::system_popup::gaming_mode::{self, GamingModeState};` → `use crate::gaming_mode::{self, GamingModeState};`

## Evidence (commands run)

```
cargo check -p chronos
cargo test -p chronos --lib
cargo test -p chronos --lib side_panel_left
cargo test -p chronos --lib select_display_uses_fixed_preferred_width
cargo test -p chronos --lib primary_tabs_in_fixed_order
```

| Command | Result |
|---|---|
| `cargo check -p chronos` | clean (only pre-existing/unrelated warnings: `std::ops::Range` in `side_panel_left/mod.rs:38`, `width_for_open` in `rail_view.rs:26`, `UPowerSubscriber` in `gaming_mode.rs:40`, `BorrowAppContext` in `theme_config.rs`). |
| `cargo test -p chronos --lib` | **477 passed; 0 failed; 0 ignored.** |
| `cargo test -p chronos --lib side_panel_left` | 114 passed; 0 failed. Includes new `select_display_uses_fixed_preferred_width` + updated inventory/order/width tests; `switch_project_sets_path_and_clears_session`, `chat_tab_source_has_no_window_lifecycle`, `canvas_constants_match_tabs_constants` still green. |
| `cargo test -p chronos --lib select_display_uses_fixed_preferred_width` | 1 passed — `r == (LeftTab::Display, 440.0, false)` (fixed, not resizable). |
| `cargo test -p chronos --lib primary_tabs_in_fixed_order` | 1 passed — Display sits between Project and Sessions. |

### Isolation proof

Per the blood rule each task lives on its own sibling git worktree; master's
dirty tree is never touched. The change is self-contained on `wt-t290`
(`235185a5` → `bb9790a`):

- `cargo check -p chronos` clean and `cargo test -p chronos --lib` → 477 green,
  i.e. the commit builds and passes **by itself**, not merely "the tree builds".
- `Cargo.lock` reverted (`git checkout -- Cargo.lock`) before `git add` — not
  staged (verified `git diff --cached --name-only` excludes it). Blood rule:
  cargo re-resolves `Cargo.lock`; never commit it.
- `Source/gpui/` (the external fork) was **not** modified.
- `git diff --stat` of the commit:

```
 15 files changed, 324 insertions(+), 600 deletions(-)
 create mode 100644 crates/app/assets/icons/rail-display.svg
 rename crates/app/src/{system_popup => }/gaming_mode.rs (87%)
 rename crates/app/src/{system_popup/view.rs => side_panel_left/tabs/display.rs} (53%)
 delete mode 100644 crates/app/src/side_panel_right/wallpaper_card.rs
 delete mode 100644 crates/app/src/system_popup/mod.rs
```

## Что НЕ сделано (выполняет Архитектор / дальше)

1. **Live + release verification (T290 §Верификация).** Unit tests prove the
   wiring, ordering, and width policy, but per project rules "компилируется и
   тесты зелёные" для окон/UX ничего не значит — нужен релизный бинарь и живой
   кадр. Не проверял, за архитектором. Required live:
   - клик по system-виджету в bar → открывается **левая** Display-вкладка (не попап);
   - слайдер яркости меняет глобальную яркость (через `AppState::brightness`);
   - карточка wallpaper видна в левой Display и **исчезла** из правой System;
   - gaming-переключатель всё ещё перекрашивает окна через `cx.refresh_windows()`
     (T291-E сохранён).
2. **Не в зоне (по контракту T290):** pull power/gaming влево; ACP/composer/T288
   cwd; `Source/gpui/`; `Cargo.lock`; вторая копия `wallpaper_card` (единственный
   экземпляр теперь в `display.rs`).
3. **Самоприём / `done/` — НЕ делать.** Отчёт написан, коммит `bb9790a` готов;
   принимает Архитектор. Без самоприёма и без переноса в `done/`.

## Коммит

```
feat(left-panel): Display tab takes brightness and wallpapers (T290)
```

(15 files, +324 / −600 — see isolation `git diff --stat` above. `Cargo.lock`
исключён, `Source/gpui/` нетронут.)
