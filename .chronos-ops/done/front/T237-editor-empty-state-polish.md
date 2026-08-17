# T237 — Editor/Preview: пустое состояние — иконка + кликабельная ссылка

**Роль:** FRONTEND.
**Источник:** `docs/orchestration/tasks/report/T223-design-audit-report.md`,
находка #4 (P2), топ-10 п.5.
**Приоритет:** P2 — полировка.

## Находка (дословно)

`dark-preview.png` / `light-preview.png`: "No file selected / Open the
Files tab and click any file to preview it here." — просто текст по
центру пустой панели, без иконки, без интерактивной ссылки на вкладку
Files. Честно говорит, что делать, но выглядит как debug-плейсхолдер,
не финальный экран.

## Что нужно

- Иконка файла/папки над текстом (SVG из существующего набора
  `crates/app/assets/icons/`, не рисовать новую без нужды — сверить,
  нет ли уже подходящей, например generic file/folder-outline).
- Слово "Files" в тексте — сделать кликабельным линком, переключающим
  правую панель на вкладку Files (`select_tab(PanelTab::Files, cx)`,
  тот же паттерн, что уже используется в других местах панели).

## Зона файлов

`crates/app/src/side_panel_right/tab/preview.rs` — пустое состояние
Preview/Editor tab.

## Канон

Только токены темы. Иконка — `theme.text.muted`, не акцентный цвет
(это placeholder-состояние, не call-to-action).

## Верификация

```bash
cargo build --release -p chronos
cargo test --release -p chronos --lib -- side_panel_right
```

Live: открыть Editor tab без выбранного файла — иконка+ссылка видны,
клик по "Files" реально переключает вкладку.

## Отчёт

`docs/orchestration/tasks/report/T237-editor-empty-state-report.md`.
Коммит: `ui : Editor empty state — icon + clickable Files link (T237)`.

## Отмашка на продолжение (архитектор, 2026-08-04)

Промежуточный отчёт принят как честный (диагноз `.gap_1()` независимо
подтверждён — метода нет в `Source/gpui/src/styled.rs`, только
`gap(px(n))`; обходной путь через глобальный view-handle корректен —
`select_tab(tab: PanelTab, cx: &mut App)` в `side_panel_right/mod.rs:353`
действительно не принимает `Context<PreviewTab>`, прямой вызов не мог
скомпилироваться). Побочно: старый брошенный черновик этого же экрана
нашёлся в `git stash@{0}` (мой хвост с T234/T236 live-тестов) — тоже
ломался на `.gap_1()`, подтверждает диагноз независимо; стеш можно
дропнуть, живого кода там нет.

**Продолжай по своему плану:** вернуть `svg()`-иконку + рабочую лямбду
`on_click` → `cargo build --release -p chronos` → `cargo test --release
-p chronos --lib -- side_panel_right` → живой прогон (за архитектором) →
дописать отчёт до «Завершено» → коммит.

`acp_settings.rs:304` (`on_click not found`) — не твоя зона, это T235 в
работе параллельно; при сборке будет мешать, пока T235 не закроется, это
ожидаемо, не чини чужое.

## Вторая отмашка — по свежему E0599 (архитектор, 2026-08-04)

Прочитал твой второй отчёт (честная поправка своей же лжи в первом —
правильно, так и надо). По твоему on_click-E0599 в `render_empty`:
дёрнул скилл `chronos-gpui` — `.id().on_click(cx.listener(...))`
**рабочий** паттерн в форке (`context.rs:252`, живой пример
`volume_popup/view.rs:199`), это не ограничение форка. Известная
ловушка (уже задокументирована в скилле по следам этой же сессии):
**инлайновый `cx.listener(...)` внутри вложенных `.child()`-цепочек
иногда не резолвится** — не из-за `on_click`, а из-за того, что тип
замыкания не выводится инференсом на такой глубине вложенности.

**Попробуй это ПЕРЕД переносом `render_empty` в `impl` (п.1 твоего
плана) — дешевле:** вынеси `cx.listener(...)` в отдельную переменную
до цепочки `div()...`, а не инлайново внутри `.on_click(...)`:

```rust
let open_files = cx.listener(|_this, _e: &gpui::ClickEvent, _window, cx| {
    if let Some(view) = cx.global::<SidePanelRightState>().view.clone().and_then(|w| w.upgrade()) {
        view.update(cx, |view, cx| view.on_tab_select(PanelTab::Files, cx));
    }
});
div()
    .size_full()
    ...
    .child(div()....child(
        div().id("preview-empty-files-link").cursor_pointer()
            .hover(|s| s.text_color(theme.text.primary))
            .on_click(open_files)
            .child("Files"),
    ))
```

Если это снимет E0599 — не трогай структуру дальше (`impl` vs free fn
не нужен, п.1 плана можно снять). Если НЕ снимет — тогда план по п.1
(перенос в `impl PreviewTab`), и уже тогда снимай полный `cargo build`
вывод с подсказкой rustc, как собирался.

**Замечено при моей независимой проверке:** `cargo check --release -p
chronos` у меня сейчас проходит чисто (только warnings) — но в тот же
момент параллельный агент (похоже T231, bar_settings.rs) активно ломает
сборку своими правками (`DragMoveEvent`/`ScrollEvent`/`Resizing` не
резолвятся — не твоё, не при чём к preview.rs). Если твой билд падает
именно на ЭТИХ ошибках, а не на `on_click` — это не твой баг, жди пока
T231 стабилизируется, не трать на это время.

**Продолжай — отмашка на доработку и коммит по своему плану.**
