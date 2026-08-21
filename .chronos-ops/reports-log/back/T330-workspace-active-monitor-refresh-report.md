# T330 — отчёт BACKEND: подсветка воркспейса слушает focusedmon

**Роль:** BACKEND. **Дата:** 2026-08-21.
**Зона изменена:** только `crates/services/src/compositor/hyprland.rs` (+70/−1).
`bar/widgets/workspaces.rs`, UI, IPC — НЕ трогал.

## Что сделал

Один файл, три правки.

1. **Хендлер `ActiveMonitorChanged` в `run_listener`.** Рядом с
   `workspace_changed` добавлен `add_active_monitor_changed_handler`, который
   зовёт `refresh_workspaces`. Комментарий в коде фиксирует корень T330:
   `focusedmon` приходит, когда фокус уезжает на монитор, чей воркспейс уже
   активен — `workspace`-события в этот момент Hyprland не шлёт, поэтому бар
   не перечитывался и синяя точка оставалась на старом мониторе.

2. **Чистый хелпер `focusedmon_active_id_hint(evt) -> Option<i32>`.**
   `MonitorEventData.workspace_name` — это `WorkspaceType` (имя), а не id.
   Для числового воркспейса (`Regular("2")`) парсим id и передаём как hint
   (тот же приём «доверяем событию, а не опросу», что и `workspace_changed`);
   именованный/`special`/отсутствующий — возвращаем `None`, и
   `refresh_workspaces` падает в `HWorkspace::get_active()` (как уже делают
   `workspace_added`/`workspace_deleted`).

3. **Три юнита без сокета:** `focusedmon_hint_parses_numeric_workspace`,
   `focusedmon_hint_falls_back_for_named_special_and_missing`,
   `active_monitor_handler_registers_without_socket` (последний — ровно
   «хендлер регистрируется»: `EventListener::new()` + регистрация не трогают
   сокет, его открывает только `start_listener`).

## Как проверил

### Тесты и сборка (своим прогоном)

```
cargo test -p chronos-services
  → 273 passed; 0 failed; 1 ignored   (было 270, +3 новых)
cargo build --release -p chronos
  → Finished release в 3m52s
```

mtime-сверка: `hyprland.rs` = 1787343713, `target/release/chronos` =
1787344056, старт живого процесса = 1787344150 — процесс запущен ПОСЛЕ
сборки, кадры сняты с текущего бинаря, не с чужого артефакта.

### Живой smoke (focusedmon → синяя точка)

Реальный сеанс, два монитора. Воркспейсы в баре слева направо: **2, 11, 12**
(точки на x≈120/132/144 пикселей DP-1). До теста фокус на HDMI-A-1 (ws 12),
на DP-1 уже активен ws 2 — ровно сценарий «воркспейс уже активен на другом
мониторе».

Шаги и результат (`dump/qa-ux/T330/state.txt` + фреймы):

```
BEFORE : focused=HDMI-A-1 ws=12   (синяя точка на x=144 → ws 12)
ACTION : /dispatch hl.dsp.focus({ workspace = 2 })   ← тот же FocusWorkspace, что шлёт клик по точке
AFTER  : focused=DP-1 ws=2        (hyprctl activeworkspace = ws 2)
```

Пиксельная проверка точки в баре (grim до/после, левая секция DP-1):

```
before: акцентная точка rgb(3,108,181) на x=144  (ws 12)
after : акцентная точка rgb(3,108,181) на x=120  (ws 2)
неактивные точки rgb(67,69,88) на остальных позициях
```

То есть после `focusedmon` `hyprctl activeworkspace` (ws 2) и синяя точка
бара (ws 2) показывают **один** id — приёмка «Готово когда» выполнена.
Лог без panic/error в `chronos_services::compositor`, listener жив.

## Чего НЕ делал

- `WorkspaceMoved` / `WorkspaceRenamed` — в тикет не тащил (как и предписано);
  в событийном списке крейта они есть, слушателей на них по-прежнему нет.
- `bar/widgets/workspaces.rs`, UI, IPC — не трогал.
- Не коммитил, тикет из `active/` не двигал (приёмка за архитектором).
