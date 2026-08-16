# T265-F — Launcher system-action header — Report

**Date:** 2026-08-16
**Role:** FRONTEND + тонкий сервис-бэкенд. Zone: `launcher/**` + вынос в
`crates/app/src/power.rs` + `services/power` (Lock/Sleep/Hibernate).
**Commit:** `ba810d8` `feat(launcher): system action header (T265-F)`.

**Приёмка (архитектор, 2026-08-16):** код принят с однострочной эрратой.
`folder_serializes_and_reloads` не заполнил `system_actions` — `cargo test
--lib` не собирался (check без test-модуля это скрыл). В изолированном
worktree после эрраты: `--lib` 557/557, `chronos-services power` 10/10.
Live grim — долг.

## Status

**Done (code). Services power тесты зелёные. Полный `--lib`/`--release`
заблокирован чужим WIP (Notifications-tab) — см. «Про release».**

## Что сделано

### `crates/app/src/power.rs` — вынос общего `PowerAction` + arm

Как просила спека («вынести общий `PowerAction` + arm»), arm/confirm-машина
вынесена из `power_row.rs` в `crates/app/src/power.rs`:
`PowerAction { Lock, LogOut, Sleep, Hibernate, Restart, Shutdown }` (6
вариантов, `needs_confirm()` = LogOut/Restart/Shutdown), `ArmState`,
`ARM_TIMEOUT=3s`, `on_click` / `is_confirming_click` / `on_timeout`. Правую
панель не тронул поведенчески: `power_row.rs` теперь UI-only, импортирует из
`crate::power`, его футер по-прежнему рендерит только Switch/LogOut/Restart/
Shutdown (Lock/Sleep/Hibernate — недостижимые arm в `label_for` + `warn` в
`on_power_click`).

### `services/power` — бэкенды Lock/Sleep/Hibernate

В `PowerSubscriber` добавлены `lock()` (`loginctl lock-session`),
`suspend()` (`systemctl suspend`), `hibernate()` (`systemctl hibernate`) —
тем же паттерном `spawn_command`, что `log_out/restart/shutdown`, с
command-level юнитами. Lock в дереве отсутствовал (нет hyprlock/swaylock),
поэтому взят именованный в спеке `loginctl lock-session`.

### `launcher/system_actions.rs` — новый модуль

- `DEFAULT_ACTIONS` (спека-порядок: Lock/LogOut/Sleep/Hibernate/Restart/
  Shutdown), `parse_action` (id + алиасы `reboot`/`suspend`/`poweroff`/…),
  `resolve_actions` (`[system_actions] order` → список; мусор → warn+skip,
  всё-мусор → дефолт+warn).
- Доступность: `hibernate_available()`/`suspend_available()` читают
  `/sys/power/state` (`disk`/`mem`); `available()` + `disabled_reason()`.
- Аватар/имя: `user_name` ($USER/$LOGNAME), `user_full_name` (GECOS из
  `/etc/passwd`), `face_path` (`~/.face` → AccountsService icon → None),
  `user_initial` (первая буква, иначе `?`).

### `launcher/view.rs` — шапка

- Шапка = две строки: прежний title-row (sigil/title/mode/`invoke SUPER
  SPACE`) + новая system-row: аватар (фото или инициал-круг) + имя + шесть
  тайлов.
- Тайлы — **`gpui-component::Button`** (не голые `div`): Ghost для обычных,
  Danger для Shutdown/armed, `disabled` + tooltip-причина для недоступного
  Sleep/Hibernate, `Confirm?` на armed, `.compact().with_size(XSmall)`.
- `on_system_action_click`: one-click (Lock/Sleep/Hibernate) — сразу в
  `AppState::power`; confirming — arm → confirm (тот же стейт, что правая
  панель) + `ARM_TIMEOUT` disarms.
- `[system_actions]` hot-reload через существующую `launcher_config::subscribe()`.

### `launcher_config.rs`

`SystemActionsConfig { order }` + поле `system_actions` + ключ в RMW-цикле
`write_config` (unknown top-level ключи сохраняются, как раньше).

## Verification

| Command | Result |
|---|---|
| `cargo test -p chronos-services power` | **10 passed; 0 failed** (было 3 → +lock/suspend/hibernate + существующие) |
| `cargo check -p chronos` | **мои файлы чисты (0 errors/warnings)**; lib в целом — 8 ошибок **чужого WIP** |

Юниты спеки на месте (в `power.rs` / `system_actions.rs`, pure):
- дефолтный порядок = спека-шестёрка;
- мусор в `[system_actions]` → дефолт (+warn), mixed → valid kept / junk skipped;
- arm/confirm как у `power_row` (тесты переехали в `power.rs`, поведение 1:1).

**Про release/lib.** Полный `cargo test -p chronos --lib` и `--release`
сейчас не компилируются из-за **параллельной сессии (Notifications-tab)**:
8 ошибок в `side_panel_right/tab/{mod,notifications}.rs`, `tabs.rs`,
`notifications/history_list.rs`, `side_panel_right/view.rs:636`
(`TabContent::Notifications` / `PanelTab::Notifications` не покрыты, нет
`subscribe`/`overflow_y_scrollbar`). Это не моя зона, файлы не трогал; мои
файлы в выводе `cargo check` — ноль строк. Прогнать `--lib`/`--release` после
того, как сосед доведёт Notifications (или принимать в ворктри).

## Что НЕ сделано (Архитектор / дальше)

1. **Live grim** (спека): шапка с аватаром/именем; Lock запирает;
   недоступный Hibernate — disabled+причина; reboot/shutdown — только до
   `Confirm?` на кадре (не жать на рабочей сессии). Требует живого шелла.
   На этой машине hibernate **доступен** (`disk` в `/sys/power/state`,
   `[platform]` в `/sys/power/disk`), поэтому disabled-ветку Hibernate
   живьём не увидим — она для машин без swap/hibernate.
2. **Перенос в `done/` + статус родителя T265** — самоприём не делаю.

## Отчёт одной строкой (выборы из спеки)

- `PowerAction`+arm — **вынесены в `crates/app/src/power.rs`** (не копия enum).
- Кнопки — **`gpui-component::Button`** (Ghost/Danger/disabled+tooltip).
- Lock — **`loginctl lock-session`** (в дереве своего lock-бэкенда не было).
- Порядок — `[system_actions] order = [...]`, мусор → дефолт+warn.

## Коммит

```
feat(launcher): system action header (T265-F)
```

(10 files: `power.rs`/`system_actions.rs` новые, `launcher/{view,mod,
launcher_config}.rs`, `services/power/mod.rs`, `side_panel_right/{power_row,
view}.rs`, `lib.rs`, `main.rs`. `Cargo.lock`, `Source/gpui/`,
Hyprland-конфиг — не тронуты.)
