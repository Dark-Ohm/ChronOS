# T265-F — лаунчер: системные действия в шапке

**Статус:** DONE 2026-08-16 (`ba810d8` + эррата теста). Live grim ещё открыт.
**Приоритет:** P2.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND (`crates/app/src/launcher/**`).
**Канон шапки:** `docs/design/Chronos-OSD-Launcher.dc.html`.

## Задача

Шапка OSD (сейчас заголовок + поиск в `view.rs::render_header`):

- Ряд системных действий: Lock, Logout, Sleep, Hibernate, Reboot, Shutdown.
  Дефолтный набор и порядок — эти шесть. Выкинуть/переставить — ключи
  `launcher.toml` `[system_actions]`, UI порядка — T265-G; в F читать
  ключ если есть, иначе дефолт.
- Двухшаговое подтверждение на Logout/Reboot/Shutdown — как
  `side_panel_right/power_row.rs` (`ArmState`, 3s). Не копипастить
  enum вслепую: вынести общий `PowerAction` + arm, либо звать те же
  хелперы. Lock/Sleep/Hibernate — один клик, если бэкенд жив.
- Аватар + имя: `passwd`/`GECOS` + `~/.face` / AccountsService иконка,
  если файл есть; нет файла — инициал, не битая картинка.

Бэкенды (честно, T246):

| Действие | Как |
|---|---|
| Lock | уже существующий lock шелла / `loginctl lock-session`, что реально в дереве |
| Logout | тот же путь, что `PowerAction::LogOut` |
| Reboot / Shutdown | как `power_row` |
| Sleep / Hibernate | `systemctl suspend` / `hibernate`; нет в системе → плитка `disabled` + причина |

Не писать в Hyprland-конфиг пользователя.

Кнопки — `gpui-component::Button`, не голые `div` с рамкой без `on_click`.

## Нельзя

- Настройки порядка в правой панели (G).
- Менять геометрию Frame / бар (T284).
- Второй ряд power в футере лаунчера «на всякий».
- `Source/gpui/`, `Cargo.lock`.

## Зона

`launcher/view.rs` шапка, `launcher/system_actions.rs`, при необходимости
тонкий общее с `power_row.rs` (вынести в `crates/app/src/power.rs`, не
наоборот тащить лаунчер в правую панель).

## Верификация

Юниты: дефолтный порядок; мусор в toml → дефолт + warn; arm/confirm
как у `power_row`.

Live grim шапки; Lock запирает; недоступный Hibernate — disabled, не
пустышка. Reboot не жать вслепую на рабочей сессии — достаточно
Confirm? на кадре и лог команды в dry-run, если есть; иначе сказать в
отчёте, что reboot/shutdown смокнуты только до arm.

## Коммит

`feat(launcher): system action header (T265-F)`
