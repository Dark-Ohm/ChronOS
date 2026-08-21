---
ticket: T338
role: back
status: done
tags: [chronos-ops, back, done]
---

# T338 — не сносить стол пользователя при старте

**Роль:** BACKEND. **P1.** Живая находка T328 B1.
**Зона:** `crates/services/src/wallpaper/mod.rs` (`ensure_daemon` ~209–248
и старт подписчика). При необходимости `crates/app/src/wallpaper_ctl.rs`
ветка `open_waytrogen_gallery` (сейчас синхронный spawn).
**Не трогать:** UI Display-карточки (T339), theme, frame.

Параллелен T330 (другие файлы).

## Зачем

ChronOS на каждом старте поднимает `awww-daemon --no-cache` и **не ставит
обои**. Стол падает в дефолт Hyprland. Комментарий T244 уже знает войну
с waytrogen (кэш → чёрный слой); `--no-cache` не отменяет захват
background-слоя пустым демоном. Обратно: кнопка Open waytrogen + mpvpaper
убивает `awww`; `wallpaper-refresh` → Connection refused, UI молчит.

Улики: `dump/qa-ux/T328/log/chronos.log` `starting awww-daemon` × старты;
кадры 18/20/24/35/36. Стол владельца — `~/Pictures/Wallpapers/*.mp4` через
waytrogen/mpvpaper.

## Что сделать

1. Если background уже занят чужим бэкендом (`mpvpaper`, `gslapper`,
   живой `waytrogen` wallpaper) — **не** спавнить `awww-daemon`.
2. Не убивать чужой wallpaper-процесс стартом шелла.
3. Если awww всё же наш и жив — не оставлять `color: 000000` без
   явного действия пользователя (не авто-restore кэша, это T244).
4. `open_waytrogen_gallery`: не блокировать UI; после закрытия —
   refresh карточки **или** честный «daemon dead».

Не писать свой video wallpaper. Не `awww restore`.

## Готово когда

- Рестарт ChronOS при живом mpvpaper: mpvpaper жив, awww не стартует
  (лог без `starting awww-daemon`), стол не Hyprland-сплэш.
- Чистый сеанс без чужого демона: awww как сейчас, без регрессии T244
  (чёрный слой поверх).
- Живой grim + `hyprctl layers` namespace до/после в отчёт.

**Отчёт:** `.chronos-ops/reports-fresh/T338-wallpaper-do-not-stomp-desk-report.md`
