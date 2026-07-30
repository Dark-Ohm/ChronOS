# T108 Task3 — Фикс кликабельности dropdown агентов

**Дата:** 2026-07-24
**Статус:** ЗАКРЫТО

## Что сделано

### 1. Фикс: dropdown агентов не кликался

**Проблема:** `panel.rs` строил dropdown в два прохода — сначала визуальные пункты списка (lines 89-144), потом навешивал отдельные абсолютные `div`-оверлеи с `on_click` (lines 320-331). Оверлеи перехватывали все клики, делая оригинальные пункты некликабельными — выбор агента из меню не работал.

**Фикс:** `on_click` встроен прямо в каждый пункт dropdown через `cx.listener(move |this, _, _, cx| { this.switch_agent(&agent_id, cx); })`. Абсолютные оверлеи удалены.

### 2. Фикс: порядок построения элементов (E0502 borrow-checker)

**Проблема:** `render_composer()` и `chat.render()` захватывают `cx` через RPIT на весь срок жизни возвращаемого элемента (Rust 2024 impl Trait capture rules). Dropdown тоже нуждался в `cx.listener()`. Раньше chat/composer строились ПЕРЕД dropdown — `cx` был заимствован mutable на весь остаток `render_panel`, и `cx.listener()` в dropdown конфликтовал (E0502).

**Фикс:** Dropdown теперь строится ПЕРЕД chat/composer — `cx.listener()` вызывается до того, как `cx` захватывается RPIT-элементами.

## Верификация

- `cargo test -p chronos --lib` — **26 passed, 0 failed**
- `cargo check` для `panel.rs` — без ошибок
- Единственная ошибка сборки — `updates.rs:109` (pre-existing, не связана с T108)

## Что не в this scope

- Jank дропдауна (#7) и ghost-trail (#8) — отдельные задачи
- Ghost-trail на уровне форка (#8-bis) — отдельная задача (gpui fork, `PlatformWindow::resize`)
- Live round-trip с текстовым вводом — требует ydotool/sim, не воспроизведено в этой сессии
- Список агентов: только Hermes реально ACP-совместим. Cline/OpenCode/др. не проверены — не включать без live handshake

## Затронутые файлы

- `crates/app/src/side_panel_left/panel.rs` —重构 dropdown construction, merge click handlers, reorder build sequence
