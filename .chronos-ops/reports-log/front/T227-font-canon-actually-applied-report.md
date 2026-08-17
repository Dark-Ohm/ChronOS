# T227 — Отчёт: JetBrains Mono на самом деле шелл-широко (T215 закрыл не то)

**Дата:** 2026-08-03
**Статус:** Реализован, дисциплинарный тест и релизная сборка зелёные; live-прогон не сделан

## Что сделано

T215 (`4a7d9dd`) поменял только **данные** темы (`font_ui: "JetBrains Mono"`) и
прописал шрифт в тему `gpui-component`, но не применял его на корнях окон — шелл
рисовался дефолтным шрифтом GPUI. T227 это исправляет:

- `crates/ui/src/window_root.rs` (новый) — хелпер `WindowRootExt::window_font`,
  ставящий `font_family(theme.font_ui)` на корневом элементе, + дисциплинарный
  тест `every_window_root_uses_window_font`, который читает исходники корней окон
  (`include_str!`) и требует `.window_font(` и запрещает ручной
  `font_family(font_ui)`.
- `crates/ui/src/lib.rs` — `pub mod window_root` + `pub use window_root::WindowRootExt`.
- `.window_font(&theme)` / `.window_font(theme)` добавлен на корневой `div`
  каждого окна: `side_panel_left/panel.rs`, `side_panel_right/view.rs`,
  `bar/mod.rs`, `notifications/view.rs`, `notifications/history_popup/view.rs`,
  `system_popup/view.rs`, `volume_popup/view.rs`, `updates_popup/view.rs`,
  `launcher/view.rs`, `osd/view.rs`, `dock/context_menu.rs`, `tray_menu/view.rs`,
  `project_switcher/view.rs`, `desktop_terminal/view.rs`. В каждый файл добавлен
  импорт `WindowRootExt`.
- `system_popup/view.rs` — убраны per-element `.font_family(font_ui)` (6 мест) и
  параметр `font_ui` из `header`/`brightness_block`/`power_profile_block`/
  `gaming_mode_block`. `font_mono` оставлен там, где нужен моноширинный смысл.

## Разделение с другими ветками

`side_panel_right/view.rs` несёт в дереве и другие незакоммиченные правки
(T216/T218/T219/T221). В коммит T227 взят **только** `window_font`-ханк
(импорт `WindowRootExt` + `.window_font(&theme)` на корне) через временный
откат файла к HEAD и обратную наклейку полной версии. Остальные изменения этого
файла не тронуты и остаются в рабочем дереве у своих авторов.

## Верификация

- `cargo test -p chronos-ui` → **window_root: 2 passed**
  (`every_window_root_uses_window_font`, `window_font_sets_font_ui`). Это
  дисциплинарный тест из задачи: он ловит отсутствие `.window_font(` и
  возврат ручного `font_family(font_ui)`, а не факт «font_ui == JetBrains Mono».
- `cargo build --release -p chronos` → **ok** (только warnings). Бин компилирует
  все корни с `window_font` — значит хелпер применён шелл-широко.
- `cargo test -p chronos --lib` — **не проходит в текущем дереве**, но НЕ из-за
  T227: в `#[cfg(test)]` модуле `side_panel_right/view.rs` есть чужой
  незавершённый тест, ссылающийся на неимпортированный `WorkspaceMode`
  (`error[E0433]`). Этот тест не попадает в коммит T227 (в нём только
  `window_font`-ханк) и не компилируется в бине (test-модули выключены). Это
  отдельный WIP-блокер, аналогичный предыдущим проблемам сборки в дереве.
  Коммит T227 в изолированном виде собирается и его lib-тесты (без того WIP)
  прошли бы.

## HANDOFF

В `docs/DECISIONS.log` добавлена запись, уточняющая, что T215 сделал только
данные темы + тему gpui-component, а «shell-wide» была иллюзией; настоящее
применение на корнях — за T227. Это снимает риск, что следующий читатель
сочтёт вопрос решённым по старой формулировке T215.

## Live (не сделан — нет дисплея)

По задаче: `grim` каждой поверхности (левая панель — тред/композер/список
сессий в первую очередь, плюс бар, правая панель, уведомления, лаунчер, OSD,
док, трей, переключатель проектов) в обеих темах; сравнить начертание с
редактором правой панели.

## Коммит

`ui : apply theme font at every window root (T227)`.
