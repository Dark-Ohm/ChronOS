# T278 — левая workspace: two-surface lifecycle cutover report

**Slice A1 (Tasks 1–2 из `2026-08-13-left-ai-workspace-slice-a.md`).
Unit green; live UX не проверялось.**

## Implementation commits

```text
c38f93a feat(left-panel): define workspace geometry and tab contracts
0fe6a47 feat(left-panel): split rail and fixed workspace canvas
9cd0e53 fix(left-panel): architect round 2 — clip to visible slice,
        reset close-state, preserve dock width, drop legacy resize
ac4533d fix(left-panel): architect round 3 — pure dock_transition
        helper, restore spec rail-only expansion
19263d3 fix(left-panel): architect round 4 — real integration test,
        free-function dock reducer
```

c38f93a — Task 1: `tabs/mod.rs` (LeftTab enum + ResizableWidths + width_for_open),
`state/geometry` (pure LEFT helpers — visible_content_width, content_input_region
left-aligned, resize_handle_x, resize_target_width +delta not negated,
content_interactive_width с 4 px handle на visible=0 при drag, clamp_panel /
clamp_resizable с sanitization NaN), `lib.rs` (publish side_panel_left).

0fe6a47 — Task 2: новый `SidePanelLeftState_` SoT (rail/content handles,
content_view weak entity, active_tab, panel_width, remembered_widths,
dock_content, resizing, last_exclusive_zone), `TwoSurfaceOpen` enum +
`two_surface_open_outcome` (pure), `rail_window_options` + `content_window_options`,
полный rewrite `open_window` / `close` / `close_this` / `toggle` для
двух-surface lifecycle, новые сущности `RailView` и `WorkspaceView`.
`SidePanelLeft::render` потерял все `window.resize()` / `set_exclusive_*` вызовы
(rail/content теперь владеют), `panel.rs` лишился resize-handle и связанных
listeners, `expand_with_composer` / `compose_and_send` маршрутизируются
через weak content_view entity.

9cd0e53 — Architect round 2:
- WorkspaceView mirror'ит `child.state.width = max(visible_w, SIDEBAR_MIN_WIDTH)`,
  оборачивает child в `.w(px(visible_w)).overflow_hidden()` и не рендерит
  child вовсе при `visible_w == 0`.
- `close()` и `close_this()` сбрасывают `panel_width = RAIL_WIDTH` и
  `dock_content = false` ДО раннего return.
- `on_dock_toggle` только флипает `dock_content` (round 2 ошибка — не сохранял
  ширину на rail-only → dock, создавал deadlock).
- Удалён legacy resize код из `SidePanelLeft`.

ac4533d — Architect round 3 (revert round 2 dock reducer):
- Pure helper `tabs::dock_transition(panel_width, dock_content, active_tab,
  &ResizableWidths) -> (f32, bool)` — единственный источник истины для
  spec §4.1 dock reducer:
  - rail-only + dock on → expand to `width_for_open(active_tab, remembered)`
  - overlay + dock on → preserve current width, flip flag
  - docked + dock off → preserve current width, flip flag
- `WorkspaceView::on_dock_toggle` использует pure helper вместо
  inline state mutation.
- 6 scenario тестов заменили round 2 "preserves always" test, плюс
  integration test `on_dock_toggle_uses_pure_helper` (round 4 отметил
  его как тавтологию — round 3 не дёргал reducer).

19263d3 — Architect round 4 (errata на round 3 tautology):
- `pub fn apply_dock_toggle(cx: &mut App)` — free function на App,
  вынесенная из `WorkspaceView::on_dock_toggle`. Причина: попытка
  протестировать через entity заставляет инстанцировать `SidePanelLeft`,
  чей `new()` spawn'ит async ACP-connect через Tokio runtime, который
  нельзя собрать в `TestAppContext`. Это аргумент за то, чтобы reducer
  не требовал entity, а не за отказ от теста.
