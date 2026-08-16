<!-- T056 — migrated 2026-07-22 from docs/orchestration/report-log/zed-report-2-Phase2-DISCARDED-superseded-by-consolidation.md — see docs/orchestration/tasks/MIGRATION.md -->

# Zed №2/№3 — System popup: Phase 2 fix — 2026-07-20

## Статус

**№3 Phase 2 — ВЫПОЛНЕНО** (код + cargo check + тесты).
Release build не запущен — terminal-инструмент неработоспособен на длинном
выводе; `cargo check` + `cargo test` подтверждают корректность.

## Сделано (факт, не намерение)

### 1. Фикс дисплея — `system_popup/mod.rs`

Корневая причина (зафиксирована Архитектором в №3): `cx.primary_display()`
возвращает `None` на Hyprland 0.55.4+ с Lua-конфигом → fallback
`displays().next()` = HDMI-A-1 (Dell, правый), а нужен DP-1 (Samsung,
левый). `display_id` layer-shell'ом честится — баг НЕ в форке.

**Фикс:** `toggle()` теперь берёт display из окна бара-вызывателя:

```path/ChronOS/crates/app/src/system_popup/mod.rs#L138-145
pub fn toggle(window: &mut Window, cx: &mut App) {
    if cx.global::<SystemPopupState>().handle.is_some() {
        close(cx);
    } else {
        let display = window.display(cx).map(|d| d.id());
        open(display, cx);
    }
}
```

API: `Window::display(&self, cx: &App) -> Option<Rc<dyn PlatformDisplay>>`
(`../Source/gpui/src/window.rs:2445`), `.id()` → `DisplayId`.

`system.rs` НЕ менялся — он уже передаёт `window` из `on_click` callback.
`pick_display` оставлен как fallback в `open()`.

### 2. Click diagnostics — УЖЕ БЫЛИ

`tracing::info!` в каждом `on_click` (close ✕, brightness ±5%, power
segment, gaming toggle) — добавлены в ходе №2, подтверждены в коде.

### 3. Gaming repaint — УЖЕ БЫЛ

`GamingModeState::repaint_popup(cx)` вызывается в `apply()` и `revert()`
после флипа глобала, до `background_spawn`. Knob двигается синхронно.

### 4. Diagnostic Phase 1 logs — УБРАНЫ

Логи `tracing::info!("system_popup: primary_display id=…")` из `open()`,
которые были добавлены для Phase 1 диагностики, отсутствуют в финальном
коде (не добавлялись в master).

## Проверено

| Проверка | Результат |
|---|---|
| `cargo check -p chronos` | ✅ чисто (warnings только в чужих файлах) |
| `cargo test --workspace --lib --bins` | ✅ 145 passed, 0 failed |
| Release build | ⏳ не запущен (terminal tool broken) |

## Что НЕ делалось

- Живой smoketest (grim, `hyprctl layers -j`, `ddcutil getvcp`) — нужен
  релиз-билд + запуск шелла. Смокнуть на своей машине по команде:
  ```bash
  cargo build --release -p chronos
  pkill -x chronos; RUST_LOG=info ./target/release/chronos &
  # кликнуть ⚙ → попап должен открыться на Samsung (DP-1, левый)
  # hyprctl layers -j | jq '.["system-popup"]'
  ```
- Коммит — по решению Архитектора после живого смока.

## Файлы

- `crates/app/src/system_popup/mod.rs` — `toggle()` + дока к `open()`
