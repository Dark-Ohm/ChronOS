**Принято архитектором 2026-08-15.** Live `+` владельца. Коммит `17afee6` (только view.rs+mod.rs).

# T289 — Right dock must not lock tabs

**Дата:** 2026-08-15
**Статус:** готово к приёмке

## Что сделано

Dock (⊞/⊟) перестал раскрывать вкладку и блокировать её. Контракт изменён:
dock = **exclusive-zone flag**, а не «контент всегда открыт».

### Изменения в `crates/app/src/side_panel_right/view.rs`

1. **`toggle_dock`** — убран вызов `ensure_content_width(target)`. Теперь флипает
   `dock_content`, сбрасывает `last_exclusive_zone = None` (принудительный пересчёт
   зоны в `rail_view::Render`), и `refresh_windows()`. Ширина не трогается.

2. **`on_tab_select`** — `content_open` больше не включает `dock_content`:
   `state.dock_content || state.width > RAIL_ONLY_WIDTH + 1.0` → `state.width > RAIL_ONLY_WIDTH + 1.0`.

3. **Branch 1 удалён** — блок `if dock_content { return; }` (no-op для same-tab под dock)
   удалён. Same-tab клик теперь всегда проходит через Branch 2 (collapse) / Branch 3 (re-open),
   независимо от `dock_content`. `dock_content` не меняется — остаётся `true`.

4. **`render()`** — `content_open` перешёл на `visible_w > 1.0` (без `dock_content ||`).
   Убрана переменная `dock_content` из `render()`, которая стала неиспользуемой.

5. **`apply_active_tab_width`** — `content_open` перешёл на `state.width > RAIL_ONLY_WIDTH + 1.0`
   (без `dock_content ||`). Mode-fallback больше не раскрывает контент при collapsed+locked.

### Изменения в `crates/app/src/side_panel_right/mod.rs`

6. **Doc comment `dock_content`** — переписан: теперь «exclusive-zone flag», а не
   «content is always visible».

### Тесты

7. **Удалён** `on_tab_select_active_tab_while_docked_is_noop` (T221 no-op test).
   В отчёте называется: этот тест фиксировал старый контракт T221 «same tab docked = no-op».

8. **Переписан** `toggle_dock_flips_flag_and_applies_active_tab_width` →
   `toggle_dock_flips_flag_without_changing_width`: проверяет 2 кейса из брифа
   (collapsed+dock-toggle и open+dock-toggle).

9. **Добавлен** `on_tab_select_same_tab_while_docked_collapses_then_reopens`: проверяет
   2 кейса из брифа (dock ON + same-tab → collapse, dock ON + same-tab again → reopen).

10. **Обновлены** `mode_fallback_applies_system_preferred_width` и
    `mode_fallback_applies_fixed_system_width`: добавлено `state.width = 480.0` в setup,
    т.к. dock ON больше не делает `content_open = true` при collapsed width.

### Что НЕ делал

- `cargo build --release -p chronos` — не запускался (5+ минут, см. T168/T173 паттерн).
- Живой grim-кадр — не снят. Тесты покрывают 4 кейса брифа, но live-приёмка требует
  `hyprctl layers` для верификации exclusive zone = 40 при dock ON + collapsed.
- `Source/gpui/`, `Cargo.lock`, левая панель, `tabs.rs`, `dock` module, peek/pin hover-strip — не тронуты.
- Branch 4 (different-tab under dock = switch, width pinned) — не тронут.

## Что проверено

```
cargo test -p chronos --lib side_panel_right   # → 195 passed; 0 failed
cargo test -p chronos --lib                    # → 475 passed; 0 failed
cargo check -p chronos --lib                   # → Finished, only pre-existing warnings
```

## Краткая связь с T221

T221 установил контракт: rail icon = единственный жест раскрытия; same-tab под dock = no-op.
T289 частично отменяет this: same-tab под dock теперь collapse/re-open (как и без dock),
а dock перестаёт раскрывать контент при включении. `exclusive_px()` уже правильно
вычислял: dock ON + collapsed → 40; dock ON + open → width.
