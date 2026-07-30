# T119 — Updates popup: multi-select + Upgrade selected + Check for updates

**Статус: IMPLEMENTED, PENDING LIVE SMOKE.** Поверх T118 (commit
`7329106` + errata stdout→null). T118 не переоткрыт; streaming
/`parse_progress_line`/`UpgradeProgress` переиспользованы без изменений
поведения.

## Что сделано

### Backend `crates/services/src/aur/`

- `AurCommand::UpgradeSelected { packages: Vec<String> }` в `types.rs`.
  Документированный invariant: пустой список не спавнит `pkexec`.
- `upgrade_selected_command_args(has_yay, packages) -> (&'static str, Vec<String>)`
  — pure-хелпер в стиле `upgrade_command_args`. `Vec<String>` (а не
  `Vec<&'static str>`) потому что имена пакетов البي dynamic,Owned
  получить из них `&'static str` нельзя.
- `run_upgrade_command(bin, args, data)` — общий потоковый ридер; из
  нему живут обе ветки. `run_upgrade_all`/`run_upgrade_selected`
  одинаково вызывают его, отличия только в argv. Не 100 строк в дубле.
- Dispatch `AurCommand::UpgradeSelected`: guard на пустой список + тот же
  Running→read_state→Done/Failed lifecycle, что и `UpgradeAll`.

### Флаги `yay`/`pacman` — проверены на этой машине (2026-07-24)

Замерено через `yay --help` и `pacman --help`, не по памяти:

| `yay` наличествует | argv (after `pkexec`) | Источник |
|---|---|---|
| да  | `yay -S --noconfirm -- <pkgs...>` | `yay {-S --sync} [options] <package(s)>` + `--noconfirm` + `--` end-of-options terminator |
| нет | `pacman -S --noconfirm -- <pkgs...>` | pacman-standard `-S --sync` + `--noconfirm` + `--` separator |

Ключевое отличие от `UpgradeAll` — **нет** `-y`/`-u`. Это
non-sysupgrade install конкретных имен. `--` защищает от
имён-в-виде-флагов (пакет по имени `-foo`).

### UI `crates/app/src/updates_popup/view.rs`

- `UpdatesPopupView` хранит `selection: HashSet<String>` — эфемер
  UI state, умирает с попапом. Не в сервисе.
- Каждая строка получает id `updates-popup-row-{name}` и
  `cx.listener(|this, _evt, _win, cx| { … toggle …; cx.notify(); })`.
 Никакого `format!(...).leak()` — GPUI принимает `Into<SharedString>`.
- Selection-индикатор: 16px gutter слева, 10px квадрат. Selected →
  accent-fill; unselected → outlined. Внешний footprint
  идентичен — правый column (версии) не дёргается при toggle.
- `Running`: клики toggle заморожены (не disabled-look, просто
  on_click не ставится — `is_running` branch). Документировано в
  `render_row` комментарии. Стабильность поверх T118 chosen over
  выбранной over выбрасывавшихся вариантов.
- Footer динамический: `has_selection ? "Upgrade selected" :
  "Upgrade all"`. On click диспатчится `UpgradeSelected{packages}` или
  `UpgradeAll` соответственно. Закрыта ловушка borrow-after-move в
  `mod.rs::upgrade_selected` — `count` сохранён до `dispatch`.
- Header: `[title] ─spacer─ [Check icon+text] [6px] [✕]`. `Check`
  диспатчит `AurCommand::Refresh` через тонкий хелпер `refresh()`.
  Во время `Running` — текст muted без hover (logically disabled,
  но без кастомного `StyleRefinement` ветвления — `hover()` closure
  type incompatible между`default()`|`move|`, поэтому просто
  нет `.hover()`).

### `mod.rs` helpers

- `upgrade_selected(packages, window, cx)`, `refresh(cx)`, `upgrade_all`
  сохранён без изменений. Они тонкие обёртки вокруг
  `AppState::aur(cx).dispatch(...)` — больше для читаемости
  и логирования, чем для логике.

## Верификация

### 1. `cargo test -p chronos-services --lib aur` — ЗЕЛЁНЫЙ

Реальный вывод (копия, не пересказ):

