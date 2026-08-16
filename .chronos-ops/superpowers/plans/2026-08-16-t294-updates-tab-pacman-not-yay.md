# T294 — вкладка Updates справа: ставить через pacman, AUR только показать

Дата: 2026-08-16
Статус: ПЛАН (чекпоинт). Код не трогаем до аппрува.

## Контекст (сверено с деревом master cc1f76c7)

- **T293 (Notifications) в git НЕТ** → по правилу самой спеки Updates садятся
  **сразу после `System`** (не после Notifications).
- **T290/T291 в master**: Display на правой рельсе (T296 `a2c072f`), SystemTab (T291).
- Привилегированный путь сейчас в `crates/services/src/aur/mod.rs`:
  `upgrade_command_args(has_yay)` (382) → `pkexec yay|pacman -Syu --noconfirm`,
  `upgrade_selected_command_args(has_yay, pkgs)` (411) → `pkexec yay|pacman -Sy
  --noconfirm -- <pkgs>`. Обе зовут `binary_available("yay")` в
  `run_upgrade_all` (437) / `run_upgrade_selected` (456).
- Попап: `crates/app/src/updates_popup/{mod.rs,view.rs}`. Бар-виджет
  `bar/widgets/updates.rs` → `updates_popup::toggle`. `main.rs`: `mod updates_popup;`
  (28) + `updates_popup::init(cx);` (99).
- `crates/ui/src/window_root.rs` ROOTS содержит
  `("updates_popup/view.rs", include_str!(...))` — `include_str!` на удалённый файл
  **ломает сборку**, обязателен к удалению.

## Цель

1. Новая живая правая вкладка `PanelTab::Updates` (id `"updates"`, label
   `"Updates"`), список update'ов вынести из попапа в общий рендер, попап снести.
2. Применить апдейты **только** pacman'ом (`pkexec pacman …`), AUR — display-only
   с hover-подсказкой про yay. Ни одного `yay` в apply-пути.
---

## Задачи

### Task 1 — Сервис: контракт pacman-only (`crates/services/src/aur/{mod.rs,types.rs}`)

1. `upgrade_command_args(has_yay: bool)` → **без параметра**:
   `pub fn upgrade_command_args() -> (&'static str, Vec<&'static str>)`,
   всегда `("pkexec", vec!["pacman", "-Syu", "--noconfirm"])`.
2. `upgrade_selected_command_args(has_yay, pkgs)` → **без `has_yay`**:
   `pub fn upgrade_selected_command_args(pkgs: &[String]) -> (&'static str, Vec<String>)`,
   всегда `("pkexec", vec!["pacman", "-Sy", "--noconfirm", "--", …pkgs])`.
3. `run_upgrade_all(data)` / `run_upgrade_selected(pkgs, data)` — убрать
   `binary_available("yay")`; вызывать новые сигнатуры.
4. Guard пустого списка в `dispatch(AurCommand::UpgradeSelected)` (177) **оставить** —
   это и есть no-op + warn про выбранные только AUR.
5. Док-комменты: «The one privileged path» (25-33) и
   `AurCommand::UpgradeAll/Selected` в `types.rs` (84-93) — переписать на
   `pkexec pacman`. Греп `yay` в apply-пути (`run_upgrade_*`,
   `upgrade_*_command_args`) → **ноль**; `yay -Qua` в `read_aur` (307-318) остаётся.

Тесты:
- `upgrade_args_prefers_yay` / `upgrade_args_falls_back_to_pacman` → один:
  `upgrade_command_args() == ("pkexec", ["pacman","-Syu","--noconfirm"])`.
- `upgrade_selected_command_args_yay_includes_packages` / `_pacman_fallback` /
  `_empty_yields_terminator_only` → переписать под новую сигнатуру: всегда
  pacman, пакеты дописываются, «yay на PATH не меняет argv» (параметр удалён —
  проверяется тем, что сигнатура его не принимает).
### Task 2 — Живая вкладка: рендер вынести, попап снести

2a. **Новый модуль** `crates/app/src/updates_list.rs` — общий рендер:
- геометрия/константы из `updates_popup/view.rs` (`ROW_PY/ROW_PX`, `SELECTION_GUTTER`,
  `MAX_LIST_H` и т.д.);
- `render_updates_header`, `render_updates_list`, `render_updates_footer`,
  `render_row` — собирают список по `&UpdatesState` + `selection`, принимают
  `cx` типизированным на вкладку.
- **Секции** «Repos» / «AUR» (заголовки в списке, чтобы источник был виден).
- **AUR-строка**: hover → маленькая подсказка (не `PopupMenu`), текст:
  `AUR package — install updates in a terminal with yay.` +
  `Example: yay -S <name>` (EN). Клик по AUR-строке НЕ переключает selection,
  НЕ запускает upgrade.
- **Official-строка**: чекбокс/клик как сейчас → selection. Нет yay-подсказки.
- Footer: «Upgrade all» / «Upgrade selected» по выбору **только official**;
  в `UpgradeSelected` уходит только official-имя. Если в последний момент
  selected пуст (только AUR) — не дергать pacman без пакетов (сервисный guard
  страхует).
