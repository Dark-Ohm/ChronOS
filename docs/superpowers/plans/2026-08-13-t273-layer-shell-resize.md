# T273 Layer-Shell Resize Synchronization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use `executing-plans` to implement this plan inline. Steps use checkbox syntax for tracking.

**Goal:** Устранить ghost-trail, пустые кадры и рывок правого рейла, синхронизировав размер renderer buffer с подтверждённым compositor `configure` для layer-shell поверхностей.

**Architecture:** `PlatformWindow::resize` продолжает немедленно отправлять желаемый `zwlr_layer_surface_v1.set_size`, но не меняет локальный renderer buffer до ответного `configure`. Обработчик `handle_layersurface_event` уже проводит размер через общий `handle_xdg_surface_event`, где `WaylandWindowStatePtr::resize` обновляет bounds, viewport и renderer. XDG-toplevel и popup пути не меняются.

**Tech Stack:** Rust 2024, gpui-ce ChronOS fork, Wayland `wlr-layer-shell-unstable-v1`, WGPU, Hyprland 0.56.1+.

## Global Constraints

- Не менять абсолютную resize-модель T216 и клампы T210/T214 в ChronOS.
- Не маскировать fork-level рассинхрон подгонкой layout правой панели.
- Не менять XDG-toplevel и popup resize lifecycle.
- Проверять UX только release-бинарём; статический скриншот не доказывает временной дефект.
- Снимать два видео: плавное сужение и быстрые рывки в обе стороны.
- Не трогать чужие изменения в `Cargo.lock`, T266 и остальных грязных зонах.

---

### Task 1: Зафиксировать resize policy тестом

**Files:**
- Modify: `../Source/gpui_linux/src/linux/wayland/window.rs`

**Interfaces:**
- Consumes: `WaylandSurfaceState::{Xdg, LayerShell, Popup}`.
- Produces: приватная policy-функция/тип, различающие configure-driven layer-shell, deferred XDG и reposition-driven popup.

- [ ] Добавить минимальный unit test: layer-shell не запускает локальный buffer resize из `PlatformWindow::resize`; XDG сохраняет deferred resize.
- [ ] Временно вернуть старую layer-shell policy и запустить точечный тест; ожидается FAIL по layer-shell ожиданию.
- [ ] Восстановить configure-driven policy и повторить тест; ожидается PASS.
- [ ] Запустить весь `gpui_linux` test target.

### Task 2: Проверить протокольный и сборочный контур

**Files:**
- Modify only if test exposes a defect: `../Source/gpui_linux/src/linux/wayland/window.rs`

**Interfaces:**
- Consumes: `handle_layersurface_event` → `handle_xdg_surface_event` → `WaylandWindowStatePtr::resize`.
- Produces: один подтверждённый путь, в котором `configure` обновляет bounds и renderer до следующего commit нового буфера.

- [ ] Запустить `cargo test -p gpui_linux` в `../Source`.
- [ ] Запустить `cargo check -p gpui_linux --features wayland` или эквивалентный workspace target из manifest.
- [ ] Собрать `cargo build --release -p chronos` в ChronOS.

### Task 3: Живая проверка T273

**Files:**
- Existing temporary instrumentation: `crates/app/src/side_panel_right/view.rs`
- Evidence: `~/Pictures/t273/`.

**Interfaces:**
- Consumes: `state.width`, `window.bounds().size.width`, frame counter, resize direction.
- Produces: видео и trace, показывающие отсутствие пустых кадров/обоев/смещения рейла.

- [ ] Убедиться, что параллельная сессия не владеет shell; затем остановить только `pkill -x chronos`/project CLI.
- [ ] Запустить release ChronOS с T273 trace.
- [ ] Снять `wf-recorder` плавного сужения; остановить `kill -INT`.
- [ ] Снять `wf-recorder` быстрых рывков в обе стороны; остановить `kill -INT`.
- [ ] Перепроверить `select-tab:hyprland_binds` на докнутой ширине и длинный drag до обоих клампов.

### Task 4: Очистка и завершение

**Files:**
- Modify: `crates/app/src/side_panel_right/view.rs`
- Modify: `docs/orchestration/tasks/active/T273-rail-wobble-during-shrink-resize.md`
- Modify: `docs/HANDOFF.md` only if the accepted outcome changes item #8-bis state.

**Interfaces:**
- Consumes: live trace/video verdict.
- Produces: минимальный production diff без временного frame logging и без неподтверждённой layout-маскировки.

- [ ] Удалить `t273_frame` и временные T273 debug-lines.
- [ ] Удалить `resizing` из `content_open`, если live evidence не доказывает самостоятельную необходимость.
- [ ] Повторить `cargo test -p chronos side_panel_right --lib --bins`.
- [ ] Повторить release build и короткий live smoke после очистки.
- [ ] Обновить тикет фактическими командами, числами и путями видео.
- [ ] Коммитить по репозиториям раздельно и только поимённым `git add`: fork fix в `Source`, task/report cleanup в ChronOS.
