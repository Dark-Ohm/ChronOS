# T227 — Отчёт: JetBrains Mono на самом деле применяется на корне каждого окна

**Дата:** 2026-08-03
**Статус:** Реализация готова, дисциплина-тест в `chronos-ui` green (2/2).
Полный билд `chronos` **заблокирован чужим незакоммиченным WIP** (T219 rail.rs /
T221 view.rs / panels_config.rs в рабочем дереве, см. «Верификация»). Живой
Wayland-прогон (`grim` в обеих темах) **НЕ выполнен** — не из чего собирать.

## Что было

Диагноз из брифа подтвердился гепом по дереву:

- `theme.font_ui` читался ровно в одном месте на весь шелл —
  `system_popup/view.rs` (поэлементно: `:157`, `:254`, `:471`, `:502`, `:551`, `:573`).
- Бар один раз ставил `font_mono` (`bar/mod.rs:140`), левая панель — только
  `font_mono` на tool-карточках (`side_panel_left/tool_card.rs:58,116,146`).
- **Ни одно окно не задавало семейство шрифта на корне.** Все собственные вьюхи
  ChronOS (левая панель целиком, хром правой, уведомления, лаунчер, OSD, док,
  трей, переключатель проектов, терминал) рисовались дефолтным шрифтом GPUI.
  Поле `font_ui` было декоративным.

Проверка «`font_ui` равен JetBrains Mono» оставалась зелёной всё это время —
ровно та ложь, на которой приняли T215 «shell-wide».

## Решение

Шрифт применяется **один раз на корне каждого окна** и наследуется вниз; поле
`font_ui` больше не декоративное. Mono с сохранённым смыслом — там, где он
нужен осознанно, а не «потому что так вышло».

### `crates/ui/src/window_root.rs` (новый)

`WindowRootExt::window_font(&self, theme: &Theme)` — расширение на `Styled`,
ставит `font_family(theme.font_ui)` на корневой элемент окна. Одна точка
применения, наследование вниз. Два теста:

- `every_window_root_uses_window_font` — **тест на дисциплину, а не на факт**
  (как требует приёмка): через `include_str!` перечисляет 14 корней окон и
  требует, что каждый проходит через `.window_font(...)`, и что ни в одном нет
  поэлементного `font_family(theme.font_ui)` / `font_family(font_ui)`.
- `window_font_sets_font_ui` — хелпер реально пинит шрифт.

Hover-полосы (`side_panel_left/hover_strip.rs`, `side_panel_right/hover_strip.rs`)
в список не входят осознанно: это прозрачные безтекстовые hit-поверхности.

### `crates/ui/src/lib.rs`

`pub mod window_root;` + `pub use window_root::WindowRootExt;`.

### Корни окон — `.window_font(&theme)` в 14 местах

`side_panel_left/panel.rs`, `side_panel_right/view.rs`, `bar/mod.rs`,
`notifications/view.rs`, `notifications/history_popup/view.rs`,
`system_popup/view.rs`, `volume_popup/view.rs`, `updates_popup/view.rs`,
`launcher/view.rs`, `osd/view.rs`, `dock/context_menu.rs`, `tray_menu/view.rs`,
`project_switcher/view.rs`, `desktop_terminal/view.rs`.

### `system_popup/view.rs` — поэлементный хром убран

Удалены все шесть `.font_family(font_ui)` и параметры `font_ui` из сигнатур
`header` / `brightness_block` / `power_profile_block` / `gaming_mode_block`.
После корня они маскировали регрессии. `font_family(font_mono)` остался один
(вывод команд) — там моноширинный смысл.

## Верификация

| Проверка | Результат |
|---|---|
| `cargo test -p chronos-ui window_root` | ✅ 2/2 green |
| Дисциплина-тест (14 корней через хелпер, ни одного поэлементного) | ✅ |
| `cargo check -p chronos --lib` | ❌ **не мой блок** — см. ниже |
| `cargo check -p chronos --bin chronos` | ❌ то же |
| `cargo build --release -p chronos` | ❌ то же |
| Live `grim` в обеих темах | ⏳ не выполнен (нет билда) |

**Диагноз блока честно:** `cargo check -p chronos` падает на 5 ошибках, все в
`side_panel_right/*`:

- `rail.rs:213` — `Rc` не реализует `Copy` (x2, **T219** edit-mode rail, не мой код);
- `panels_config.rs:406` — сравнение `&PanelTab` с `PanelTab` (**не мой код**);
- `view.rs:642` — `handle.update` приватный (**не мой код**);
- `view.rs:22` `use crate::edit_mode;` — `mod edit_mode` есть только в `main.rs`
  (bin), в `lib.rs` его нет → E0432 именно на lib-таргете.

Контрольная проба: `git stash push -- crates/app crates/ui` → чистый HEAD →
`cargo check -p chronos --lib` = **0 ошибок** → `pop` → те же 5 ошибок. То есть
поломку внёс незакоммиченный WIP **T219 (rail.rs edit-mode)**, который лег в
рабочее дерево поверх коммитов T216/T217, а не правки T227. Мои изменения
(`window_root.rs`, `.window_font(...)` в корнях, чистка `system_popup`) в списке
ошибок не фигурируют ни одной строкой.

**Порядок раскрытия:** T219/T221 должны закрыться и встать в коммит раньше
T227 — иначе приёмка `cargo test -p chronos --lib` невозможна.

## Следующий шаг

1. Закрыть/закоммитить T219 (rail.rs, panels_config.rs, edit_mode) и T221.
2. `cargo test -p chronos --lib` + `cargo build --release -p chronos` — приёмка T227.
3. Live: `grim` каждого окна из зон в обеих темах, сравнить начертание с
   редактором правой панели. Особое внимание — левая панель.
4. При закрытии поправить строку про T215 в HANDOFF (см. заметку в брифе).

## Файлы

- `crates/ui/src/window_root.rs` (новый, хелпер + 2 теста)
- `crates/ui/src/lib.rs` (`pub mod window_root;` + re-export)
- 14 корневых `render` с `.window_font(&theme)`
- `crates/app/src/system_popup/view.rs` (чистка поэлементного хрома)

**Коммит (после разблокировки):** `ui : apply theme font at every window root (T227)`.
