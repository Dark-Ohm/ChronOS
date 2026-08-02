# T200 report — Apply bar appearance live (hot-reload, no kill)

**Роль:** FRONTEND. **Коммит:** `64fc2df` `bar : apply appearance hot-reload (T200)`.
**Зона:** `crates/app/src/bar/{mod,layout_config}.rs`, `state.rs`,
`side_panel_{left,right}/{mod,view,hover_strip}.rs`.
**Параллельные задачи:** T201 (bar agent API — в HEAD `51219ab`), T204
(rail 36px/ghost handle — **не коммичена**, живёт в рабочем дереве).

---

## 1. What applies live (table field → API)

| field | apply | API | live? |
|---|---|---|---|
| `height` | `Window::resize(width × height)` | `window.resize` (`window.rs:2318`) | ✅ |
| `exclusive`/`floating` | floating/`exclusive=false` → zone `0`; иначе `Some(height)` | `set_exclusive_zone` (`window.rs:2005`) | ✅ |
| `radius` | root div `.rounded(px(r)).overflow_hidden()` при `r>0` | render (per-frame) | ✅ |
| `elevation` | `none` → flat; `soft`/`strong` → drop-shadows через `elevation_shadow()` | render | ✅ |
| `edge` top↔bottom | **cold-path** — anchor в `window_options()` при open; mid-session смена → warn «restart shell» | — | ⚠️ restart |
| `width` full | текущий stretch | — | ✅ (status quo) |
| `width` fraction | **не применён** — нет live `set_anchor`/`set_margin` в форке (T198) | — | ⚠️ warn |
| `align`/`margin` | не применены (только при fraction/inset, которых нет) | — | ⚠️ warn |
| widgets L/C/R | уже hot-reload'ились (registry rebuild) | `layout_config::apply` | ✅ |

Всё применяется идемпотентно: `bar::apply_appearance(cx)` вызывается из
`layout_config::apply` на каждый `bar.toml`-watcher fire (300 ms debounce) и
один раз после open в `bar::init`.

## 2. WindowHandle store

`open_on_display` раньше выбрасывал handle (`Ok(_) => true`). Теперь:

```rust
static BAR_WINDOW: OnceLock<Mutex<Option<WindowHandle<Bar>>>> = OnceLock::new();
```

- Handle пишется при open, читается в `apply_appearance` через
  `handle.update(cx, |bar, window, cx| { resize; set_exclusive_zone; set_input_region; })`.
- **Нет** `remove_window`+reopen — это нарушало бы `wayland-window-lifecycle`
  (ghost windows). Один surface, живые resize/zone.
- `set_input_region(None)` вызывается явно: v1 surface == видимая полоса,
  full-surface region корректен (T198 NOTE — API уже есть, `window.rs:2029`).

## 3. Panel gap consumers

`PANEL_EDGE_GAP = BAR_HEIGHT` const сломался бы при live-height. Введён live
источник в **lib-видимом** `crate::state` (atomic f32), потому что `bar` —
bin-only модуль (`mod bar;` в `main.rs`), панели в lib-таргете не могут
`crate::bar`:

- `state.rs`: `bar_height_px()` / `set_bar_height_px()` — AtomicU32 (f32 bits),
  default = `chronos_luau::bar::BAR_HEIGHT` (30). Пишется в `bar::init` (до
  open панелей) и в каждом `apply_appearance`.
- `side_panel_right/mod.rs`: `PANEL_EDGE_GAP` const → `panel_edge_gap()` fn.
- `side_panel_right/view.rs`: resize-ветка читает `panel_edge_gap()`.
- `side_panel_left/mod.rs`: `PANEL_EDGE_GAP` const → `panel_edge_gap()` fn
  (оба места: `window_options` + Render-resize).
- `side_panel_{left,right}/hover_strip.rs`: `STRIP_EDGE_GAP` → `super::panel_edge_gap()`.

Residual (документирован в коде): geometry панелей фиксируется при open —
уже открытая панель не ресайзится при live-height, пока не переоткроется.
Это сознательный v1 (у панелей нет постоянного handle-хранилища на resize
в эту задачу; T194/T204 могли бы добавить).

