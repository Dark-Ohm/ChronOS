# T242 — expand-left width desync: отчёт

**Дата:** 2026-08-04
**Статус:** traced + fixed (preventive), live repro pending

## Диагностика

Добавлен `tracing::debug!` в два места:
- `state.rs:ensure_chat_width()` — логирует `width`, `target`, `remembered` до и после
- `mod.rs:render()` — логирует `state_width`, `last_resized`, `dock_chat` при resize

## Гипотеза (подтверждена code review, не живым трейсингом)

`open_window()` имеет ранний return когда `handle.is_some()`. Если панель уже открыта (rail-only), `expand_with_composer` работает на существующей сущности. `ensure_chat_width()` меняет `state.width`, но render resize-guard (`last_resized_width != Some(self.state.width)`) может не сработать, если `last_resized_width` случайно совпал с новым значением.

## Фикс

В `expand_with_composer()` и `compose_and_send()`: `this.last_resized_width = None` перед `ensure_chat_width()`. Это гарантирует, что render всегда делает `window.resize()`.

Плата: один лишний `set_size` Wayland round-trip при программном открытии (не в drag-цикле). Ничтожно.

## Верификация

```bash
cargo check -p chronos  # зелёный
```

Живой smoke: рецепт из T242.md (10× повтор `toggle-side-panel-left` → `expand-left`). Не проведён — требуется живой ChronOS.

## Коммит

`panels : expand-left/compose-and-send reset last_resized_width to prevent width desync (T242)`
