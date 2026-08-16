# T265-D — лаунчер: контекст-меню и Desktop Actions

**Статус:** BLOCKED — после T265-C в git.
**Приоритет:** P2.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND. Меню — наш `gpui-component::PopupMenu`, как pin.

## Задача

Правый клик по клетке/строке — **одно** меню, не второе окно рядом с
`pin_menu`. Расширить `crates/app/src/launcher/pin_menu.rs` (или
переименовать в `app_menu.rs` в том же коммите): тот же якорь
screen-space (`window.bounds().origin + position`, урок `180fe88`),
`grab: false`, Overlay catcher, `Root`.

Пункты (честное состояние, не рисовать оба Pin и Unpin):

| Пункт | Бэкенд |
|---|---|
| Launch | `launch.rs` как Enter |
| Desktop Actions | `entry.actions` из T265-A; пусто → секции нет |
| Add/Remove favorite | `launcher.toml` T265-C |
| Pin / Unpin dock | уже есть |
| Hide from list | `no_display` пользователя в `launcher.toml` `[hidden]`, не правка `.desktop` на диске |
| Show in file manager | `xdg-open` каталога `.desktop` / `exec` path |
| Properties | диалог или панель с Name/Exec/файл; если диалога нет в ките — честный disabled + причина (планка T246) |
| Launch as other user | **disabled** с причиной, пока нет pkexec/бэкенда. Не рисовать рабочую кнопку |

После Hide строка пропадает из listed, остаётся в скрытых (T265-G).

Образец меню: `tray_menu/`, `dock/context_menu.rs`, текущий `pin_menu.rs`.

## Нельзя

- Второй `PopupMenu` стек / свой `div`-меню.
- Писать в системные `.desktop`.
- Префиксы, сетку с нуля, правую панель.
- `Source/gpui/`, `Cargo.lock`.
- `.unwrap()` на launch/hide.

## Зона

`crates/app/src/launcher/pin_menu.rs` (+ rename ок), `launch.rs` если
нужен action-exec, `launcher_config.rs` ключ `hidden`.
Не `tray_menu/**` кроме чтения как образца.

## Верификация

Юниты: Pin vs Unpin; Hide пишет id в hidden; action id мапится на exec.

Live grim rest/hover меню; Launch; Desktop Action живого приложения
(если есть); favorite toggle; hide → нет в сетке; pin по-прежнему пишет
`dock.toml`. Якорь не мимо catcher.

## Коммит

`feat(launcher): app context menu and desktop actions (T265-D)`