- `WorkspaceView::on_dock_toggle` стал тонким dispatcher'ом:
  `apply_dock_toggle(cx); cx.notify();`.
- Тавтология `on_dock_toggle_uses_pure_helper` (round 3, присваивала и
  читала обратно без вызова reducer'а) удалена и заменена на реальный
  интеграционный тест `apply_dock_toggle_matches_helper_in_real_app`,
  который на `TestAppContext` дёргает `apply_dock_toggle(cx)` на
  пяти ветках spec §4.1 и проверяет, что SoT совпадает с pure
  helper'ом:
  1. rail-only + dock on (Chat default = 560)
  2. overlay + dock on (612 preserved)
  3. docked + dock off (612 preserved)
  4. rail-only + dock on с Chat remembered = 700
  5. rail-only + dock on с Sessions (fixed 400)

## Изменённые символы

`crates/app/src/side_panel_left/tabs/mod.rs`:
- `pub enum LeftTab { Project, Sessions, Chat, Plan, Tools, Skills, ContextFiles, Archive }`
- `pub struct ResizableWidths { chat, plan, context_files }` с `Default` и `sanitized`
- `pub const RAIL_WIDTH: f32 = 40.0`
- `pub const RESIZE_HANDLE_WIDTH: f32 = 4.0`
- `pub const MAX_PANEL_WIDTH: f32 = 960.0`
- `pub const CONTENT_CANVAS_WIDTH: f32 = 920.0`
- `pub const SOFT_OPEN_MIN_WIDTH: f32 = 360.0`
- `pub const PRIMARY_TABS: &[LeftTab]`
- `pub const BOTTOM_TAB: LeftTab = LeftTab::Archive`
- `pub fn width_for_open(tab, &ResizableWidths) -> f32`
- `LeftTab::is_resizable / preferred_panel_width / label / icon_path`
- **round 3:** `pub fn dock_transition(panel_width, dock_content, active_tab, &ResizableWidths) -> (f32, bool)`

`crates/app/src/side_panel_left/state.rs`:
- новый `pub mod geometry { hard_min, hard_max, clamp_panel, clamp_resizable,
  visible_content_width, content_interactive_width, content_input_region,
  resize_handle_x, resize_target_width }`
- старые `SidePanelLeftState` / `StreamingState` / `PanelState` / `AgentStatus`
  сохранены — render-path legacy bridge использует их

`crates/app/src/side_panel_left/mod.rs`:
- новый `pub struct SidePanelLeftState_` с полями rail_handle, content_handle,
  content_view, active_tab, panel_width, remembered_widths,
  active_project_path, active_session_id, dock_content, resizing, pinned,
  peek_generation, last_exclusive_zone
- `impl SidePanelLeftState_ { exclusive_px, resize, ensure_content_width }`
- `pub(crate) enum TwoSurfaceOpen { CommitBoth, RollbackContent }`
- `pub(crate) fn two_surface_open_outcome(rail_opened: bool) -> TwoSurfaceOpen`
- `pub(crate) fn rail_window_options(display_id, cx) -> WindowOptions`
- `pub(crate) fn content_window_options(display_id, cx) -> WindowOptions`
- `fn content_window_margin(top_gap) -> (Pixels, Pixels, Pixels, Pixels)`
- `fn display_height / panel_height / panel_edge_gap`
- `open_window` (двух-surface: content → rail; rollback при rail failure)
- `open_pinned / open_peek / close / close_this / toggle` переписаны
- `close()` и `close_this()` сбрасывают panel_width и dock_content
- `should_close_on_peek_leave` учитывает `resizing`
- `hold_peek / schedule_release_peek / schedule_release_from_app /
  close_peek_if_not_pinned / PEEK_LEAVE_DEBOUNCE` — сохранены
- `expand_with_composer / compose_and_send` — через weak content_view
- **round 4:** `pub fn apply_dock_toggle(cx: &mut App)` — free function
  (вынесена из `WorkspaceView::on_dock_toggle` чтобы reducer мог быть
  покрыт unit-тестом без entity)
- `init` — сохранён, hover strip остаётся dormant
- `SidePanelLeft::render` потерял `set_exclusive_*` и `window.resize()`
- `impl Drop for SidePanelLeft` сохранён
- Поля `resize_start_x`, `resize_start_width`, `last_resized_width` удалены
- Методы `start_resize`, `update_resize` удалены (legacy dead code)

`crates/app/src/side_panel_left/rail_view.rs`:
- `pub struct RailView { content: WeakEntity<WorkspaceView>, _content_sub: Subscription }`
- `impl RailView { pub fn new(...) -> Self }`
- `fn rail_button_bg(...) -> Hsla`
- `fn render_rail_button(tab, is_active, content, theme) -> impl IntoElement`
- `pub fn render_rail(cx, content) -> impl IntoElement`
- `impl Render for RailView { exclusive_zone cache, render rail column }`
- Dock toggle icon: `if dock_content { "⊟" } else { "⊞" }` (action-oriented)
- `WorkspaceView::on_rail_tab_select` — three-action policy
- **round 3:** `WorkspaceView::on_dock_toggle` использует pure helper
  `tabs::dock_transition`

`crates/app/src/side_panel_left/workspace_view.rs`:
- `pub struct WorkspaceView { content: Entity<SidePanelLeft>, last_visible_width,
  resize_start_x, resize_start_width, focus_composer_pending, _sub }`
- `impl WorkspaceView { pub fn new, pub panel_width, pub set_panel_width,
  pub request_focus_composer, fn perform_focus_composer,
  fn start_resize, fn update_resize, fn end_resize,
  pub on_rail_tab_select, pub on_dock_toggle }`
- `impl Render for WorkspaceView` — mirror child.state.width = max(visible_w,
  SIDEBAR_MIN_WIDTH), wrap child in `.w(px(visible_w)).overflow_hidden()` div,
  render nothing when visible_w == 0

`crates/app/src/side_panel_left/panel.rs`:
- удалён resize-handle element (`#resize-handle` div)
- удалены `resize_drag_handler` / `resize_mouse_handler` listeners
- остальное наследие (sidebar, sessions, chat, composer) сохранено — bridge для A2

`crates/app/src/lib.rs`:
- добавлен `pub mod side_panel_left;` для доступа тестов lib

## Ownership двух surfaces

| Характеристика          | `rail` surface                      | `content` surface                     |
|-------------------------|--------------------------------------|---------------------------------------|
| Namespace               | `side_panel_left_rail`               | `side_panel_left_content`             |
| Anchor                  | `TOP \| LEFT`                        | `TOP \| LEFT`                         |
| Layer                   | `Overlay`                            | `Overlay`                             |
| Keyboard                | `None`                               | `OnDemand`                            |
| Width                   | 40 px fixed                          | 920 px fixed                          |
| Left margin             | 0                                    | 40 px (явный, после `-1` opt-out)     |
| `exclusive_zone`        | live value `RAIL_WIDTH` или `panel_width` | `Some(px(-1.))` (opt-out foreign) |
| `exclusive_edge`        | `LEFT`                               | —                                     |
| Resize                  | никогда                              | никогда                               |
| Visible slice           | весь surface                         | left-aligned slice в 920 px canvas    |
| Painted slice           | весь surface                         | `visible_w` px (clipped), 0 при rail-only |
| Render owner            | `RailView::render` (rail_view.rs)    | `WorkspaceView::render` (workspace_view.rs) |
| Exclusive owner         | `RailView::render`                   | `WorkspaceView::render` (через input region) |
| Input region owner      | —                                    | `WorkspaceView::render` (left-aligned, пустой при visible_w=0) |
| Product content         | —                                    | `Entity<SidePanelLeft>` child         |
| Close path              | `Window::remove_window()` из `close` | то же                                 |
| Hover enter/leave       | `hold_peek` / `schedule_release_peek` | то же                                 |
| Dock reducer            | rail-only + on → expand; else preserve (round 3) | то же                                 |

State хранится в `SidePanelLeftState_` (global) и `SidePanelLeft` (per-instance
legacy child). `WorkspaceView::render` зеркалит `panel_width` (clamped to
visible slice) и `dock_content` из SoT в child перед рендером, чтобы legacy
`chat_open` / `dock_chat` чекеры видели актуальные значения.

## Atomic open / rollback

```text
open_window(cx, pinned):
    if rail_handle.is_some():
        upgrade peek → pinned if requested
        return
    display_id = pult_display_id_or_primary(cx)

    # Open content first
    mut opened_workspace = None
    content_result = cx.open_window(content_window_options, |window, view_cx| {
        panel = view_cx.new(|cx| SidePanelLeft::new(cx))
        workspace = view_cx.new(|cx| WorkspaceView::new(panel.clone(), cx))
        opened_workspace = Some(workspace.clone())
        view_cx.new(|cx| Root::new(workspace, window, cx).bordered(false).bg(transparent_black()))
    })
    match content_result:
        Err(_) → return (early, before any state mutation)
        Ok(handle) → continue
    let workspace_entity = opened_workspace.unwrap_or_else(rollback content)

    # Open rail second
    rail_result = cx.open_window(rail_window_options, |window, view_cx| {
        rail = view_cx.new(|cx| RailView::new(workspace_entity.downgrade(), cx))
        view_cx.new(|cx| Root::new(rail, window, cx).bordered(false).bg(transparent_black()))
    })

    match two_surface_open_outcome(rail_result.is_ok()):
        RollbackContent → content_handle.update(remove_window)
        CommitBoth → publish rail_handle + content_handle + content_view weak
                      to SidePanelLeftState_
```

`close(cx)` (round 2):
```text
1. take rail_handle, content_handle
2. unconditionally reset SoT:
   - content_view = None
   - pinned = false
   - resizing = false
   - last_exclusive_zone = None
   - panel_width = RAIL_WIDTH
   - dock_content = false
3. if both handles None → return (idempotent)
4. clear rail zone + remove_window rail
5. remove_window content
```

`close_this(window, cx)` (round 2): тот же reset в inner block, плюс
clear zone + remove_window this + remove other via second handle.

`on_dock_toggle(cx)` (round 4):
```text
# WorkspaceView method — thin dispatcher.
WorkspaceView::on_dock_toggle(cx):
    crate::side_panel_left::apply_dock_toggle(cx)
    cx.notify()

# Real reducer — free function on &mut App.
apply_dock_toggle(cx):
    1. (next_w, next_dock) = tabs::dock_transition(
           panel_width, dock_content, active_tab, &remembered_widths,
       )
    2. apply to SoT: panel_width = next_w; dock_content = next_dock
    3. invalidate last_exclusive_zone cache if exclusive_px changed
```

`tabs::dock_transition` (round 3, pure):
```text
next_dock = !dock_content
next_w = if !dock_content && visible_content_width(panel_width) <= 0.0:
    width_for_open(active_tab, remembered)  # rail-only expansion
else:
    panel_width  # preserve (overlay↔docked transitions)
return (next_w, next_dock)
```

## Тесты — exit codes и counts

```text
cargo test -p chronos --lib --no-run        → Finished, exit 0
cargo test -p chronos --lib side_panel_left   → 72 listed; real run filtered by
                                                test name match, 71 in side_panel_left
cargo test -p chronos --lib side_panel_left:: → 71 passed; 0 failed, exit 0
cargo test -p chronos --lib                  → 401 passed; 0 failed, exit 0
cargo check -p chronos                        → Finished, exit 0
cargo check -p chronos                        → (bin) Finished, exit 0
```

**Count note:** `cargo test -- --list` с фильтром `side_panel_left`
возвращает **72** — это включает
`side_panel_right::view::tests::needs_width_resize_still_serves_side_panel_left`,
чей identifier содержит "side_panel_left" в имени (right panel читает
left panel state в тесте T243 регрессии). Side_panel_left под
непосредственным `::` фильтром (без промежуточных подстроковых матчей)
отдаёт 71. Архитектор правильно сосчитал 72 по широкому фильтру;
внутри модуля — 71.

Side-panel tests breakdown (71 total in module):
- `side_panel_left::tabs::tests::*` — 16 passed (Task 1)
- `side_panel_left::state::geometry::tests::*` — 19 passed (Task 1)
- `side_panel_left::tests::*` — 35 passed (Task 2 + rounds 2/3/4 regression)
- `side_panel_left::workspace_view::tests::canvas_constants_match_tabs_constants` — 1 passed

Round 2 new tests (5):
- `reopen_after_dock_resets_to_rail_only` — gpui::test, drives `close()`
- `close_this_path_also_resets_to_rail_only` — source-text guard
- `painted_slice_width_matches_visible_w` — geometry contract
- `dock_toggle_icon_convention_is_action_oriented` — icon direction
- `dock_toggle_preserves_panel_width` — round 2 deadlock fix, удалён в round 3

Round 3 new tests (6 — круглый-2 test заменён; нетто +5):
- `dock_transition_from_rail_only_expands_to_preferred_width`
- `dock_transition_from_rail_only_uses_fixed_width_for_fixed_tabs`
- `dock_transition_from_overlay_preserves_width_on_dock_on`
- `dock_transition_from_docked_preserves_width_on_dock_off`
- `dock_transition_uses_remembered_width_for_resizable_tab`
- `dock_transition_does_not_leak_into_dock_off_cases`

Round 4 test:
- `apply_dock_toggle_matches_helper_in_real_app` — заменяет round-3
  тавтологию `on_dock_toggle_uses_pure_helper`. Дёргает production
  reducer `apply_dock_toggle(cx)` на 5 spec-ветках через TestAppContext,
  проверяет что SoT совпадает с pure helper'ом. Нетто +0 (один удалён,
  один добавлен).

