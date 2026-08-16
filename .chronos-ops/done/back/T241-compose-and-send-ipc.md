# T241 — IPC `compose-and-send:<text>` — обойти Wayland seat-фокус для капч/тестов

**Роль:** BACKEND, короткая задача (механизм уже почти весь есть).
**Источник:** `docs/orchestration/tasks/report/T233-reshoot-report.md` —
живой блокер, пойманный при пересъёмке T223 2026-08-04.
**Приоритет:** P1 — без этого T223 физически не может доказать ядро
продукта (агент слева отвечает), и любая будущая live-капча композера
упрётся в ту же стену.

## Проблема (доказано живьём)

`wtype` печатает в **Wayland seat keyboard focus** компоситора, а не в
окно, на которое GPUI навёл `window.focus()` внутри своего дерева
элементов. Проверено: `chronos-ipc expand-left` докает левую панель и
вызывает `window.focus(&this.composer_focus, cx)`
(`side_panel_left/mod.rs`, `expand_with_composer`) — GPUI-уровень фокуса
меняется, но реальный seat-фокус компоситора остаётся на том окне, что
было активно до этого (в тесте — на kitty-терминале). Текст, посланный
`wtype`, ушёл мимо ChronOS полностью.

`ydotool` (клики мышью) отдельно и давно нерабочий (kernel-module-
mismatch, см. память `cachyos-kernel-modules-mismatch`).

Итог: **нет рабочего способа отправить сообщение в левую панель
программно** — ни клавиатурой, ни мышью. Это блокирует не только T223
(нужен кадр "тред с ответом агента"), но и любой будущий live-тест
композера/agent-flow.

## Решение

IPC-команда `compose-and-send:<text>`, которая пишет прямо в
`composer_input.content` и вызывает уже существующий
`send_composer()` — **минуя Wayland seat целиком**, тем же классом
приёма, что `preview-target` уже использует для редактора (пишет
в global напрямую из App-контекста, не через симулированный ввод).

## Что уже есть (не изобретать заново)

- `crates/app/src/side_panel_left/composer.rs:918` —
  `pub(crate) fn send_composer(&mut self, _window: &mut Window, cx:
  &mut Context<Self>)` — уже существует, приватная для крейта. Просто
  публично не вызывается извне панели.
- `composer_input` — свой `TextInputState`-подобный тип (НЕ
  gpui-component `InputState`), с полем `.content` (`String`) и методом
  `.insert_char(&text)` (`composer.rs:200`). Прямая запись в `.content`
  дешевле посимвольного `insert_char` в цикле — сверить, не сломает ли
  прямая перезапись `.content` какой-то побочный инвариант
  (`selected_range`/`cursor_visible` состояние) — если сломает, тогда
  через `insert_char` в цикле, не напрямую.
- Паттерн IPC-плюмбинга — три новых команды (`expand-left`,
  `select-tab`, `preview-target`) уже добавлены и работают
  (`ipc/messages.rs`, `ipc/service.rs`, `ipc/mod.rs` — сверить diff
  T226-infrastructure, коммит уже в дереве). Повторить тот же паттерн:
  парсинг префикса в `messages.rs`, канал в `service.rs`, приём с
  дебаунсом в `mod.rs`.

## Что сделать

1. `crates/app/src/ipc/messages.rs` — payload `compose-and-send:<text>`
   (текст может содержать пробелы/спецсимволы — распарсить как
   "всё после первого `:`", по образцу `preview-target:<path>`).
2. `crates/app/src/ipc/service.rs` + `mod.rs` — канал + приём, тот же
   паттерн.
3. `crates/app/src/side_panel_left/mod.rs` — новая публичная функция
   (например `compose_and_send(text: String, cx: &mut App)`):
   - Гарантировать что панель открыта и докнута (`expand_with_composer`
     логика, реюз — не дублировать).
   - Записать `text` в `composer_input.content` (или `insert_char` в
     цикле, если прямая запись небезопасна — см. выше).
   - Вызвать `send_composer(window, cx)` — но `send_composer` требует
     `&mut Window`, а IPC-хендлер обычно не имеет `Window` в скоупе
     (см. комментарий у `pub fn toggle(cx: &mut App)` — "IPC handler has
     no Window in scope there"). Нужно достать `Window` через
     `handle.update(cx, |this, window, cx| ...)` на трекнутом хендле
     панели, тем же паттерном, что `expand_with_composer` уже делает.

## Канон

- Не логировать текст сообщения в `tracing::info!` на уровне выше
  `debug!` (это потенциально пользовательский ввод, тот же класс
  осторожности, что с паролем в T232-отклонённом плане, хоть и не
  секрет — просто гигиена).
- Только для dev/tooling использования (капчи, тесты) — не документировать
  как продуктовую фичу пользователя, это debug/QA-инструмент, аналог
  `CHRONOS_SMOKE_SIDE_PANEL`.

## Верификация

```bash
cargo build --release -p chronos
cargo test --release -p chronos --lib -- side_panel_left
```

Live:
```bash
chronos-ipc expand-left
chronos-ipc compose-and-send:"hello, what can you help with?"
```
— проверить `chronos.log` на реальный ACP prompt turn (не просто "IPC
received"), и `grim` треда — сообщение пользователя видно в чате, ждать
ответ агента, снять кадр с ответом.

## Отчёт

`docs/orchestration/tasks/report/T241-compose-and-send-ipc-report.md`.
Коммит: `services+ui : compose-and-send IPC for programmatic composer testing (T241)`.
