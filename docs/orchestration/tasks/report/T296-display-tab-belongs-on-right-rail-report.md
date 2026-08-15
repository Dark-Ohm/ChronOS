# T296 — Display tab belongs on the right rail — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `crates/app/src/side_panel_left/**`, `crates/app/src/side_panel_right/**` (`tabs.rs`, `tab/mod.rs`, `tab/display.rs`, `panels_config.rs`, `view.rs`), `crates/app/src/bar/**` (`layout_config.rs`, `mod.rs`, `widgets/mod.rs`, `widgets/system.rs`), icon asset.
**Worktree:** `wt-t296` @ `81fd7cb` (T290 + T290-E merged on master) → branch `feat/t296-display-right-rail` → commit `a2c072f`.

## Status

**Done (code + unit tests green).** Live release verification deferred — see "Не сделано / Live".

## Симптом

T290 placed the new Display tab on the **left** rail (between Project and
Sessions). Per the 2026-08-16 panel canon, the left rail is AI-only
(Chat / Sessions / Project / Plan / Tools / Skills / Context Files / Archive);
Display (brightness + wallpaper — display *settings*) belongs on the **right**
rail, in the **bottom group**, immediately above shell settings. The bar's
`system` hexagon widget (T290's entry point into Display) was also the wrong
door — entry to Display is the right-rail button, not a bar icon.

## Contract (implemented)

1. **`PanelTab::Display`** added to `side_panel_right/tabs.rs`: NOT resizable,
   `preferred_content_width = 440.0`, label `"Display"`, icon
   `icons/rail-display.svg`. Added to `ALL` (now **18**), `id()`/`parse_id()`
   (`"display"`), `for_mode` **both** modes **before `EditorSettings`**.
2. **`panels_config.rs`** bottom-group defaults: `default_dev_bottom` /
   `default_gamer_bottom = ["display", "editor_settings"]`. Display is the **first
   button of the bottom group**, directly above shell settings. (`resolve_grouped`
   builds top/bottom from config; the rail renders top group, spacer, then bottom
   group — so Display sits below the spacer, above `editor_settings`.)
3. **Content is the SAME live view as T290** — `brightness_block` (slider/drag,
   latest-wins via `AppState::brightness`, no per-sample `ddcutil`) +
   `render_wallpaper_card` (single copy). Moved (rename) from
   `side_panel_left/tabs/display.rs` → `side_panel_right/tab/display.rs`; the left
   file is deleted.
4. **`TabContent::Display(Entity<DisplayTab>)`** in `side_panel_right/tab/mod.rs`;
   `create` routes it to a real `DisplayTab`; `view.rs` render + `tab_entity_id`
   matches gained a `Display` arm. Not a placeholder.
5. **Left rail stripped of Display** — `LeftTab::Display` removed from the enum,
   `PRIMARY_TABS`, `preferred_panel_width`/`label`/`icon_path`, the inventory /
   order / width tests, and `workspace_view.rs` (`display` field, `ensure_display`,
   render arm). Left rail is again Project-first.
6. **Bar `system` widget DELETED** — `bar/widgets/system.rs` removed; `mod system;`
   and the `"system"` `instantiate` arm dropped from `bar/widgets/mod.rs`;
   `"system"` removed from `BUILTIN_NAMES` and the default `right` in
   `bar/layout_config.rs`; all `layout_config` / `bar/mod.rs` tests expecting
   `"system"` updated. Entry to Display is now the right rail, not a bar icon.
7. **Preserved from T290:** popup deleted, `gaming_mode` at top level, wallpaper
   off `SystemTab`, `rail-display.svg` icon, T291-E `cx.refresh_windows()`.

## Done

### 1. `crates/app/src/side_panel_right/tabs.rs`

- Enum: `Display` variant added to the settings group (after `EditorSettings`,
  before `HyprlandBinds`).
- `pub const ALL: [PanelTab; 18]` — `PanelTab::Display` appended (index 17).
- `id()` → `"display"`; `parse_id()` → `"display" => Some(PanelTab::Display)`.
- `for_mode` (Developer **and** Gamer): `PanelTab::Display` inserted immediately
  before `PanelTab::EditorSettings`.
- `label()` → `"Display"`; `icon_path()` → `"icons/rail-display.svg"`;
  `preferred_content_width()` → `440.`. (`resizable()` already returns `false`
  for everything except `Preview`, so Display is correctly non-resizable.)
- Tests: `all_has_seventeen_tabs_in_fixed_order` renamed →
  `all_has_eighteen_tabs_in_fixed_order` (`len() == 18`, `ALL[17] == Display`);
  `developer_rail_is_six_product_tabs` / `gamer_rail_is_six_product_tabs` renamed
  → `…_seven_…` (vec +1, Display before EditorSettings). The `acp_settings
  precedes system_settings` invariant still holds (ACP < Display < EditorSettings).

### 2. `crates/app/src/side_panel_right/tab/mod.rs`

- `pub(crate) mod display;` added (module re-home); `use display::DisplayTab;`.
- `TabContent::Display(gpui::Entity<DisplayTab>)` variant.
- `TabContent::create` → `PanelTab::Display => TabContent::Display(cx.new(|cx|
  DisplayTab::new(cx)))`.
- `placeholder_description` gained `PanelTab::Display => "Display settings:
  brightness and wallpaper"` (exhaustive match over `PanelTab`).

### 3. `crates/app/src/side_panel_right/tab/display.rs` (MOVED from left, rename)

- `git mv` from `side_panel_left/tabs/display.rs`; content unchanged (uses
  `crate::side_panel_right::surfaces`, `crate::state`, `crate::wallpaper_ctl` —
  all valid from the new location). Keeps its own brightness + wallpaper
  `state::watch` subscriptions, `dispatched_brightness`, `track_bounds`,
  `waytrogen_available`.

### 4. `crates/app/src/side_panel_right/panels_config.rs`

- `default_dev_bottom` / `default_gamer_bottom` →
  `vec!["display".into(), "editor_settings".into()]`.
- Tests: `sanitize_drops_unknown_and_deduplicates` (top now gets `display`
  appended as a missing mode tab → `["system","files","preview","hyprland_binds","acp_settings","display"]`);
  `resolve_grouped_uses_config_values` (bottom `len() == 2`,
  `[Display, EditorSettings]`). `resolve_grouped_deduplicates` left unchanged
  (Display not in that fixture's config → not appended).

### 5. `crates/app/src/side_panel_right/view.rs`

- Render match + `tab_entity_id` match gained `TabContent::Display(entity) => …`
  arms (both one-liners, matching the other real-tab arms).
- **Test fix:** `move_tab_helper_noop_leaves_cache_and_disk_untouched` called
  `cx.read(|cx| panels_config::cached())` — but `cached()` is a free function
  (`pub fn cached() -> PanelLayoutConfig`), not a method on `App`. Changed to
  `panels_config::cached()` directly (matches the non-test call site at line 503).
  This was a latent bug exposed by the T296 compile (the test module now builds
  because the Display wiring forced a full `chronos --lib` test compile).

### 6. `crates/app/src/side_panel_left/tabs/mod.rs` (Display removed)

- `pub(crate) mod display;` + `pub(crate) use display::DisplayTab;` removed.
- Enum `LeftTab`: `Display` variant removed. `preferred_panel_width` /
  `label` / `icon_path` `Self::Display` arms removed. `PRIMARY_TABS` no longer
  contains `LeftTab::Display`.
- Tests: `primary_tabs_in_fixed_order` (Display out of the array);
  `select_display_uses_fixed_preferred_width` **deleted**; `resize_policy_matches_spec`
  / `fixed_widths_match_spec` / `width_for_open_fixed_tabs_use_preferred_exactly`
  / `all_tabs_inventory_is_complete` had their `Display` assertions/entries
  removed.

### 7. `crates/app/src/side_panel_left/workspace_view.rs` (Display removed)

- `display: Option<Entity<tabs::DisplayTab>>` field + doc comment removed.
- `WorkspaceView::new` initializer `display: None` removed.
- `ensure_display` method removed.
- Render `match` no longer has a `tabs::LeftTab::Display` arm; the
  `Plan | Tools | Skills | ContextFiles | Archive` arm follows `Project` directly.

### 8. `crates/app/src/bar/**` (system widget removed)

- `bar/widgets/system.rs` **deleted** (was the only `SystemWidget` definition;
  its `on_click` referenced `side_panel_left::tabs::LeftTab::Display`, now gone
  with the file).
- `bar/widgets/mod.rs`: `mod system;` + `"system" => Box::new(system::SystemWidget::new())`
  removed.
- `bar/layout_config.rs`: `"system"` removed from `BUILTIN_NAMES` and the default
  `right` vec; `default_matches_historical_builtin_order` + the four migration
  fixtures no longer contain `"system"`.
- `bar/mod.rs`: the `group_right_names` test cluster assertion updated to drop
  `"system"`.

## Evidence (commands run)

```
cargo check -p chronos
cargo test -p chronos --lib side_panel_left
cargo test -p chronos --lib side_panel_right
cargo test -p chronos-ui --lib
cargo test -p chronos --lib
```

| Command | Result |
|---|---|
| `cargo check -p chronos` | clean (only pre-existing/unrelated warnings — same set as T290/T290-E: `display_w` in `rail_view.rs:212`, `theme` in `mpris_card.rs`, `window` in `bar_settings.rs`/`hypr_binds.rs`, `BorrowAppContext` in `theme_config.rs`, etc.). |
| `cargo test -p chronos --lib side_panel_left` | **113 passed; 0 failed.** (T290 baseline for this subset was 114 — one fewer because `select_display_uses_fixed_preferred_width` was deleted with the left Display tab.) |
| `cargo test -p chronos --lib side_panel_right` | **195 passed; 0 failed.** Includes `all_has_eighteen_tabs_in_fixed_order`, `developer_rail_is_seven_product_tabs`, `gamer_rail_is_seven_product_tabs`, `resolve_grouped_uses_config_values` (bottom `[Display, EditorSettings]`), `tab_entity_id`/`render` exhaustiveness. |
| `cargo test -p chronos-ui --lib` | **19 passed; 0 failed** (`every_window_root_uses_window_font` + `window_font_sets_font_ui` green — T290-E work intact). |
| `cargo test -p chronos --lib` (full) | **476 passed; 0 failed; 0 ignored.** No regression vs the T290 477 baseline (the −1 is the deleted left-rail Display test). |

### Isolation proof

Per the blood rule each task lives on its own sibling git worktree; master's
dirty tree is never touched. T296 branched from `81fd7cb` (master, T290 + T290-E
already merged) into a fresh `wt-t296`:

- `cargo check -p chronos` clean and all three spec suites green **by itself** —
  the commit builds and passes in isolation, not merely "the tree builds".
- `Cargo.lock` reverted (`git checkout -- Cargo.lock`) before `git add` — not
  staged (verified `git diff --cached --name-only` excludes it). Blood rule:
  cargo re-resolves `Cargo.lock`; never commit it.
- `Source/gpui/` (the external fork) was **not** modified. T285 (`chat.rs`) was
  **not** touched.
- `git diff --stat` of the commit:

```
 11 files changed, 58 insertions(+), 159 deletions(-)
 rename crates/app/src/{side_panel_left/tabs => side_panel_right/tab}/display.rs (100%)
 delete mode 100644 crates/app/src/bar/widgets/system.rs
```

## Что НЕ сделано (выполняет Архитектор / дальше)

1. **Live + release verification (T296 §Верификация).** Unit tests prove the
   wiring, ordering, width, and bottom-group placement, but per project rules
   "компилируется и тесты зелёные" для окон/UX ничего не значит — нужен
   релизный бинарь и живой кадр. Не проверял, за архитектором. Required live:
   - на баре **нет** hexagon/яркости (виджет `system` удалён);
   - Display — **нижняя группа** правой рельсы, **над** System settings
     (выше `editor_settings`);
   - клик по ней → яркость + wallpaper; слева монитора нет;
   - `hyprctl layers` — без `system_popup`; grim бар + правая рельса.
2. **Не в зоне (по контракту T296):** возвращать `system_popup/`; вторая копия
   wallpaper на System; power/gaming на Display (они на System, T291);
   `Source/`, `Cargo.lock`, T285 `chat.rs`; формат-всё `side_panel_right/`
   (rustfmt не гонял).
3. **Самоприём / `done/` — НЕ делать.** Отчёт написан, коммит `a2c072f` готов;
   принимает Архитектор. Без самоприёма и без переноса в `done/`.

## Коммит

```
fix(right-panel): Display tab lives on the right rail (T296)
```

(11 files, +58 / −159 — see isolation `git diff --stat` above. `display.rs`
moved as a rename (100%); `Cargo.lock` исключён, `Source/gpui/` нетронут,
T285 `chat.rs` нетронут.)
