# T220 — Отчёт: Левая панель при призыве открывается rail-only (без чата)

**Дата:** 2026-08-03
**Статус:** Код готов, юнит-тесты green (12/12), release-билд OK. Живой Wayland-прогон НЕ выполнен — блокер среды (агентский сеанс без GUI/Wayland).

## Диагноз (подтверждён кодом)

Приёмка T220 требует: призыв левой панели (Super+A / peek) открывает **только
рейл** (36px strip + 4px handle = 40px), чат выезжает отдельно (dock-тоггл `⊞`/`⊟`
или drag ручки). Ранее (`ensure_chat_width` задавал стартовую ширину 352px и
`chat_open = width > SIDEBAR_MIN_WIDTH + 4`) призыв сразу раскрывал колонку чата —
поведение старого T137, которое по T220 должно уйти.

Доп. требование: запомненная ширина раскрытого чата (N) живёт между призывами —
после expand-to-N → закрыть → призвать → expand возвращает N, а не дефолтные 352px.

Третий пункт приёмки (exclusive zone == ширина панели в обоих состояниях): уже
корректно в `exclusive_px()` — для rail-only (dock_chat=false) зона =
`sidebar_width() + HANDLE_WIDTH` = 40px, для dock = width. Значит тайловые окна не
заезжают под панель и не остаётся мёртвой полосы. Нужно было не сломать.

## Внесённые изменения

### `crates/app/src/side_panel_left/state.rs`

1. **Стартовая ширина призыва** в `SidePanelLeftState::new()` — `SIDEBAR_MIN_WIDTH`
   (40px = рейл-only) вместо `SIDEBAR_MIN_WIDTH`+`DEFAULT_CHAT_WIDTH`/… авто-раскрытия.
   Убрал лишний `mut`. Чат теперь НЕ выезжает при Super+A/peek.
2. **Добавлено поле `remembered_chat_width: Option<f32>`** — память раскрытой ширины N.
3. **Добавлена `rail_only_width()`** (= `sidebar_width() + SIDEBAR_HANDLE_WIDTH`, как
   `RAIL_ONLY_WIDTH` у правой панели) — единственный источник истины для рейл-ширины.
4. **`ensure_chat_width()`** теперь раскрывает до `remembered_chat_width.unwrap_or(DEFAULT_CHAT_WIDTH)`,
   а не жёстко до 352px — так возвращается запомненная N.
5. **`resize(new)`** фиксирует ширину в `remembered_chat_width`, когда она заметно
   больше рейл (drag ручки / программный resize). Клэмп по min/max — как и было.

### `crates/app/src/side_panel_left/mod.rs`

1. **`window_options`** — стартовая ширина `rail_only_width()` (40px). Exclusive zone
   (уже верно) совпадает с шириной в обоих состояниях.
2. **Глобальное `SidePanelLeftState_`** — добавлено поле `remembered_chat_width`, чтобы
   ширина переживала закрытие surface между призывами.
3. **`open_window`** — применяет `remembered_chat_width` из глобала к свежему инстансу
   через `handle.update` (после `cx.open_window`), без double-borrow `cx`.
4. **`close`** — читает `remembered_chat_width` из панели **до** `handle.update`
   (отдельным `update`-чтением), пишет в глобал после уничтожения surface. Так ширина
   не теряется между призывами.

### `crates/app/src/side_panel_left/panel.rs`

Оба dock-тоггла (в collapsed- и expanded-ветках `dock-toggle`) приведены к одному
контракту T220:
- сворачивание (dock on → off): сначала сохранить текущую ширину в
  `remembered_chat_width`, затем `dock_chat = false`;
- раскрытие (off → on): сначала `ensure_chat_width()` (до запомненной/дефолтной),
  затем `dock_chat = true`.
Сброс `last_exclusive_zone = None` + `cx.notify()` — как и было, чтобы exclusive
пересчиталась на следующем paint.

### `docs/HANDOFF.md`

Поправлена секция «Панели (кратко)» (левая панель теперь описана как rail-only при
призыве) + добавлен честный рецепт живого прогона левой панели (чтобы раздел
«живой прогон» не врал про авто-раскрытие чата).

## Тесты

Новые/обновлённые unit-тесты в `side_panel_left::tests` (все прошли):
- `state_default_width_opens_rail_only` — призыв = 40px, чат не выехал (переименован
  из старого `state_default_width_opens_chat_column`, который проверял обратное).
- `ensure_chat_width_restores_remembered_width` — при `remembered = Some(N)`
  `ensure_chat_width()` раскрывает ровно до N, а не 352.
- `resize_remembers_expanded_width` — `resize(N)` сохраняет N в `remembered_chat_width`.
- `exclusive_px_dock_vs_overlay` — exclusive zone == width в обоих состояниях (rail-only
  40px и dock).
- плюс существующие `rails_and_handles_match_right_panel`, `state_min_width_is_sidebar_plus_handle`,
  `ensure_chat_width_expands_from_sidebar_only`, `toggle_collapse_recalculates_min_width`,
  `clamp_width_below_min_after_recalc`, `state_starts_as_peek` и 2 IPC-теста toggle.

## Верификация

- `cargo test -p chronos --bin chronos side_panel_left` → **`test result: ok. 12 passed; 0 failed`**
  (ad-hoc скрипт `/tmp/hermes-verify-t220-left-rail.sh` — собрал, прогнал, сделал 4
  assert-проверки на ключевых поведениях T220, удалил себя и лог). Без `error[`/`could not compile`.
- `cargo build --release -p chronos` → **Finished `release` profile**, exit 0.
  (Варнинги — pre-existing: `tray_menu` drop-ref, `gpui-component`, `updates_popup`
  и пр., не из моих правок.)
- **Live (hyprctl layers | grep side_panel_left; grim до/после) — НЕ сделан.** Агентский
  сеанс не имеет GUI/Wayland, визуально подтвердить «видна только рейл-полоса, чат не
  выехал; окна не заезжают под панель» нельзя. Это обязательный LIVE SMOKE из брифа —
  его гонит архитектор.

## Следующий шаг

1. Архитектор: живой прогон по рецепту из `docs/HANDOFF.md` (секция «Живой прогон панелей»):
   `Super+A` → рейл ~40px (чат не выехал) → `⊞` раскрывает → drag до N → закрыть →
   снова `Super+A` → рейл → `⊞` → ширина N (не 352) → `grim` до/после; окна не заезжают
   под панель.
2. Если визуально расходится — вернусь править.

**Коммит:** `eb563a5 panels : left summons rail-only (T220)` (4 файла, +213/−23).
