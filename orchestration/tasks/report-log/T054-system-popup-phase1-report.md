<!-- T054 — migrated 2026-07-22 from orchestration/report-log/zed-report-2.md — see orchestration/tasks/MIGRATION.md -->

# Zed №2 → №3 — System popup: Phase 1 (диагностика дисплея)

**Дата:** 2026-07-19 (ночь).
**Бриф:** `orchestration/agents/ZED.md` §«Задание №3 Phase 1».
**Вердикт Phase 1:** 🟡 **СТОП, жду Архитектора** — root cause найден, это **баг GPUI-форка**, не app-уровня. App-фикс возможен, но это общий рефактор 9 попапов — зона Архитектора.

---

## Phase 1 — что я установил (evidence, не гадания)

### Методика

В `system_popup::open` (worktree `ChronOS-zed2`, коммит 8457bbc + мой WIP) добавил диагностический лог ПЕРЕД `open_window`:

```rust
tracing::info!("system_popup: primary_display id={:?}", cx.primary_display().map(|d| d.id()));
for d in cx.displays() {
    tracing::info!("system_popup: display id={:?} bounds={:?}", d.id(), d.bounds());
}
// ... pick_display ...
tracing::info!("system_popup: pick_display returned id={display_id:?}");
// ... open_window ...
tracing::info!("system_popup: opened on display_id={display_id:?}");
```

Пересобрал (`cargo build --release -p chronos`), запустил (`RUST_LOG=info ./target/release/chronos`), кликнул ⚙ на **DP-1** (левый, Samsung, primary), снял лог + `hyprctl layers -j`.

### Живой вывод (2026-07-19 17:32:39)

```
system_popup: primary_display id=None
system_popup: display id=DisplayId(4) bounds=Bounds { origin: Point { x: 0px, y: 0px }, size: Size { 1920px × 1200px } }
system_popup: display id=DisplayId(5) bounds=Bounds { origin: Point { x: 0px, y: 0px }, size: Size { 2560px × 1440px } }
system_popup: pick_display returned id=Some(DisplayId(4))
system_popup: opened on display_id=Some(DisplayId(4))
```

`hyprctl layers -j` после клика:
```
HDMI-A-1 L3: system-popup  300x284 at (4172,68)  pid=1113806
```

### Расшифровка

**1. `cx.primary_display()` возвращает `None`.** Это корневая аномалия. На машине с двумя живыми дисплеями (DP-1 Samsung primary 144Hz + HDMI-A-1 Dell вторичный) GPUI-форк **не знает, какой primary**. Поэтому `pick_display` падает в fallback:

```rust
fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}
```

`primary_display() == None` → `.or_else(...)` → `cx.displays().into_iter().next()` = **первый в списке**.

**2. `cx.displays()` возвращает дисплеи в порядке:**
- `DisplayId(4)` — `1920×1200` = **HDMI-A-1** (Dell U2412M, вторичный, правый)
- `DisplayId(5)` — `2560×1440` = **DP-1** (Samsung LC32G5xT, primary, левый, 144Hz)

`.next()` берёт `DisplayId(4)` = HDMI-A-1. Отсюда попап сел на HDMI-A-1, хотя пользователь кликнул ⚙ на баре DP-1.

**3. `display_id` ДЕЙСТВИТЕЛЬНО учитывается layer-shell'ом.** Попап сел именно на `DisplayId(4)` (HDMI-A-1), координаты `(4172, 68)` = 2560 (HDMI-A-1 origin в глобальных) + 1920 - 300 - 8 (margin) ≈ 4172. Если бы `display_id` игнорировался — попап садился бы всегда на один и тот же монитор независимо от переданного id. Этого **не происходит**: передали `DisplayId(4)` → сел на HDMI-A-1. Значит layer-shell output binding в GPUI-форке работает, проблема — в **выборе** `display_id`, не в backend.

**4. Bounds у обоих дисплеев = `origin (0,0)`.** GPUI отдаёт bounds в **локальных** координатах дисплея, не глобальных. Поэтому по bounds нельзя отличить «левый» от «правого» — только по размеру. DP-1 = 2560×1440, HDMI-A-1 = 1920×1200.

