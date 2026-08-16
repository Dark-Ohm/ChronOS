# T265-H — Start menu (вторая поверхность) — Report

**Date:** 2026-08-16
**Role:** FRONTEND. Zone: `bar/widgets/dock.rs` + новый `start_menu/**`.
**Commit:** `6774bc6` `feat(launcher): start menu — second surface on Layer::Overlay, shared model (T265-H)`.

## Status

**Done (code).** Полная верификация прогнана в изолированном ворктри
(`git worktree add ../chronos-t265h-verify 6774bc6`), потому что общее
дерево в этот момент заблокировано чужим WIP (T287-B переписывает
`side_panel_left/tabs/sessions.rs`: `gpui::ListDelegate` не резолвится +
`Popupmenu`→`PopupMenu` опечатка — 2 ошибки не мои). Ворктри после
проверки удалён.

## Решения, зафиксированные до кода (по ask владельца)

1. **Placement — верх-лево, под баром** (не низ-лево как `align-items:flex-end`
   в мокапе). Тикет технически явен: «от кнопки», «падает ровно в колонку
   чата». Мокап — канон *внутренней раскладки*, не позиции на экране.
2. **Full rail** — Places + Categories + user/power из мокапа, всё реюзом.

## Что сделано

### `start_menu/mod.rs` — поверхность и слой

- **`Layer::Overlay`, не `AnchoredPopup`** (рецепт тикета): popup от Top-бара
  рендерится в Top-слое и накрывается Overlay-панелями. Overlay — единственный
  слой выше панелей.
- `anchor: TOP|LEFT`, `exclusive_zone: Some(px(-1.))` (escape-hatch, тот же
  контракт, что `side_panel_left::content_window_options`) + явный
  `margin: (bar_height, 0, 0, 16)` — детерминированная позиция без двойного
  оффсета от exclusive-зоны бара.
- `keyboard_interactivity: OnDemand` (Exclusive запрещён — T264). Поле
  фокусируется при открытии; сиденье композитор даёт после клика в поверхность.
- **Dismiss свой** (grab у layer-поверхности нет): клик мимо — общий
  `popup_click_catcher::open` с дырой `outside_input_regions(menu_bounds)`;
  Esc / повторный клик Start / запуск — через `close_this`/`close`/`toggle`.
- В `open`: `side_panel_left::close` (одна колонка с меню) и
  `side_panel_right::close` **только** если геометрия пересекает
  (`right_panel_overlaps(display_w, state.width, menu_right)` — на 2560 не
  трогает, на 1366 закрывает). Не два Overlay в одном прямоугольнике.

### `start_menu/view.rs` — компактная вьюха над общей моделью

Одна модель, две вьюхи: `StartMenuView` реюзит `search::FuzzySearch`,
`frecency::{cached,rank,record_launch}`, `favorites::{resolve_favorites,
top_recents,index_by_id}`, `grid::{build_categories,filter_by_category,
move_2d}`, `launch::launch`, `launcher_config` (один `launcher.toml`),
`system_actions` + `power`. **Нового состояния/индекса не создано.**

- **Левый рейл** (210px): Places (All Apps / Pinned / Recent / Files, с
  бейджами) + Categories (Main Categories из `build_categories(&all)`,
  клик-фильтр) + footer (user-card: аватар GECOS/`~/.face` + имя + host;
  power-mini: Lock/Sleep/LogOut/Restart/Shutdown — мокапные пять, Hibernate
  остаётся в OSD-шапке). Arm/confirm через общий `crate::power`.
- **Правый main**: компонентный `Input` («Search applications…»), breadcrumb
  (All Apps / Pinned / … / «All Apps › Cat»), скроллируемая сетка
  (5×84px ячеек, иконка+подпись) с 2D-навигацией `move_2d`; Files → честный
  empty-state из мокапа.
- **Клавиатура**: Esc — закрыть, Enter — запустить выбранное,
  стрелки/Home/End/PgUp/PgDn — 2D, фокус-корень `track_focus` + `on_key_down`.
- Подписки на `applications` (live entries) и `launcher_config` (favorite/hide
  из OSD-контекст-меню или страницы настроек мгновенно отражаются в меню).

### Прочее

- `dock.rs`: Start-кнопка `id="dock-start"` → `start_menu::toggle` (не OSD).
- `lib.rs`/`main.rs`: модуль-близнец + `start_menu::init`.
- Два ассета: `icons/lock.svg`, `icons/suspend.svg` (мокап-пати, monoline).
- `Cargo.lock`, `Source/gpui/` — не тронуты.

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos --lib start_menu` | **9 passed; 0 failed** |
| `cargo test -p chronos --lib` | **573 passed; 0 failed** |
| `cargo build --release -p chronos` | **чисто, 5m00s** (78 warnings — pre-existing) |

Прогнано в ворктри `6774bc6` (без чужого WIP). Юниты: `nav_filter`
(All/Pinned-order/query-narrow/Category/Files), `breadcrumbs_are_stable`,
`rail_power_actions_match_mockup_five`, `right_panel_overlaps_only_when_it_reaches_menu`,
`menu_dimensions_reasonable`, `power_icons_resolve_for_every_rail_action`.

В общем дереве `cargo check -p chronos` даёт ровно 2 ошибки — обе в чужом
`side_panel_left/tabs/sessions.rs` (T287-B). Мои файлы: 0 ошибок, 0 варнингов.

## Что НЕ сделано (владелец / дальше)

1. **Live grim + `hyprctl layers`** (спека, обязателен): Start открывает меню
   у кнопки; поиск находит/запускает; Esc/Enter/стрелки; левая панель открыта
   → Start → меню целиком видно, панель не кроет; обычное окно не кроет;
   `hyprctl layers` — меню выше панели в этом прямоугольнике.
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.

## Отчёт одной строкой (честные выборы)

- **Слой** — `Layer::Overlay` + `exclusive_zone: -1` + явный margin (рецепт
  тикета); в коммит-сообщении тикета было «AnchoredPopup» — это противоречит
  его же рецепту, реализовал по рецепту.
- **Клик-мимо** — общий `popup_click_catcher` с дырой под меню (как
  tray/dock/app-menu), дыра вычисляется из детерминированной позиции меню,
  не hyprctl-запроса (меню не центрировано, в отличие от OSD).
- **Навигация/категории в рейле** — текстовые пункты + бейдж, без per-item
  иконок (в мокапе они есть; в ассетах наших нет, плодить 10 svg не стал).
  Иконки только в power-mini (3 готовых + 2 новых).
- **Правый клик в меню** не открывает `app_menu` — его click-catcher якорь
  захардкожен на `chronos-launcher` (`launcher_output_local_origin`), для
  start-menu он был бы неверным; не в scope H.
- **Анимация въезда** (`start-in` мокапа) не делал — не в scope, добавить
  поверх `motion` позже тривиально.
- **OnDemand, не Exclusive**: печатать можно после клика в поле — поведение
  layer-поверхностей, не баг кода.

## Коммит

```
feat(launcher): start menu — second surface on Layer::Overlay, shared model (T265-H)
```

(7 files: `start_menu/{mod,view}.rs` (новые), `icons/{lock,suspend}.svg` (новые),
`bar/widgets/dock.rs`, `lib.rs`, `main.rs`. `Cargo.lock`, `Source/gpui/` — не тронуты.)
