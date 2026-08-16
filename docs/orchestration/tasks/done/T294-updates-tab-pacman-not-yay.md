# T294 — вкладка Updates справа: ставить через pacman, AUR только показать

**Статус:** SPEC. Не выдавать до чекпоинта.
**Приоритет:** P2 IA + смена привилегированного пути.
**Роль:** FRONTEND + SERVICES (`crates/services/src/aur/**`).
**Не параллелить** с T293 / T291 / T289 (те же `tabs.rs`, `TabContent`,
`panels_config`).

## Сейчас

Бар `updates` → `updates_popup`. Список: Official (`checkupdates`/`pacman
-Qu`) + AUR (`yay -Qua`). Кнопка Upgrade → `pkexec yay -Syu` если yay
на PATH (`upgrade_command_args`, `aur/mod.rs:382`). То же для selected
(`yay -Sy`).

## Вкладка

Как T293, но Updates:

`PanelTab::Updates` — id `"updates"`, label `"Updates"`, иконка как у
бар-виджета (стрелки / `arrows-clockwise.svg`).

- `ALL`, `parse_id`, `for_mode` **оба** режима: после Notifications
  (если T293 уже в git) иначе после System.
- `default_dev_top` / `default_gamer_top` — тот же слот.
- Живая вьюха `tab/updates.rs`, не EmptyTab.
- Список вынести из `updates_popup/view.rs` в общий рендер. Попап снести.

Бар-виджет остаётся (счётчик). Клик:
`side_panel_right::select_tab(PanelTab::Updates)` — не `updates_popup::toggle`.

Тосты/OSD не при чём.

## Контракт обновления (главное)

| | Official (репо) | AUR |
|---|---|---|
| Показывать в списке | да | да (`yay -Qua`, как сейчас; нет yay — секция честно пустая) |
| Кнопка Upgrade / selected | **только** они | **нет** |
| Команда | всегда `pkexec pacman …`, **никогда** `yay` | — |

`upgrade_command_args` больше не принимает `has_yay`. Всегда:

- All: `pkexec pacman -Syu --noconfirm`
- Selected: `pkexec pacman -Sy --noconfirm -- <official pkgs>`

`UpgradeSelected` с пустым official-списком (выбрали только AUR) — no-op
+ warn, не `pacman` без пакетов и не yay-picker.

Тесты `upgrade_command_args` / `upgrade_selected_command_args`
переписать: yay на PATH не меняет argv. Греп `yay` в apply-пути
(`run_upgrade_*`, `upgrade_*_command_args`) — ноль. `yay -Qua` в
`read_aur` остаётся.

## Hover на AUR

Наведение на строку `UpdateSource::Aur` — маленькая подсказка (tooltip /
hover card), не полный `PopupMenu` с пунктами-пустышками.

Текст (EN, как остальной UI), смысл фиксирован:

`AUR package — install updates in a terminal with yay.`
`Example: yay -S <name>`

Клик по AUR-строке **не** запускает upgrade. Честно (T246).

Official-строки: hover без этой подсказки; клик/checkbox как сейчас
для selected official.

Секции в списке: «Repos» / «AUR», чтобы источник был виден без hover.

## Нельзя

- `pkexec yay` в любом apply.
- Прятать AUR из списка.
- Автооткрывать терминал с yay (только текст).
- `Source/gpui/`, `Cargo.lock`.
- Второй попап «на всякий».

## Верификация

```
cargo test -p chronos-services --lib aur
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib updates
```

Live: клик счётчика на баре → вкладка Updates. Upgrade all гоняет
pacman (pkexec), AUR в списке остаётся. Hover AUR — подсказка про yay.
Попапа нет. Grim репо-строка + AUR+tooltip.

## Коммит

`feat(updates): tab uses pacman to apply, AUR is display-only (T294)`
