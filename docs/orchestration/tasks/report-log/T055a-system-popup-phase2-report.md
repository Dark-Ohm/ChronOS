<!-- T055a — migrated 2026-07-22 from docs/orchestration/report-log/zed-report-3.md — see docs/orchestration/tasks/MIGRATION.md -->

# Zed №2 → №3 — System popup: Phase 2 (Bug 2 + gaming repaint + display fix)

**Дата:** 2026-07-19 (ночь, продолжение).
**Бриф:** `docs/orchestration/agents/ZED.md` §«Задание №3 Phase 2».
**Вердикт Phase 2:** 🟡 **ЧАСТИЧНО ПРИНЯТО** — Bug 2 (кнопки) + gaming repaint **починены и работают живьём**, но фикс дисплея (Phase 1 → Phase 2 п.1) **не работает**: `window.display(cx)` возвращает `None` для layer-shell окна бара. Это **баг GPUI-форка**, не app-уровня. Жду решения Архитектора.

---

## Что починено (живой смок подтверждён)

### Bug 2 — `background_spawn` без `.detach()` убивает task

**Root cause:** `gpui_scheduler::Task` имеет `#[must_use]` и **drop = cancel immediately** (дока: «If you drop a task it will be cancelled immediately. Calling `Task::detach` allows the task to continue running»). `cx.background_spawn(async move {...})` без `.detach()` и без сохранения в переменную — убивает task сразу после создания. Клик по кнопке доходил до `on_click` (логировался), но async-тело **не запускалось**.

**Фикс:** `.detach()` на каждый `cx.background_spawn` в `system_popup/view.rs` (power profile) и `system_popup/gaming_mode.rs` (apply + revert).

**Почему `battery.rs` и `notifications/view.rs` работали без `.detach()`** — не установлено. Тот же паттерн `cx.background_spawn(...)` без detach. Возможно, `Task` drop в `Context<T>` ведёт себя иначе, чем в `App` (on_click колбэк даёт `&mut App`, не `Context`). Не проверял — не моя зона, факт зафиксирован.

### Bug 2b — `tokio::task::spawn_blocking` вне tokio runtime виснет

**Root cause:** `run_hyprctl_eval` звал `tokio::task::spawn_blocking(|| Command::new("hyprctl")...)`. Но `cx.background_spawn` использует **GPUI background executor**, не tokio. `spawn_blocking` требует tokio runtime в текущем контексте — вне его виснет (не паникует, просто не завершается). `upower.set_power_profile` (zbus) работает, потому что zbus async и спавнит свой runtime; `std::process::Command::status()` через `spawn_blocking` — нет.

**Фикс:** заменил `tokio::task::spawn_blocking` на `std::thread::spawn` + `tokio::sync::oneshot` channel:

```rust
async fn run_hyprctl_eval(payload: &'static str) -> anyhow::Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let result = Command::new("hyprctl").args(["eval", payload]).status();
        let _ = tx.send(result);
    });
    let status = rx.await??;
    if !status.success() { anyhow::bail!("hyprctl eval exited {status}"); }
    Ok(())
}
```

`std::thread::spawn` — реальный OS thread, не требует tokio. `oneshot` channel — async-мост обратно в GPUI executor. `hyprctl eval` отрабатывает за 2-3ms.

### Gaming repaint

**Root cause:** `GamingModeState` — Global, флипается синхронно в `apply`/`revert`, но попап подписан только на brightness + upower сигналы (`init` в `mod.rs:147-188`), не на gaming-глобал. Тоггл физически срабатывал, но knob в UI не сдвигался — визуально мёртвый.

**Фикс:** `GamingModeState::repaint_popup(cx)` — после флипа глобала дёргает `handle.update(...view_cx.notify())` по хендлу из `SystemPopupState`. Вызывается в `apply` и `revert` перед `background_spawn`. Knob сдвигается мгновенно.

### Диагностический лог `on_click`

`tracing::info!` в начале каждого `on_click` (close, brightness ±5%, power segments, gaming toggle) — оставил как постоянный diagnostic (полезен для будущих смоков, не спам — только на клик).

---

## Что НЕ починено (баг GPUI-форка)

### Фикс дисплея — `window.display(cx)` возвращает `None`

**По брифу Архитектора:** попап должен открываться на дисплее ТОГО бара, из которого кликнули. `system.rs:43` зовёт `toggle(window, cx)`, где `window` = окно бара на кликнутом мониторе. `window.display(cx).map(|d| d.id())` должен дать `DisplayId` кликнутого бара.

**Evidence (живой смок 2026-07-19 18:57:27):**
```
system_popup: toggle — window.display()=None, primary_display=None
system_popup: cx.displays() id=DisplayId(4) bounds=Bounds { origin: (0,0), size: 1920×1200 }  # HDMI-A-1
system_popup: cx.displays() id=DisplayId(5) bounds=Bounds { origin: (0,0), size: 2560×1440 }  # DP-1
```

Курсор на (2028, 610) — это DP-1 (x<2560). Пользователь кликнул ⚙ на DP-1. Но `window.display()` = `None`. Fallback `pick_display` → `cx.displays().next()` = `DisplayId(4)` = HDMI-A-1. Попап сел на HDMI-A-1 (4172, 68), а не на DP-1.

