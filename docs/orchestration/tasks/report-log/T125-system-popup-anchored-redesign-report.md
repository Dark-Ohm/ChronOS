# T125 — System Popup Anchored Redesign

**Status:** ✅ Done  
**Date:** 2026-07-25  
**Branch:** `T125-system-popup-redesign` (commits: on main, non-breaking)

## Что сделано

### 1. `system_popup/mod.rs` — AnchoredPopup skeleton

Старого LayerShell-попапа больше нет. Теперь — `AnchoredPopup` с grab, якорь `BottomRight`, гравитация `BottomLeft`, fallback на `LayerShell TOP|RIGHT` когда платформа не поддерживает AnchoredPopup. Весь lifecycle совпадает с `volume_popup/`:

- `POPUP_WIDTH = 360` (было 300)
- `BASE_HEIGHT = 274` (фиксированная — все три блока всегда показаны, высота не от данных)
- `estimate_popup_height()` — функция-геттер для консистентности с volume_popup
- `open(cx, anchor_rect, parent)` — открывает с anchored, падает на LayerShell
- `close(cx)` — через `handle.update`, без reentrancy
- `close_this(window, cx)` — reentrancy guard (HANDOFF.md ghost window saga)
- `toggle(anchor_rect, parent, window, cx)` — bar widget entry point
- `init(cx)` — подписки на brightness + upower, `GamingModeState::init`

### 2. `system_popup/view.rs` — Mockup chrome

Полный пересмотр рендера под `design/System Popup.dc.html`:

- **Blur layer** — `window.paint_blur` (18px, radius_lg углы) как в volume_popup
- **Light C recipe** — тень, inset accent ring, top glow, hexagon-sigil watermark (только для `is_light`)
- **Header** — «System» (13px semibold) + ✕ (через `img("icons/x.svg")`)
- **Brightness block**:
  - Title row: `icons/brightness.svg` (15px) + «Brightness» (12.5px medium) + процент (mono 11px)
  - Control row: `icons/minus.svg` (22×22 кнопка) → `Step(-5)`, трек 4px (full-width), `icons/plus.svg` (22×22) → `Step(+5)`
  - Трек окрашивается в `text_muted.alpha(0.3)` когда бэкенд недоступен
- **Power profile block**:
  - Title + 3-сегмент (Quiet/Balanced/Performance), активный сегмент accent + `on_fill(accent)` text
- **Gaming mode block**:
  - Title row + toggle switch 34×19 (iOS-style, как было)
  - Effect text: «Performance profile · No animations · Do Not Disturb · Hide bar/dock · VSync forced»
- Все секции: `px(PAD=14)`, `py(14)`, divider 1px между ними

### 3. `bar/widgets/system.rs` — Bounds capture + on_mouse_down

Переписан с `on_click` на `on_mouse_down(Left)` c bounds capture через canvas (как `volume.rs`):

- `Rc<Cell<Bounds<Pixels>>>` — захват layout bounds через zero-opacity canvas
- `.relative()` wrapper
- `on_mouse_down(Left)` → `system_popup::toggle(anchor_rect, parent, window, cx)`

### 4. Новые SVG иконки

Созданы 3 иконки (Phosphor-style, viewBox 256, `currentColor`):

- `crates/app/assets/icons/brightness.svg` — солнце (circle + 8 rays)
- `crates/app/assets/icons/minus.svg` — горизонтальная линия
- `crates/app/assets/icons/plus.svg` — крест

Обновлён `assets.rs` — добавлены в макрос `icons!()`.

## Тесты

```
cargo build --release -p chronos  → OK (только pre-existing warnings)
cargo test --release -p chronos   → 151 passed, 0 failed
```

## Не изменилось

- `gaming_mode.rs` — нетронут (backend, `run_hyprctl_eval`, тесты)
- `volume_popup/` — нетронут
- `updates_popup/` — нетронут
- `main.rs` — вызов `system_popup::init(cx)` остался тем же
- `monitor.rs`, `state.rs` — нетронуты

## Проверка live (следующий запуск)

1. Запустить `chronos` (Hyprland session)
2. Кликнуть на hexagon-sigil иконку в баре → popup открывается под иконкой с grab
3. ✕ закрывает попап
4. − / + шагают яркость через `ddcutil`
5. Power profile сегменты переключают профиль
6. Gaming mode toggle применяет `hyprctl eval` и DND флаг
7. Brightness + upower watchers ререндерят попап при внешних изменениях
8. На платформах без AnchoredPopup — LayerShell fallback TOP|RIGHT
