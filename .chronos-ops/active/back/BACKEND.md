---
role: back
status: index
tags: [chronos-ops, back, index]
---

# BACKEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** сервисы, протоколы, IPC, скрипты и packaging-конфиги **этого
репозитория**. Не UI (это FRONTEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

## Очередь

Пусто.

**T330 ЗАКРЫТ 2026-08-21** — `run_listener` слушает `ActiveMonitorChanged`
(`focusedmon`) и зовёт `refresh_workspaces`; хелпер
`focusedmon_active_id_hint` парсит id из `WorkspaceType::Regular`, для
именованных/special/отсутствующих падает в `HWorkspace::get_active()`.
Живой прогон перепроверен архитектором независимо (кадры исполнителя
не показывали бар — не были доказательством): все три позиции точки
(ws2/ws11/ws12, `crop 0,0 240x28`) корректно следуют фокусу монитора
через `focusedmon`, включая точный сценарий бага. `cargo test -p
chronos-services` 273/0 (перепрогнан). Отчёт —
`reports-log/back/T330-workspace-active-monitor-refresh-report.md`.

**T338 ЗАКРЫТ 2026-08-21** — старт больше не спавнит пустой `awww-daemon`
поверх чужого бэкенда (`FOREIGN_BACKEND_BINS`: hyprpaper/swaybg/mpvpaper/
gslapper; awww сознательно не считается чужим — он общий с waytrogen).
`refresh()` на мёртвом awww ставит честный `Degraded` вместо протухшего
состояния. Живой прогон (`hyprctl layers` + лог, оба кейса) и
`cargo test -p chronos-services wallpaper` (11/11) сошлись с кодом. Отчёт —
`reports-log/back/T338-wallpaper-do-not-stomp-desk-report.md`.
