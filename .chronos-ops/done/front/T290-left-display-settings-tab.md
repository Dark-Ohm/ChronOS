# T290 — вкладка Display слева: яркость + waytrogen

**Статус:** SUPERSEDED by T296. Попап снесён, сторона была левая; Display справа закрыл T296.
**Приоритет:** P2 IA.
**Роль:** FRONTEND. Левая рельса + перенос карточек.
**Не параллелить** с T285/T286/T288 (`side_panel_left`).

## Куда что

| Что | Сейчас | Стало |
|---|---|---|
| Яркость | `system_popup` ← бар `system` | левая вкладка **Display** |
| Waytrogen / Next wallpaper | правый `SystemTab` + `wallpaper_card.rs` | та же вкладка Display |
| Power profile | бар-попап → **T291** правый System | не здесь |
| Gaming Mode | бар-попап → **T291** правый System | не здесь |

После переноса в `system_popup` не остаётся тела. **Попап снести.**
Бар-виджет `system` не врать (T246): клик →
`side_panel_left::select_tab(LeftTab::Display)`. Индикатор на баре
(иконка / %) можно оставить, отдельного окна больше нет.

## Новая вкладка

`LeftTab::Display` в `side_panel_left/tabs/mod.rs`:

- Не resizable. `preferred_panel_width` = 440 (как Project).
- Label `"Display"`.
- Иконка: новый `icons/rail-display.svg` **или** свободный существующий
  (`rail-api` / `rail-binds`), если не занят левой рельсой. В отчёте —
  какой файл. Не отбирать иконку Chat/Sessions/Project.
- В `PRIMARY_TABS` — **после Project, до Sessions**
  (`Project, Display, Sessions, Chat, …`).
- Не `BOTTOM_TAB`. Archive остаётся один снизу.

Протащить variant через все `match LeftTab` (label/icon/width/inventory
тест `all_tabs_inventory_is_complete`, `workspace_view.rs` render).
Контент — живая вьюха, не Slice B/C shell «Coming later».

`workspace_view.rs`: рука `LeftTab::Display` как у Project/Sessions
(`ensure_display`), не через `ensure_shell`.

## Содержимое вкладки

1. **Яркость** — та же семантика, что `brightness_block` в
   `system_popup/view.rs` (слайдер, % , latest-wins / debounce,
   `AppState::brightness`). Не новый слайдер «на глаз». Не спавнить
   ddcutil на каждый сэмпл (`slow-service-dispatch`).
2. **Wallpapers** — перенести вызов `render_wallpaper_card` из
   `SystemTab`. Хелпер можно оставить в
   `side_panel_right/wallpaper_card.rs` или сдвинуть в
   `crates/app/src/wallpaper_card.rs`. С правого System карточка
   **исчезает**.

Кит: слайдер — существующий паттерн яркости; кнопки waytrogen —
`Button` кита, если трогаем разметку.

## Бар

`bar/widgets/system.rs`: `on_click` больше не `system_popup::toggle`.
Открыть левую Display. Модуль `system_popup/` удалить, если греп пуст
(mod + init + close). Висячий `system_popup::` — отказ.

## Нельзя

- Тащить power/gaming налево (это T291 / правый System).
- ACP / composer / T288 cwd.
- `Source/gpui/`, `Cargo.lock`.
- Вторая копия `wallpaper_card` «на всякий» на System.

## Тесты

- `PRIMARY_TABS` содержит Display ровно один раз, inventory complete.
- Display не resizable, width 440.
- `tab_select_transition` на Display ведёт себя как на Project (fixed).

Не тестировать «хелпер вернул то, что хелпер вернул».

## Верификация

```
cargo test -p chronos --lib side_panel_left
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib bar
```

Live: клик system на баре → левая Display, яркость живая.
Waytrogen/Next на Display, на правом System карточки нет.
Попапа System нет в слоях (`hyprctl layers` / clients).
Power/gaming по-прежнему на правом System (T291). Grim.

## Коммит

`feat(left-panel): Display tab takes brightness and wallpapers (T290)`
