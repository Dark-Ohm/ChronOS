# T230 — Live re-smoke T210/T211 + IPC `select-tab`

**Роль:** FRONTEND (может быть тот же минион, что вёл T210/T211).
**Источник:** T223 design-audit report, отклонён приёмкой архитектора
2026-08-04 — `docs/orchestration/tasks/rejected/T223-design-audit-report.md`
(причина отклонения — в шапке того файла, прочитать перед стартом).
**Канон:** `docs/HANDOFF.md` 2026-08-04 чекпоинт (ydotool erratic весь
вечер — читай раздел ниже про обходной путь), `docs/ARCHITECT.md`
(живой прогон ≠ "compiles and tests are green" для UX-кода).

## Контекст

T210 (drag/peek hover-strip) и T211 (theme-toggle crash + Follow SVG)
были приняты по коду+юнит-тестам, но **живой grim/видео-прогон после
патча ни разу не сделан**. T223-аудитор (текстовая сессия без vision)
это перепутал с "баг всё ещё живой" и процитировал старый T209-евиденс
как текущий — приёмка это отловила и отклонила отчёт. Но сам факт
остаётся: три вещи нужно реально увидеть на живом шелле, не только в
коде.

Отдельно: T223 наткнулся на дыру в IPC — `toggle-side-panel-right`
умеет только open/close, а раскрыть конкретную вкладку контента
(`Files`, `Acp`, `Settings`, ...) программно нельзя, только кликом по
rail-иконке. Это блокирует любую будущую автоматизацию скриншотов
(CI, r/unixporn пресс-кит, следующий design-audit). Решаем в этом же
тикете, зона файлов не пересекается с re-smoke частью.

## Задача A — Live re-smoke (доказать, не чинить)

Цель — либо подтвердить, что T210/T211 держат в реальности, либо
найти живой регресс и вернуть в `active/` отдельным тикетом с
конкретным репро.

1. **Theme toggle (T211 P0 #3 из T209).** Открыть System settings
   правой панели, кликнуть Theme toggle. Ожидание: тема меняется
   живьём (dark↔light), шелл не падает. Заснять `grim` до/после ИЛИ
   `wf-recorder` клип с моментом клика.
2. **Follow-иконка visual state (T211 P0 #2 из T209).** Включить/
   выключить Follow в левой панели (тред-хедер). Заснять ДВА кадра —
   ON и OFF — и прогнать `magick compare` между ними. Ожидание:
   заметный diff (SVG перекрашивается через `theme.accent.primary` /
   `theme.text.muted`), не 0 px как было в T209.
3. **Hover-strip drag/peek (T210 P0 #1 из T209).** Навести курсор на
   hover-strip у правого края, дождаться peek-открытия, начать drag
   ручки ресайза, прервать drag НЕ отпуская кнопку мыши за пределами
   strip'а (репро из T209). Ожидание: панель не схлопывается
   намертво — hover-strip продолжает открывать panel после.
   Если vospроизвести drag руками нельзя (см. ydotool ниже) — задокументируй
   как `NOT CAPTURED` с точной причиной, не выдумывай прохождение.

**Обходной путь по ydotool:** HANDOFF 2026-08-04 фиксирует ydotool
erratic весь вечер (клики не доходят до GPUI layer-shell окон,
подозрение на `cachyos-kernel-modules-mismatch`, см. Claude-память).
Проверь `ydotool` живым тестом перед тем как полагаться на него
(простой клик по любой кнопке бара). Если всё ещё сломан — сценарии 1
и 2 можно закрыть через IPC (`toggle-theme` уже есть; Follow нужно
проверить, есть ли у него IPC-хук, если нет — клик мышкой руками
самим пользователем, попроси). Сценарий 3 (drag) в принципе требует
реального курсора — если ydotool мёртв, это законный `NOT CAPTURED`,
не блокер тикета.

## Задача B — IPC `select-tab:<alias>`

**Зона:** `crates/app/src/ipc/messages.rs`, `crates/app/src/ipc/service.rs`,
`crates/app/src/ipc/mod.rs`, `crates/app/src/side_panel_right/mod.rs`.

Добавить IPC-команду, которая открывает правую панель на полный
контент (`ensure_content_width` + `open_pinned`, см.
`side_panel_right/mod.rs:362-363` — тот же путь, что уже использует
`CHRONOS_SMOKE_SIDE_PANEL` env-переменная) и переключает на конкретную
вкладку. Alias'ы — по существующим `PanelTab` вариантам
(`System`, `Library`, `Captures`, `Acp`, `Settings`, `HyprlandBinds`,
`Preview` — свериться с `for_mode`/`PanelTab` enum, не выдумывать
имена).

Формат payload по образцу существующих: `select-tab:<alias>`
(разбор — как `classify_set_workspace_mode`/`classify_wallpaper` в
`ipc/messages.rs`, не голая строка-константа как у toggle-команд).
Канал доставки — свой `mpsc` sender/receiver пара по образцу
`side_panel_right_toggle_sender`, диспатч в `ipc/mod.rs` loop.

Не трогай `toggle-side-panel-right` — это отдельная, уже рабочая
команда (open/close pinned rail), `select-tab` её не заменяет и не
дублирует.

## Верификация

- `cargo test -p app` (или весь workspace) зелёный.
- Release-сборка (`cargo build --release`), не dev.
- Живой смок: `chronos-ipc select-tab:acp` (или выбранный alias)
  реально открывает панель с нужной вкладкой — grim-кадр приложить.
- Задача A — приложить фактический grim/видео-материал по каждому из
  трёх сценариев, либо честный `NOT CAPTURED` с причиной (не путать с
  "не пробовал").

## Отчёт

`docs/orchestration/tasks/report/T230-t210-t211-live-resmoke-and-select-tab-ipc-report.md`.
Коммит: `ui : T230 live re-smoke + select-tab IPC`.
