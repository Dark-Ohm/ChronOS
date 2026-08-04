# T237 — Editor/Preview: пустое состояние — иконка + кликабельная ссылка

## Статус: **КОД ГОТОВ, ВЕРИФИКАЦИЯ ЧАСТИЧНАЯ (in progress)**

Собственный код T237 написан и **проходит type-check** (E0599 по
`on_click` снята). Полную сборку crate и юнит-тесты прогнать НЕ могу —
сборка заблокирована чужой параллельной задачей T231
(`DragMoveEvent`/`ScrollEvent`/`Resizing` в `bar_settings.rs`), не
относящейся к T237. Архитектор подтвердил: это не моё, ждать
стабилизации T231, время не тратить. Коммит сделан исключительно по
своей работе (см. ниже).

## Что сделано (финально, на диске)

### render_empty (crates/app/src/side_panel_right/tab/preview.rs)
- Стала `render_empty(&Theme, &mut Window, &mut Context<PreviewTab>)`;
  вызов в `render()` обновлён:
  `State::Empty => render_empty(&theme, window, cx)`.
- Иконка `folder.svg` над текстом:
  `svg().path("icons/folder.svg").size(px(40.)).text_color(theme.text.muted)`.
- Текст разбит на три `.child(...)`: `"Open the "`, кликабельный
  `div()` со словом `"Files"`, `" tab and click any file to preview it here."`.
- Слово «Files» — реальный линк:
  `id("preview-empty-files-link") + cursor_pointer() + text_color(muted)
  + hover(primary) + on_click(open_files)`. Клик переключает правую
  панель на вкладку Files тем же путём, что rail/IPC — через глобальный
  view-handle:
  ```rust
  let open_files = cx.listener(|_this, _e: &gpui::ClickEvent, _window, cx| {
      if let Some(view) = cx
          .global::<SidePanelRightState>()
          .view
          .clone()
          .and_then(|w| w.upgrade())
      {
          view.update(cx, |view, cx| view.on_tab_select(PanelTab::Files, cx));
      }
  });
  // ... div().id("preview-empty-files-link").on_click(open_files).child("Files")
  ```

### Импорты (preview.rs)
Добавлены: `svg`, `InteractiveElement`, `PanelTab`, `SidePanelRightState`.
(`SidePanelRightView` добавлялся, но оказался unused — убран, warnings
нет.)

### Побочные фиксы
- `.gap_1()` → `.gap(px(4.))`/`.gap(px(10.))` (метода `gap_1` НЕТ в форке
  gpui, `Source/gpui/src/styled.rs`; подтверждено независимо
  архитектором).
- **Ключевой фикс E0599:** инлайновый `cx.listener(...)` внутри глубокой
  `.child()`-цепочки не резолвился (тип замыкания не выводился инферсом
  на глубине вложенности). Решение (из скилла `chronos-gpui`,
  подтверждено архитектором): вынести `cx.listener(...)` в отдельную
  переменную `open_files` ДО цепочки `div()` и передать его в
  `.on_click(open_files)`. После этого `on_click` резолвится, E0599
  уходит. Паттерн `.id().on_click(cx.listener(...))` в форке рабочий
  (живые примеры: `context.rs:252`, `volume_popup/view.rs:199`).

## Верификация

- `cargo build --release -p chronos` — **НЕ собирается целиком**, но
  исключительно из-за чужих ошибок T231:
  `error[E0404] expected trait, found struct DragMoveEvent`,
  `cannot find type ScrollEvent`, `cannot find value Resizing` —
  все в `bar_settings.rs` (параллельный агент T231).
- Мой файл `preview.rs` в выводе сборки **отсутствует** (ни ошибок, ни
  warnings) — значит мой код type-check проходит, E0599 снята.
- Юнит-тесты `cargo test --release -p chronos --lib -- side_panel_right`
  прогнать нельзя: crate не собирается из-за T231. Будет доступно после
  стабилизации T231.

## Live-проверка (за архитектором)
Агент без GUI сам не верифицирует. Требуемый smoke: открыть Editor tab
без выбранного файла → видна иконка папки + текст со ссылкой «Files»
(muted); клик по «Files» реально переключает правую панель на вкладку
Files.

## Коммит
`ui : Editor empty state — icon + clickable Files link (T237)`
(только правки `crates/app/src/side_panel_right/tab/preview.rs`; чужой
код T231/T235 не тронут).

---

**Ticket Status**: Code complete & type-checks; blocked on full build/test
by parallel T231 (not mine). Live UI smoke — architect's call.