### Вывод

**`cx.primary_display() == None` — баг GPUI-форка** (`../Source/gpui`), не app-уровня. На Hyprland 0.55.4+ с Lua-конфигом GPUI не получает primary display от компоузитора. Все 9 попапов с этим `pick_display` паттерном (`volume_popup`, `updates_popup`, `tray_menu`, `notifications`, `osd`, `launcher`, `desktop_terminal`, `dock/context_menu`, `notifications/history_popup`, `system_popup`) садятся на **первый в `cx.displays()`**, а не на primary. Просто раньше никто не проверял физический монитор — сверяли namespace в `hyprctl layers` и бордер, а не «на каком экране».

---

## Варианты фикса (для решения Архитектора)

### A. Эскалация в Source (Grok) — починить `primary_display()` в gpui-форке

**Плюсы:** разовый фикс, все 9 попапов автоматически начинают работать правильно.
**Минусы:** чужая зона (`../Source/gpui`), отдельный агент (Grok), время. Может оказаться, что Hyprland layer-shell protocol вообще не передаёт primary display — тогда фикс невозможен на уровне protocol.

### B. App-уровневой общий helper `popup_display(cx)` — координировать Архитектору

Заменить `pick_display` во всех 9 попапах на общий helper, который выбирает дисплей по эвристике:

```rust
// crates/app/src/popup_display.rs (новый общий модуль)
pub fn popup_display(cx: &App) -> Option<DisplayId> {
    // 1. primary_display() — если GPUI починят, работает само
    if let Some(d) = cx.primary_display() { return Some(d.id()); }
    // 2. Самый большой дисплей = primary (эвристика для desktop)
    cx.displays().into_iter()
        .max_by_key(|d| d.bounds().size.width.0 as i32 + d.bounds().size.height.0 as i32)
        .map(|d| d.id())
}
```

**Плюсы:** app-уровень, не трогает Source, работает на текущем gpui-форке.
**Минусы:** общий рефактор 9 попапов — shared-file зона поперёк других агентов (Hermes/Cline/Mimo/Autohand). Архитектор в брифе явно сказал: «НЕ изобретай общий рефактор 9 попапов сам — координирую я».

### C. Локальный фикс в `system_popup::pick_display` — только мой попап

Заменить `pick_display` в `system_popup/mod.rs` на эвристику «самый большой дисплей» (или «дисплей с курсором»). Не трогать остальные 8 попапов.

**Плюсы:** минимальная зона, только мой файл.
**Минусы:** латентный баг остаётся в 8 других попапах. Архитектор в брифе сказал: «Bug 1 общий фикс в Phase 1 — не трогай volume/updates/tray/osd/notifications пока я не решу масштаб».

### D. Курсор-ориентированный выбор — попап открывается там, где курсор

В `on_click` ⚙ передать в `toggle` позицию курсора, выбрать дисплей по координатам. Самый UX-правильный вариант (попап всегда там, где кликнули), но требует API «найти дисплей по глобальной координате» — нужно проверить, есть ли в GPUI-форке.

---

## Рекомендация

**Вариант A (Source-фикс `primary_display()`) — предпочтительный**, если Hyprland layer-shell protocol вообще передаёт primary. Если нет — **B (общий helper)** как app-уровневой workaround. **C/D** — если Архитектор решит не трогать чужие попапы сейчас и оставить латентный баг.

Жду решения. Phase 2 (Bug 2 + gaming repaint) — **после** того, как попап начнёт открываться на правильном мониторе, иначе тестировать клики по кнопкам на попапе, всплывшем не там — бессмысленно (Архитектор в брифе).

---

## Файлы

- `crates/app/src/system_popup/mod.rs` — диагностический лог в `open()` (worktree `ChronOS-zed2`).
- `/tmp/chronos-zed2-phase1.log` — живой лог смока.

## Worktree

```
/home/neo/projects/chronos-ecosystem/ChronOS-zed2  HEAD=8457bbc (detached)
```

Диагностический лог в `system_popup/mod.rs` — оставил. По решению Архитектора: оставить как постоянный diagnostic, упростить, или откатить.