- Пустой стейт — тот же хелпер, что T269 (как `EmptyTab`), с той же фразой.

2b. **Добавить вкладку в инфраструктуру**:
- `tabs.rs`: вариант `Updates` после `System`; `ALL` → `[PanelTab; 19]` (Updates на
  index 1); `id="updates"`; `parse_id("updates")`; `label="Updates"`;
  `icon_path` → иконка как у бар-виджета (см. ниже); `preferred_content_width` ~420,
  не resizable.
  - **Иконка**: спека «как у бар-виджета», в скобках допускает
    `arrows-clockwise.svg`. Бар реально рисует `icons/arrow-up.svg`
    (`bar/widgets/updates.rs:30`). Обе в `assets.rs`. Рекомендация:
    `icons/arrow-up.svg` (буквальное «как у виджета»); `arrows-clockwise.svg` —
    валидная альтернатива. Оба уникальны в `icon_path` — тест пройдёт.
- `for_mode` **оба** режима: `Updates` сразу после `System`.
  Dev: `System, Updates, Files, Preview, HyprlandBinds, AcpSettings, Display,
  EditorSettings` (len 8). Gamer: `System, Updates, Library, Captures,
  AcpSettings, Display, EditorSettings, HyprlandBinds` (len 8).
- `panels_config.rs`: `default_dev_top` / `default_gamer_top` — `"updates"` сразу
  после `"system"`.
- `tab/mod.rs`: `pub(crate) mod updates;`; `TabContent::Updates(Entity<UpdatesTab>)`;
  в `create`: `PanelTab::Updates => TabContent::Updates(cx.new(|cx| UpdatesTab::new(cx)))`.
- `tab/updates.rs` (новый): `UpdatesTab { selection: HashSet<String>,
  scroll: ScrollHandle }`; `new` хостит свою подписку
  `state::watch(AppState::aur(cx).subscribe(), …)` → `cx.notify()` (как `DisplayTab`,
  т.к. глобального watcher'а попапа больше нет); `Render` — `.id("updates-tab")
  .window_font(&theme).size_full().flex_col()` + общий рендер из 2a.
- `view.rs`: две разметки `TabContent::Updates(entity) => col.child(entity.clone())`
  (в `render` 634-659 и в `tab_entity_id`).

2c. **Снести попап**:
- удалить `crates/app/src/updates_popup/` (mod.rs, view.rs).
- `window_root.rs` ROOTS: удалить запись `updates_popup/view.rs` (иначе сборка падает).
- `main.rs`: `mod updates_popup;` и `updates_popup::init(cx);` — удалить.
- Греп `updates_popup::` → 0 в `crates/`.

### Task 3 — Бар-виджет → вкладка (`bar/widgets/updates.rs`)

Виджет остаётся (счётчик). Убрать canvas/bounds-захват (попапу не нужна привязка):
- `on_click`/`on_mouse_down` вместо `updates_popup::toggle(anchor, parent, …)` →
  `crate::side_panel_right::select_tab(PanelTab::Updates, cx)` с
  `if crate::edit_mode::is_active(cx) { return; }`.
- Тест `describe_*` не трогаем.

### Task 4 — Обновить тесты счётчиков/инвентаря

- `tabs.rs`: `all_has_eighteen_tabs_in_fixed_order` → 19 с Updates на index 1;
  `developer/gamer_rail_is_seven*` → 8 и включают Updates ровно раз;
  отдельные `parse_id("updates")`; `every_tab_has_a_non_empty_label`/
  `distinct_icon_path`/`nonempty_placeholder_description` пройдут автоматически,
  но у `Updates` должен быть `placeholder_description` (добавить строку в
  `tab/mod.rs::placeholder_description`, уникальную).
- `panels_config.rs`: `resolve_grouped_uses_config_values` (top.len 5→6,
  top[1]==Updates) и любые тесты дефолтов dev/gamer_top.
- `tab/mod.rs` тесты на exhaustiveness/placeholder — `create` получает явную руку
  Updates; placeholder-тесты итерируют `ALL` и берут `placeholder_description`.
- `bar` тесты — без изменений (describe не зависит).

### Task 5 — Docs (после зелёных тестов)

- `docs/ARCHITECTURE.md` упоминает попап updates и `pkexec yay|pacman` →
  поправить под pacman-only и вкладку.
- Коммит: `feat(updates): tab uses pacman to apply, AUR is display-only (T294)`.

## Верификация

```sh
cargo test -p chronos-services --lib aur
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib updates
cargo test -p chronos --lib bar
cargo test -p chronos-ui --lib        # window_root (ROOTS без updates_popup)
cargo build --release
```

Live: клик счётчика на баре → правая вкладка Updates. Upgrade all → `pkexec
pacman -Syu`, AUR в списке остаётся. Hover AUR → подсказка про yay. Попапа в
`hyprctl layers` нет. Grim репо-строка + AUR+tooltip.

## Нельзя

- `pkexec yay` в любом apply. Прятать AUR. Автооткрывать терминал с yay.
- `Source/gpui/`, `Cargo.lock`. Второй попап. Rustfmt всего `side_panel_right/`
  (как в T296).