## Доказательство отсутствия `window.resize(`

`rg -n 'window\.resize\(' crates/app/src/side_panel_left`:

```text
crates/app/src/side_panel_left/workspace_view.rs:13:  //! slice at `resize_handle_x(visible_w)`. `window.resize()` is forbidden
crates/app/src/side_panel_left/mod.rs:350:  // All `window.resize()` / `set_exclusive_zone()` / `set_exclusive_
crates/app/src/side_panel_left/mod.rs:1900:  // T278 spec §"Запрещено": `window.resize()` is forbidden across
crates/app/src/side_panel_left/mod.rs:1911:  // Strip inline string literals — ` "window.resize() ..." `
crates/app/src/side_panel_left/mod.rs:1920:  !without_strings.contains("window.resize("),
crates/app/src/side_panel_left/mod.rs:1921:  "{file_label} line {} contains a live `window.resize(` \
```

Все шесть совпадений — комментарии, doc-комментарии или сама строка-литерал
в source-contract тесте `window_options_have_no_resize_calls`. Ни одного
живого вызова `window.resize(` во всех файлах `side_panel_left/` нет.

Тест `window_options_have_no_resize_calls` дополнительно сканирует три
ключевых файла (`mod.rs`, `workspace_view.rs`, `rail_view.rs`) с фильтром,
отбрасывающим строки, начинающиеся с `//`, `/*`, `*`, `//!` или содержащие
inline string-литералы.

