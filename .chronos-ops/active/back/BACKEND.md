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

1. **T352** — `T352-wallpaper-swaybg-live-verify.md`. P2. Живой Set,
   проверить потерю картинки на другом мониторе при повторном Set
   (нет per-monitor реестра в отличие от waytrogen).
2. **T353** — `T353-wallpaper-gslapper-live-verify.md`. P2. Самый
   рискованный — IPC-протокол (спавн/change/restart-на-смене-медиа),
   живьём не подтверждён.

Делать **последовательно**, не параллельно — оба в одном файле
(`backends.rs`).

**T351 ЗАКРЫТ 2026-08-22** — `ensure_hyprpaper_daemon()` (lazy bootstrap:
pidof → `systemctl --user start hyprpaper` → голый спавн → поллинг →
200мс settle), решение архитектора (вариант 1) реализовано дословно.
Попутно найден API-дрейф Hyprland 0.56.2: у hyprpaper из IPC осталась
ТОЛЬКО `wallpaper mon,path,fit` (preload/listloaded/listactive мертвы) —
T349-argv уже был на актуальной поверхности, повезло. Архитектор
перепроверил на ЗАНОВО собранном release-бинаре (чужой был на 32 мин
старше правки) — живой Set подтверждён и на одном мониторе, и на «All»
(оба монитора, все открытые поля/углы — красный `srgb(253,0,0)`).
`cargo test -p chronos-services wallpaper` 28/0/1-ignored. Отчёт —
`reports-log/back/T351-wallpaper-hyprpaper-live-verify-report.md`.

**T349 ЗАКРЫТ 2026-08-22** — диспетчер: `resolve_backend()` (config
`~/.config/chronos/wallpaper.toml` → autodetect pidof mpvpaper→gslapper→
swaybg→hyprpaper→awww → default awww), `backends.rs` — argv-билдеры
+ apply для всех 4 неawww движков (точные команды из T348), `kill_all_except`
(gslapper — IPC `stop`, остальные `pkill -9`), `wallpaper_ctl::next()`
теперь ротирует видео на video-бэкендах (mpvpaper/gslapper), не только
сообщает «skipped». Живой Set подтверждён на mpvpaper (два монитора,
видео реально рисуется, не чёрный кадр); hyprpaper/swaybg/gslapper —
только argv-юниты, живьём не гонялись (честно раскрыто, бриф требовал
только «хотя бы один из четырёх»). `cargo test -p chronos-services`
289/0, `cargo test -p chronos wallpaper_ctl` 6+6 — перепрогнано
архитектором. Отчёт — `reports-log/back/T349-wallpaper-multi-backend-dispatch-report.md`.
Живая проверка остальных трёх движков — см. T351/T352/T353.

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