## 4. Fork changes

**Нет.** Live `set_anchor`/`set_margin` в форке отсутствуют (T198). Для v1
edge остаётся top → `window_options()` cold-path достаточно. Смена edge/width
в конфиге логирует warn (одна строка на apply), применяется при рестарте.

## 5. Verification

```
cargo test -p chronos bar::          → 109 passed
cargo test -p chronos side_panel     → 141 passed
cargo test -p chronos state::        → 5 passed (incl. new round-trip)
cargo test -p chronos                → 208 + 388 passed (lib + bin), 0 failed
cargo clippy -p chronos --all-targets → чисто по зоне (0 warnings в моих файлах)
cargo check -p chronos --all-targets  → Finished, без errors
```

Изоляция: основное дерево на момент проверки содержало чужой WIP T204
(rail/ghost-handle) в тех же файлах панелей. Проверка моей зоны выполнялась в
**отдельном worktree на HEAD (`51219ab`) + мои 8 файлов** (общий
`CARGO_TARGET_DIR`), чтобы T204 не влиял на результат. Зелёное.

**Live shell: `НЕ ПРОВЕРЕНО`** — сессия терминальная, без compositor/Chronos;
grim невозможен. Юнит-уровень зелёный, код-путь идентичен dock-образцу
(resize + set_exclusive_zone уже используются dock'ом в проде).

## 6. Что НЕ сделано

- **hug** (measure→resize loop) — schema принимает, apply трактует как
  full + warn (по T200-брифу).
- **fraction width + align margin** — нет live `set_margin` в форке; cold-path
  не реализован (v1 full). Warn при width != full.
- **live edge flip top↔bottom** — требует fork `set_anchor`; cold-path +
  warn. Отдельный fork-коммит отложен (T200-опция, не обязательный).
- **input_region на «пилюлю»** для fraction — неактуально (fraction не
  применён; full → region = весь surface, `None`).
- **OSD bottom-collision** — edge остаётся top, коллизии нет.
- **theme.toml token overrides, dock-отдельное окно** — вне зоны.
- Живой кадр (см. §5).

## 7. Git-hygiene

Параллельно в дереве живут T204 (не закоммичена). Три файла были смешанными
(`side_panel_right/{mod,view}.rs`, `side_panel_left/mod.rs`): застейджены
**только мои хунки** (mine-only версии из HEAD + мои правки через
`git update-index --cacheinfo`), рабочие файлы T204 не тронуты. Коммит
содержит ровно 9 файлов зоны (8 кода + отчёт); чужой dirt (`RAIL_WIDTH 36`,
ghost handle, `rails_and_handles_match_right_panel`, `agent_api.rs`, docs) не
задет.

## 8. Code review — принятые правки

Ревью (code-reviewer) указало три пункта — все внесены в аменд коммита:

1. **Publish высоты после успешного update.** `set_bar_height_px` вызывался до
   `handle.update` — при Err state расходился с окном. Теперь публикуется в
   `Ok(())` ветке (гап панелей следует *применённой*, а не сконфигурированной
   высоте). Стартовый publish в `bar::init` сохранён (панели открываются раньше
   бара).
2. **Round-trip тест атомарной высоты** (`state.rs`): дефолт == `BAR_HEIGHT`, `set→get`
   возвращает значение, восстановление дефолта — `bar_height_defaults_and_round_trips`.
3. **Dedupe warn-сообщений.** edge/width warn спамил на каждый watcher fire
   (300 ms debounce); `warn_deferred_fields` логирует один раз на значение
   (`DEFERRED_WARNED: OnceLock<Mutex<Option<(BarWidth, BarEdge)>>>`).

Не принято (сознательно): reuse `Theme::elevation_popup()` токенов для тени —
попап-тени заточены под карточки (крупный radius), у бара свой профиль
(полоса на краю экрана); решение задокументировано в `elevation_shadow`.
Поведение при закрытом окне (`Err` → warn) и `Some(px(0.))` vs `None` для
floating — оставлены как есть (эквивалентны, дешевле без fork).