## Что не проверялось живьём

Live smoke не запускался. Unit-зелёное покрытие не равно UX-приёмке —
это gate T281. Не проверены:

- Реальное появление двух surface в Hyprland (`hyprctl layers`)
- Geometry rail (40 px) и content (920 px) на мониторе pult
- Визуально корректный left-margin content = 40 px
- Visible slice clipping: при rail-only (visible=0) на экране ничего не
  рисуется внутри content canvas; при expanded контент не переползает
  за правильную кромку visible slice (round 2 — клип-обёртка подтверждена
  unit-тестом `painted_slice_width_matches_visible_w`, но live paint не
  проверен)
- Input region left-aligned at x=0 — Wayland реально пропускает клики
  вне видимого slice в content canvas
- Drag handle на правой кромке видимого content:
  - mouse-down во время rail-only (visible=0) → handle at x=0, интерактивен
  - drag вправо увеличивает panel (формула `+delta`, не `-delta`)
  - drag влево уменьшает до rail-only, не ниже
  - mouse-up чистит `resizing` флаг → input region возвращается empty
- Rail-only default: `Super+A` открывает обе surface в rail-only визуале
  (panel_width=40, visible_w=0, input region пустой)
- Повторный `Super+A` закрывает обе surface и сбрасывает panel_width=40,
  dock_content=false (round 2 фикс — unit-тест
  `reopen_after_dock_resets_to_rail_only` подтверждает SoT reset)
