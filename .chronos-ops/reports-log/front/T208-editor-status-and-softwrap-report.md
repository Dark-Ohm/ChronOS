# T208 report

**Зона:** `crates/app/src/side_panel_right/tab/preview.rs` only.  
`bar/`, `view.rs`, `surfaces.rs`, `tab/mod.rs` — не тронуты.

**Предшественники:** T205 (`8b36055` — themed buffer + gutter + code_editor),  
T194c (Preview/Edit dual-mode toggle). **Спека:** phase C из `docs/superpowers/specs/2026-08-02-editor-themed-notepad-gutter.md` §6C.

## Цель

1. Статус-строка `Ln X, Col Y` в Edit mode — читает позицию каретки из `InputState::cursor_position()` (0-based `Position { line, character }`), показывает 1-based.
2. Soft-wrap toggle в chrome bar — кнопка «Wrap», переключает `InputState::set_soft_wrap()` на editor entity.
3. Не syntax/LSP (explicit non-goal).

## Changes

### `crates/app/src/side_panel_right/tab/preview.rs`

**1. `soft_wrap: bool` поле в `PreviewTab` (default `true`):**

```rust
/// Soft-wrap toggle (T208). Default true — long lines wrap at the
/// panel width instead of requiring horizontal scroll. Flips
/// `InputState::set_soft_wrap` on the editor entity when toggled.
soft_wrap: bool,
```

**2. Soft-wrap toggle в `render_chrome_bar`:**

Кнопка «Wrap» между Preview|Edit и Terminal ▾ в правой части chrome bar.
Активное состояние (`soft_wrap == true`) → `interactive.hover` фон, выключено → `border.subtle` + muted текст.

```rust
let sw_listener = cx.listener(|this, _e, _w: &mut Window, cx| {
    this.soft_wrap = !this.soft_wrap;
    if let Some(editor) = &this.editor {
        editor.update(cx, |input, cx| {
            input.set_soft_wrap(this.soft_wrap, _w, cx);
        });
    }
    cx.notify();
});
```

Chrome bar layout: `Preview|Edit · Wrap · Terminal ▾`.

**3. Создание editor с `.soft_wrap(self.soft_wrap)`:**

При первом создании editor в `render()` — builder chain дополнен `.soft_wrap(self.soft_wrap)`:

```rust
let editor = cx.new(|cx| {
    InputState::new(window, cx)
        .code_editor("plaintext")
        .soft_wrap(self.soft_wrap)  // ← T208 fix: no state drift
});
```

Без этого `InputState` default = `false`, а `self.soft_wrap` = `true` — кнопка «Wrap» показывала активное состояние при выключенном переносе. Исправлено по code-review.

**4. Статус-строка «Ln X, Col Y» в `render_editor_input_body`:**

В хедере редактора, между path/dirty колонкой и кнопкой Save:

```rust
let cursor_pos = self
    .editor
    .as_ref()
    .map(|editor| editor.read(cx).cursor_position())
    .map(|pos| format!("Ln {}, Col {}", pos.line + 1, pos.character + 1));
```

Рендерится через `.when_some(cursor_pos, |el, pos| { ... })` — если editor ещё не создан, статус-строка скрыта (Empty/Loading/View mode).

Формат: моноширинный шрифт, `text.muted`, 10.5px, справа от Save.

## Verification

```
$ cargo check -p chronos
0 errors

$ cargo test -p chronos --lib
test result: ok. 219 passed; 0 failed
```

## Edge cases

| случай | поведение |
|---|---|
| Editor не создан (View mode, Empty, Loading) | `cursor_pos = None` → статус-строка скрыта |
| Каретка в начале файла | `Ln 1, Col 1` |
| Многострочный файл, каретка на строке 42 | `Ln 42, Col X` |
| Soft-wrap toggle при editor = None | `self.soft_wrap` флипается, `set_soft_wrap` не вызывается (no-op) |
| Создание editor (первый вход в Edit) | `.soft_wrap(self.soft_wrap)` — начальное состояние синхронизировано |
| Переключение файлов в Edit | `soft_wrap` на InputState сохраняется (entity переживает `set_value`) |
| Soft-wrap off → long line | горизонтальный скролл (built-in в code_editor режиме) |

## Что НЕ сделано

- **Живой смок** — LIVE NOT VERIFIED. Статика зелёная: check + 219 тестов.
  Требуется release-бинарь + ручной тест: открыть .md в Editor, клавиши-стрелки →
  Ln/Col обновляется, toggle Wrap → длинная строка переносится.
- **Highlight current line** — deferred (T205 spec: «optional, skip without blocking»).
- **Выбор по строке (click gutter → select line)** — не в scope T208.
- **Синтаксис / LSP** — explicit non-goal (PRODUCT: «not an IDE»).

## Acceptance

- [x] Status line `Ln X, Col Y` в Edit mode (1-based, читает `InputState::cursor_position`)
- [x] Soft-wrap toggle в chrome bar (Wrap кнопка, `set_soft_wrap`)
- [x] Editor создаётся с корректным soft_wrap (нет дрейфа toggle↔input)
- [x] Preview|Edit guard, Terminal drawer, Save/dirty — без изменений
- [x] View mode, image/markdown/text — без изменений
- [x] Check чистая, 219 тестов зелёные
- [x] Зона: только `preview.rs`
- [ ] LIVE smoke — не проверено

---

## Приёмка

**Коммит:** `editor : status line + soft wrap (T208)`.

**Вердикт:** ACCEPTED (статика). Живой смок — отдельным прогоном release-бинаря.
