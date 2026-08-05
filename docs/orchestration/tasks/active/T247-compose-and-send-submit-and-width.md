# T247 — compose-and-send не сабмитит + роняет ширину левой панели до 160px

**Приоритет:** P1 — блокирует единственное имеющееся доказательство
«агент слева отвечает» (продуктовое обещание, не косметика).
**Роль:** FRONTEND (Rust, GPUI) + живая проверка IPC.
**Источник:** `docs/orchestration/tasks/report-log/T223-design-audit-report-v4-reshoot.md`
находка #5 (табл. §3); зафиксировано и в `report-log/T233-reshoot-report.md`
§«Снято» (кадр 17) и §«Чего всё ещё нет».

## Контекст

`compose-and-send:<text>` (IPC-команда, введена в T241, коммит из серии
`437bb11`) пишет текст напрямую в `InputState` композера левой панели,
минуя Wayland seat (обходит известный блокер `wtype`/`ydotool`). Живой
прогон 2026-08-05 подтвердил: текст реально доставляется в поле (кадр
`17-left-panel-compose-and-send-dark.png`, репо-путь см. манифест в
`/tmp/t223-captures-2026-08-05/meta/manifest.txt` — evidence pack на
`/tmp`, gitignored, могло уйти на ребут; если недоступен — переснять
заново, это не блокер тикета).

Два отдельных бага, найдены тем же прогоном:

1. **Не сабмитит.** После `compose-and-send:<text>` тред остаётся
   «No messages yet» — похоже, команда только пишет в `InputState`, не
   зовёт `send_composer()` следом (сравнить с тем, что уже описано в
   `docs/orchestration/tasks/done/T241-compose-and-send-ipc.md` —
   исходное задание могло не требовать авто-сабмита, это нормально, но
   тогда нужна ОТДЕЛЬНАЯ IPC-команда `send` — искать, есть ли уже такая
   в IPC-диспетчере, или заводить новую).
2. **Ширина панели просела до 160px** после `compose-and-send` +
   повторных `expand-left`, не восстановилась. Тот же класс бага, что
   T242/T243 (width-desync state vs live geometry), но с новым
   триггером — не resize/select-tab, а запись текста в композер. T243
   (принят, коммит `974ea93`+`99107fe`) уже гейтит резайз-триггеры
   `expand-left`/`select-tab` по `window.bounds()` — вероятно,
   `compose-and-send` идёт мимо этого гейта отдельным путём и не
   триггерит пере-выпуск resize вообще.

## Что нужно

1. Найти обработчик `compose-and-send:` в IPC-диспетчере
   (`crates/app/src/ipc.rs` или аналог — грепнуть
   `"compose-and-send"`), проверить, зовёт ли он `send_composer()`.
   Если сознательно нет (raw-write only, по дизайну T241) — добавить
   рядом отдельную команду `send` (аналог `Enter` в композере), которая
   зовёт уже существующий `send_composer()`.
2. Трейсить width-desync тем же методом, что T243 (живой
   `tracing::debug!` + `systemd-run`, рецепт в
   `skills/chronos-shell/SKILL.md` "live automation from sandbox",
   коммит `17bde3c`) — конкретно: какой путь вызывается при
   `compose-and-send`, гейтится ли он по `window.bounds()` как остальные
   триггеры в `side_panel_left/mod.rs` (T243-фикс, коммит `99107fe`).
3. Живой репро: `compose-and-send:<text>` → замер ширины панели до/после
   через `hyprctl layers -j` — до фикса воспроизвести просадку до 160px,
   после — панель держит докнутую ширину (~352-400).

## Зона файлов

`crates/app/src/side_panel_left/mod.rs` (тот же файл, что T243 уже
трогал — **читать актуальное состояние после T243**, не работать по
памяти старой версии), IPC-диспетчер (найти грепом `compose-and-send`).
Не пересекается с T245/T246/T248-254.

## Верификация

- Живой прогон: `compose-and-send:<text>` → текст в поле → авто-сабмит
  → ответ (или хотя бы исходящее сообщение) появляется в треде, «No
  messages yet» пропадает.
- `hyprctl layers -j` ширина панели после `compose-and-send` = ожидаемая
  докнутая ширина, не 40/160px.
- `cargo build --release -p chronos` чисто, `cargo test --release -p
  chronos --lib -- side_panel_left` зелёные.

## Коммит

`panels+ipc : compose-and-send auto-submit + width-desync fix (T247)`.
