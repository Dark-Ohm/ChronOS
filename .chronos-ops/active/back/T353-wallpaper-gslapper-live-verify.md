---
ticket: T353
role: back
status: active
tags: [chronos-ops, back, active]
---

# T353 — BACKEND: gslapper — живой Set, самый рискованный из четырёх

**Роль:** BACKEND. **P2.** Дочерний T349 (принят 2026-08-22, канон —
`.chronos-ops/checkpoint/ARCHITECTURE.md` §19). Отчёт T349 сам назвал
gslapper «самым рискованным»: IPC-сокет-протокол
(`crates/services/src/wallpaper/backends.rs` `gslapper_change`/
`gslapper_ipc`/`gslapper_spawn`/`gslapper_stop`) портирован из
waytrogen-источника (`reference/waytrogen-main/src/changers/gslapper.rs`,
см. цитаты в
`.chronos-ops/reports-log/recon/T348-wallpaper-backend-control-surfaces-report.md`),
юнит-протестирован только на уровне argv/socket-path, живым процессом не
подтверждён. `gslapper` установлен на этой машине (`/usr/bin/gslapper`,
проверено архитектором 2026-08-22) — блокера на отсутствие бинаря нет.
**Зона:** только живая проверка + фикс багов — код в `backends.rs` уже
написан в T349, не переписывать с нуля, если протокол не совпадёт —
чинить точечно, не выбрасывать существующий код.
**Не параллелить с T351/T352** — один файл (`backends.rs`), делать
последовательно.

## Зачем

Единственный движок с настоящим двусторонним IPC (unix-сокет
`$XDG_RUNTIME_DIR/chronos/gslapper-<fnv1a(monitor)>.sock`, команды
`change <path>`/`stop`/`query`, ответы `STATUS: ...`/`ERROR: ...`).
Риск конкретный: `gslapper_spawn` в ChronOS поллит IPC `query` до
`STARTUP_TIMEOUT` (3с) с интервалом 25мс, ожидая, что сокет откликнется
сразу после спавна — если реальный gslapper держит паузу дольше или
формат ответа на `query` отличается от того, что зашито в
`gslapper_ipc`'s парсинге (`ERROR:`-префикс, one-line response), Set
зависнет молча (timeout → `anyhow::bail!`, но без диагностики что именно
не так).

## Что сделать

1. Живой Set на «All» (`*`) — первый вызов должен пойти по ветке спавна
   (`gslapper_change`: сокета ещё нет → `gslapper_spawn`), не по IPC
   `change`.
2. Второй Set (другой файл, тот же монитор) — должен пойти по ветке
   живого IPC `change <path>` (сокет уже существует).
3. Смена типа медиа (картинка → видео или наоборот на том же мониторе)
   — проверить ветку `Err(e) if e.to_string().contains("cannot update
   path")` → restart (`stop` + новый спавн). Если реальный текст ошибки
   gslapper отличается от `"cannot update path"` — это баг, чинить
   сравнение (например на `contains` подстроки из реального вывода).
4. `kill_backend_fn(Backend::Gslapper, ...)` — живой `stop` (не `pkill`)
   останавливает процесс, сокет-файл убирается.
5. Доказательства: `pidof gslapper`, `grim` кадр (видео/картинка реально
   на экране), лог IPC-обмена (добавить `debug!`/`trace!` вокруг
   `gslapper_ipc`, если сейчас недостаточно видимости для диагностики).
6. `cargo test -p chronos-services wallpaper` зелёный.

## Готово когда

Все три сценария (спавн / живой change / restart на смене типа медиа)
подтверждены живьём с кадрами и логом; `kill_backend_fn` для gslapper
подтверждён (сокет пропадает, процесс мёртв); тесты зелёные. Если
протокол реального gslapper разошёлся с портом из waytrogen — расхождение
зафиксировано и почитано архитектором до фикса, не тихо переписано.

**Отчёт:** `.chronos-ops/reports-fresh/T353-wallpaper-gslapper-live-verify-report.md`