- `Super+A → close → Super+A` цикл приходит в rail-only, не expanded (round 2)
- Hover на rail не открывает peek (per 2026-07-23 design, hover strip
  остаётся dormant)
- Peek debounce не закрывает panel во время resize drag
- `expand-left` IPC: открывает обе, dock=true, фокусирует composer
- `compose-and-send` IPC: открывает обе, dock=true, fills composer,
  отправляет в ACP, фокусирует composer после send
- Долгое нажатие активной tab → collapse to rail-only
- Долгое нажатие другой tab → switch + open
- Dock toggle round 3 contract (live):
  - rail-only + ⊞ → content открывается на remembered/preferred width,
    dock=true, rail-exclusive растёт до panel_width
  - overlay + ⊞ → width preserved, dock=true, rail-exclusive растёт
  - docked + ⊟ → width preserved, dock=false, rail-exclusive падает до
    RAIL_WIDTH
  - rail-only + ⊟ (после dock=true через 1) → нет deadlock: width
    preserved at expanded value, dock=false
- Toggling между Project/Sessions/etc — фиксированные ширины применяются
  без resize handle
- Resizable tabs (Chat/Plan/ContextFiles) показывают handle и применяют
  remembered_widths
- `compose_and_send` корректно отправляет multi-line текст в ACP turn
- Peek-leave от rail переходит на content без ghost-эффектов
- Click-X button в legacy panel (`side-panel-left-close`) вызывает
  `close_this` который сбрасывает SoT (round 2 — source-text guard)
- Удаление legacy resize fields/methods не сломало ничего в runtime path
  (active code path: WorkspaceView start/update/end_resize)

Всё это — gate T281 (live proof). Эта задача специально не заявляет live
готовность.

## Cleanup и долги

- `Cargo.lock` обновлён (compile-only dependency tree changes)
- Не закоммичены unrelated dirty-worktree изменения (per plan: "Preserve
  unrelated dirty-worktree changes. Every commit stages only files listed
  in its task.")

## Известные ограничения / дальнейшие шаги

- A2 (T279): Chat extraction — `SidePanelLeft` split на
  ChatTab/SessionsTab/ProjectSwitcher. Bridge из WorkspaceView убирается.
- A3 (T280): ThreadStore v2 с `workspace_project_state` (project scope
  + active_thread_id), one-way bar migration Project Switcher → left.
- A4 (T281): IPC + focus + dock integration, release build, live proof.

Эта задача (T278 / Slice A1) завершает план-каркас lifecycle + geometry +
rail chrome. Никакого продуктового поведения не добавлено — только
правильная двух-surface архитектура с фокусируемой в будущем таб-роутинг
поверхностью.