**Root cause (в Source/gpui/src/window.rs:2445):**
```rust
pub fn display(&self, cx: &App) -> Option<Rc<dyn PlatformDisplay>> {
    cx.platform
        .displays()
        .into_iter()
        .find(|display| Some(display.id()) == self.display_id)
}
```

`self.display_id` (поле окна бара) не совпадает ни с одним `display.id()` из `cx.platform.displays()`. `self.display_id` обновляется из `self.platform_window.display().map(|display| display.id())` (window.rs:2293) — значит `platform_window.display()` для layer-shell окна возвращает None или id, не совпадающий с `cx.platform.displays()`. Это **баг GPUI-форка** (`../Source/gpui`), не app-уровня.

**Не чинил** — чужая зона (Source/gpui). По брифу: «Bug 1 общий фикс в Phase 1 — не трогай volume/updates/tray/osd/notifications пока я не решу масштаб».

---

## Живой смок — все 5 элементов работают

`pkill -x chronos` → `RUST_LOG=info ./target/release/chronos` → клик ⚙ → клики по всем 5 элементам → верификация.

| Элемент | Клик (лог) | Async (лог) | Внешняя верификация |
|---|---|---|---|
| −5% / +5% | ✅ `brightness ±5% clicked` | ✅ `ddcutil: set brightness to 85/95` | ✅ `ddcutil getvcp 10 --display 2` = 95 |
| Quiet | ✅ `power profile segment clicked: PowerSaver` | ✅ `set power profile to PowerSaver` | ✅ `powerprofilesctl get` = power-saver |
| Balanced | ✅ `power profile segment clicked: Balanced` | ✅ `set power profile to Balanced` | ✅ `powerprofilesctl get` = balanced |
| Performance | ✅ `power profile segment clicked: Performance` | ✅ `set power profile to Performance` | ✅ `powerprofilesctl get` = performance |
| Gaming ON | ✅ `gaming toggle clicked` | ✅ `hyprctl eval ON applied` + `power profile set to Performance` | ✅ `animations: false`, `blur: false`, `allow_tearing: true` |
| Gaming OFF | ✅ `gaming toggle clicked` | ✅ `hyprctl eval OFF applied` + `power profile restored to Balanced` | ✅ `animations: true`, `blur: true`, `allow_tearing: false` (restore) |
| ✕ close | ✅ `✕ close clicked` | — | ✅ `system-popup NOT in layers` (чисто закрылся) |

**Финальное состояние после смока:** `powerprofilesctl: balanced`, `animations: true`, `blur: true`, `allow_tearing: false`, `ddcutil: 95%` — всё корректно восстановлено. Ошибок/panic в логе нет.

---

## Верификация

- `cargo build --release -p chronos` — ✅ ЗЕЛЁНЫЙ (worktree `ChronOS-zed2`, коммит 8457bbc + мой WIP).
- `cargo test --workspace --lib --bins` — ✅ ЗЕЛЁНЫЙ (4+86+25+119+3 = 237 тестов, 0 failed).
- Живой смок — ✅ все 5 элементов + ✕ close + внешняя верификация `ddcutil`/`powerprofilesctl`/`hyprctl getoption`.
- **Попап открывается на HDMI-A-1** (не на DP-1 где кликнули) — `window.display()` = None, fallback на первый в `cx.displays()`. **Не блокирующий** — попап кликабелен, все кнопки работают. Но UX-баг: попап не там, где кликнули.

---

## Файлы (worktree `ChronOS-zed2`)

- `crates/app/src/system_popup/mod.rs` — `toggle(window, cx)` берёт `window.display(cx)`, `open(display_id, cx)` принимает `Option<DisplayId>`, fallback `pick_display`. Диагностический лог **снят** (по брифу).
- `crates/app/src/system_popup/view.rs` — `.detach()` на power profile `background_spawn`, `tracing::info!` в начале каждого `on_click`.
- `crates/app/src/system_popup/gaming_mode.rs` — `.detach()` на apply/revert `background_spawn`, `std::thread::spawn` + oneshot вместо `tokio::task::spawn_blocking`, `repaint_popup(cx)` после флипа глобала, `tracing::info!` в `apply`/`revert`/`run_hyprctl_eval`.

## Что НЕ моё (для Архитектора)

1. **`window.display()` = None для layer-shell окна** — баг GPUI-форка (`../Source/gpui/src/window.rs:2445`). `platform_window.display()` для layer-shell возвращает None или id, не совпадающий с `cx.platform.displays()`. Эскалация в Source (Grok) или app-уровневой workaround (курсор-ориентированный выбор через `hyprctl cursorpos` + `hyprctl monitors -j`).
2. **Остальные 8 попапов** (volume/updates/tray/osd/notifications/launcher/desktop_terminal/dock/history_popup) — тот же латентный баг `pick_display` → fallback на первый дисплей. Раскатка механически отдельным координированным коммитом после решения по п.1.

## Коммит

**НЕ ЗАКОММИЧЕН** — жду решения по `window.display()` = None. Если Архитектор решит «принимать как есть, UX-баг не блокирующий» — коммичу Phase 2 как есть. Если решит эскалацию в Source — жду Grok, потом коммит. Если app-workaround — отдельный заход.

Предлагаемый коммит: `bar : system popup — on_click detach + spawn_blocking fix + gaming repaint + window.display() fix (None fallback pending)`.
