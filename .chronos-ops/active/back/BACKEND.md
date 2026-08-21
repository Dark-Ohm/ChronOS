# BACKEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** сервисы, протоколы, IPC, скрипты и packaging-конфиги **этого
репозитория**. Не UI (это FRONTEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

## Очередь

1. **T330** — `T330-workspace-active-monitor-refresh.md`. P1.
   Listener: `ActiveMonitorChanged` → `refresh_workspaces`.
2. **T338** — `T338-wallpaper-do-not-stomp-desk.md`. P1. Не стартовать
   пустой `awww-daemon` поверх mpvpaper/waytrogen. Параллелен T330.
