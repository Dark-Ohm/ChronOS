---
ticket: T348
role: recon
status: active
tags: [chronos-ops, recon, active]
---

# T348 — RECON: control-surface пяти wallpaper-движков (источник — waytrogen)

**Роль:** RECON. **P1.** Родитель T338/T339, канон —
`.chronos-ops/checkpoint/ARCHITECTURE.md` §19 (2026-08-22): владелец
подтвердил, что ChronOS поддерживает **hyprpaper, swaybg, mpvpaper, awww,
gslapper** как равноправные движки, и что **цель — отказаться от
waytrogen как внешнего приложения**: заменить кнопку «Open waytrogen» в
Display-вкладке собственным компонентом (галерея + переключатель
движков), который сам умеет во все пять.
**Питает:** T349 (BACKEND, диспетчер) — не стартовать T349, пока этот
отчёт не принят. T349, в свою очередь, питает будущий FRONTEND-тикет на
собственную галерею (заведётся после T349).
**Зона:** только чтение. Живые эксперименты в `/tmp` разрешены (спавн/kill
тестового процесса движка), правки в дереве — нет.

## Зачем

`crates/services/src/wallpaper/mod.rs` умеет управлять только awww
(`ensure_daemon`/`apply_command`/`query_current` — все жёстко на
`AWWW_BIN`/`awww query`). `Backend` enum (`types.rs:13`) перечисляет все
пять, но для четырёх остальных нет ни одной команды — только строка для
`pidof` (T338 `FOREIGN_BACKEND_BINS`). `wallpaper_ctl.rs` дополнительно
целиком завязан на внешний `waytrogen` бинарь (`WAYTROGEN_BIN`,
`open_waytrogen_gallery`, CTA «yay -S waytrogen» в `display.rs`) — это
и есть зависимость, которую нужно снять.

## Главный источник — уже в дереве, не гадать по `--help`

**`reference/waytrogen-main/`** (gitignored, НЕ коммитить — см.
`.gitignore`) — полный исходник waytrogen, **лицензия Unlicense
(public domain)**, LICENSE-файл в корне репо подтверждён. В отличие от
`reference/gpui-shell-main` (без лицензии, только rewrite-по-паттерну,
0 скопированных строк) — waytrogen можно читать и **напрямую портировать
паттерном** с атрибуцией в `Source/NOTICE` (уже есть прецедент: awww CLI
в `types.rs` doc-комментарии сослан на этот же источник).

Первичные файлы для этого тикета:
- `reference/waytrogen-main/src/changers/{awww,hyprpaper,swaybg,mpvpaper,gslapper}.rs`
  — готовые command builder'ы на все пять движков. Это и есть ответ на
  вопрос «какой Set/Query у каждого» — читать код, не изобретать.
- `reference/waytrogen-main/src/wallpaper_changers.rs` — общий трейт/enum,
  диспетчинг между `changers/*`.
- `reference/waytrogen-main/src/database.rs`, `fs.rs` — как waytrogen
  сканирует папку и кэширует превью (нужно для будущей своей галереи,
  см. «Задел на будущее» ниже).
- `reference/waytrogen-main/src/monitors.rs` — per-monitor модель.

`man`/`--help`/апстрим README — только как fallback, если код waytrogen
по какому-то движку неполон или расходится с тем, что реально стоит на
машине (тогда так и пометить в отчёте: «waytrogen делает X, но
`<bin> --help` на этой машине показывает Y»).

## Что сделать

Для **каждого** из hyprpaper, swaybg, mpvpaper, gslapper (awww уже
реализован, для сверки можно упомянуть кратко) — прочитать
соответствующий `changers/<bin>.rs` и задокументировать в отчёте:

1. **Set (сменить обои).** Точная команда/аргументы из кода
   waytrogen. Restart-based (kill + новый спавн, как сейчас awww в
   `ensure_daemon`/`spawn_daemon`) или socket/IPC-based (живой процесс
   принимает команду без рестарта, напр. `hyprctl hyprpaper wallpaper`).
2. **Query.** Спрашивает ли waytrogen движок о текущем состоянии, или
   просто хранит последнее применённое сам (тогда `WallpaperState` тоже
   должен просто помнить, не спрашивать).
3. **Per-monitor vs global.**
4. **Video vs image.** Что из четырёх реально поддерживает видео (ожидание:
   только mpvpaper — проверить по коду, не полагаться на ожидание).
5. **Multi-instance / restart safety.** Как waytrogen убивает предыдущий
   инстанс движка перед новым спавном (если убивает) — важно для
   диспетчера T349, чтобы не плодить зомби-процессы.
6. **Отличие от T338 `FOREIGN_BACKEND_BINS`.** Тот список — только имена
   для `pidof`; здесь нужен полный CLI.

## Задел на будущее (не делать сейчас, только зафиксировать в отчёте)

Отдельным коротким разделом отчёта — как waytrogen сканирует папку с
обоями и кэширует превью (`database.rs`/`fs.rs`, thumbnail-кэш если
есть). Это не для T349 (диспетчер), а для будущего FRONTEND-тикета
«своя галерея вместо кнопки Open waytrogen» — не проектировать его
здесь, просто оставить ссылки на файлы и как оно устроено в двух-трёх
предложениях, чтобы архитектор не читал weytrogen с нуля второй раз.

## Готово когда

Отчёт содержит для каждого из 4 движков: Set-команду (restart/IPC) с
цитатой/путём на `changers/<bin>.rs`, Query-способ, per-monitor да/нет,
video-support да/нет, multi-spawn поведение — каждый факт со ссылкой на
источник (путь:строка в waytrogen, или `--help`-вывод если код разошёлся
с этой машиной). Плюс двух-трёхпредложенческая справка про
scan/thumbnail-механизм waytrogen (задел). Не смок, не живой прогон —
чтение кода, но факты с путями, не домыслы.

**Отчёт:** `.chronos-ops/reports-fresh/T348-wallpaper-backend-control-surfaces-report.md`
