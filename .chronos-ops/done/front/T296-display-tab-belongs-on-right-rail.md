# T296 — вкладка Display на **правой** рельсе

**Статус:** DONE (2026-08-16). Код `a2c072f`. Live `+` владельца.
**Приоритет:** P1 — T290 посадил Display налево; владелец: правая рельса.
**Роль:** FRONTEND. `wt-t290` или новый от `81fd7cb`.
**Не параллелить** с T285 только в `tabs/mod.rs` слева (снять Display).
`chat.rs` не трогать.

**Канон панелей (владелец 2026-08-16):** слева только ИИ (Chat / Sessions /
Project / Plan / Tools агента). Справа — ежедневное и ОС (System, Display,
Files, настройки шелла). Display = настройки дисплея; v1 яркость + фон,
дальше расширяется на этой же правой вкладке, не новой слева.

T290 (`81fd7cb`) оставить: попап снесён, `gaming_mode` в корне, wallpaper
не на System, иконка `rail-display.svg`, T291-E `refresh_windows`.
Меняется **сторона**.

## Стало

| Что | Сейчас (T290) | Надо |
|---|---|---|
| Вкладка | `LeftTab::Display` | `PanelTab::Display` |
| Рельса | левая, после Project | **правая, нижняя группа, над настройками шелла** |
| Бар `system` | `side_panel_left::select_tab(Display)` | `side_panel_right::select_tab(PanelTab::Display)` |
| Контент | `side_panel_left/tabs/display.rs` | `side_panel_right/tab/display.rs` (переезд файла, не копия) |

Рельса (T219, две группы + spacer):

```
top:    System, Files/Preview/…          (как сейчас)
        ── spacer ──
bottom: Display                          ← эта вкладка
        editor_settings (System settings)
        [T292 workspace mode — НЕ вкладка, не этот тикет]
        dock ⊞/⊟
```

Display — **первая кнопка нижней группы**, сразу над настройками шелла.
Не верх после System. Не под settings. Не вместо T292.

Дефолты `panels_config.rs`:
`default_dev_bottom` / `default_gamer_bottom` = `["display", "editor_settings"]`.
Живой `~/.config/chronos/panels.toml` без `display` — вставить `display`
перед `editor_settings`, остальное не затирать.

`ALL.len() == 18`. `for_mode` оба режима: Display есть в наборе (перед
EditorSettings). Тесты `all_has_seventeen_*`, `developer_rail_is_six_*`,
`gamer_rail_is_six_*`, дефолты `panels_config` — поправить (+1).

`TabContent::create`: живая рука `Display(Entity<DisplayTab>)`, не
`EmptyTab`. Ширина как у T290: 440, не resizable.

Слева: выкинуть `LeftTab::Display` из enum / `PRIMARY_TABS` / inventory /
`workspace_view` (`display` field, `ensure_display`). Левая рельса снова
`Project, Sessions, Chat, …`.

Бар: виджет `system` (hexagon, бывшая яркость / T290 клик в Display) —
**снести**. Вход в Display = правая рельса, не иконка на баре.

- Выкинуть из `KNOWN` / дефолтного `right` в `bar/layout_config.rs`
  и из `instantiate` / `widgets/mod.rs`.
- Не регистрировать. Файл `bar/widgets/system.rs` удалить, если греп
  `SystemWidget` пуст.
- Тесты `layout_config`, что ждут `"system"` в default right — поправить.
- Живой `~/.config/chronos/bar.toml`: неизвестное имя и так дропается —
  не писать томл за юзера. После рестарта иконки нет.
- `battery` не трогать. Попап не воскрешать.

## Нельзя

- Возвращать `system_popup/`.
- Вторую копию wallpaper на System.
- Power/gaming на Display (они на System, T291).
- `Source/`, `Cargo.lock`, T285 `chat.rs`.
- Rustfmt всего `side_panel_right/`.

## Верификация

```
cargo test -p chronos --lib side_panel_left
cargo test -p chronos --lib side_panel_right
cargo test -p chronos-ui --lib
```

Live: на баре hexagon/яркости нет. Display — нижняя группа правой рельсы,
над System settings. Клик по ней → яркость + wallpaper. Слева монитора нет.
`hyprctl layers` — без system_popup. Grim бар + правая рельса.

## Коммит

`fix(right-panel): Display tab lives on the right rail (T296)`