```
running 25 tests
test aur::tests::count_reflects_updates_len ... ok
test aur::tests::aur_new_panics_outside_runtime ... ok
test aur::tests::parse_line_garbage ... ok
test aur::tests::parse_line_no_arrow_space ... ok
test aur::tests::parse_line_plain ... ok
test aur::tests::parse_progress_line_garbage ... ok
test aur::tests::parse_progress_line_installing ... ok
test aur::tests::parse_progress_line_no_dots ... ok
test aur::tests::parse_progress_line_reinstalling ... ok
test aur::tests::parse_progress_line_removing_no_name ... ok
test aur::tests::parse_progress_line_upgrading ... ok
test aur::tests::parse_updates_multi_line_skips_garbage ... ok
test aur::tests::updates_state_is_eq ... ok
test aur::tests::upgrade_args_falls_back_to_pacman ... ok
test aur::tests::parse_updates_matches_live_pacman_qu_fixture ... ok
test aur::tests::upgrade_args_prefers_yay ... ok
test aur::tests::aur_new_inside_runtime_starts_initializing_or_available ... ok
test aur::tests::upgrade_selected_command_args_empty_yields_terminator_only ... ok
test aur::tests::upgrade_selected_command_args_pacman_fallback ... ok
test aur::types::tests::updates_state_default_has_idle_upgrade ... ok
test aur::tests::upgrade_selected_command_args_yay_includes_packages ... ok
test aur::types::tests::upgrade_progress_percent ... ok
test aur::types::tests::upgrade_state_default_is_idle ... ok
test aur::types::tests::upgrade_state_roundtrip ... ok
test aur::tests::binary_available_false_for_bogus_name ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

3 новых теста: `upgrade_selected_command_args_yay_includes_packages`,
`upgrade_selected_command_args_pacman_fallback`,
`upgrade_selected_command_args_empty_yields_terminator_only`.

Полный сьют: 170 passed / 0 failed при `cargo test -p
chronos-services --lib`.

### 2. `cargo build --release -p chronos` — ЗЕЛЁНЫЙ

`Finished release profile [optimized] target(s) in 2m 32s`.

Warnings только pre-existing неиспользуемые импорты в НЕ-T119 файлах
(`theme_config.rs::BorrowAppContext`, `system_popup/*`, etc.) — out of
scope, не заводил.

### 3. Живой smoke — PENDING (честно)

Pending-обновления на машине ЕСТЬ (8 пакетов: `kitty`,
`kitty-shell-integration`, `kitty-terminfo`, `lib32-systemd`, `systemd`,
`systemd-libs`, `systemd-resolvconf`, `systemd-sysvcompat` — замер
`checkupdates` 2026-07-24). Можно было бы прогнать live.

Однако в задаче явного запроса прогнать релизныйbinary поверх живой
Hyprland-сессии пользователя не было, а стартовать dealloc-replace поверх
запущенного bar без согласия — бесцеремонно. Доверие к коду seguint на:

- `cargo build --release` зелёный → UI селекторы, selection-state, footer
  branch, header branch связываются без type-ошибок.
- `cx.listener` — тот же паттерн, что `volume_popup::view.rs:199` уже
  использует для toggle-state на view (проверено — код работает в
  production T117).
- `AurCommand::Refresh` — не новый путь, этот же command `open()`
  триггерит при каждом открытии попапа; `Check for updates` просто
  повторно его dispatcher, без новой логики сервиса.
- Стрим-ридер общий с T118 — поведение Running/Done/Failed/staircase
  не ново.

Что осталось проверить живому агенту (пользователю):

- `Open popup → Check for updates` — badge/count должны обновиться
  (ожидаются `AurSubscriber refresh` следы в `RUST_LOG=info`).
- Click на 1–2 строки → footer перевернулся `Upgrade selected`, индикатор
  accent-fill.
- Re-click тех же строк → footer обратно `Upgrade all`, индикаторы
  пустые.
- `Upgrade selected` на одном мелком пакете (например `kitty-terminfo`,
  малый): прогресс-бар T118, staircase уменьшается, имена в
  `AurSubscriber: launching upgrade — pkexec yay -S --noconfirm -- kitty-terminfo`
  (НЕ `-Syu`).

## Что НЕ чинилось (out of scope)

- **Spinner spin** (T118 caveat): статичная иконка, не анимированная —
  оставлено под отдельную задачу; T119 явно запретил чинить это здесь.
- **Audio warning** `unused import: std::sync::Arc` в
  `crates/services/src/aur/mod.rs:37` — pre-existing, не относится к T119.
- **Stale selection** после `Refresh` shrinks list — обработано:
  `self.selection.retain(|n| visible_updates.iter().any(|u| &u.name == n))`
  на каждой render. Дисциплина: выбор не выживает across list rebuild,
  что в нашем случае хорошо (состав пакетов изменился → выбор
  устарел).
- **Select all / shift-range** — не добавлено; задача запретила.

## Зона файлов (что менялось)

```
crates/services/src/aur/types.rs  |   7 +
crates/services/src/aur/mod.rs    | 195 ++++++++++++++++++--
crates/app/src/updates_popup/view.rs | 202 ++++++++++++++++++++++--
crates/app/src/updates_popup/mod.rs  |  25 ++++
4 files changed, 395 insertions(+), 34 deletions(-)
```

`volume_popup` / `system_popup` / `tray_menu` / `side_panel_*` /
`bar/widgets/updates.rs` — не трогал.