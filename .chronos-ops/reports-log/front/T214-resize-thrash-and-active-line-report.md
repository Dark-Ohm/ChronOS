# T214 report — Right resize thrash + editor active line

**Отчёт:** 2026-08-03. **Статус:** done — code in tree **`d2fa7c7`** (architect
dogfood fix), verification в этой сессии пройдена. **Источник:** live dogfood
2026-08-03 — «колбасит при resize»; «выбранная строка не подсвечивается».

## 1. Resize thrash (P0)

**Root (зафиксирован):**
- T210 ввёл frame-to-frame `width - delta` в `update_resize` +
  re-base `start_x = current_x` на каждом кадре;
- `render` при изменении ширины делал `start_x += Δw`;
- итого **двойная коррекция** (и через delta, и через сдвиг start_x) → осцилляция.

**Fix — anchor-модель (T214):**
- `start_resize` фиксирует пару `(start_x, start_w)` на всю длительность драга:
  - из rail-only (≤ `RAIL_ONLY_WIDTH+1`) → expand: `start_x += (target - w)`,
    `start_w = target`;
  - из content: `start_x = start_x`, `start_w = w`.
- `update_resize`: `new_w = start_w - (current_x - start_x)`; **не** re-base
  `start_x` на текущий кадр (комментарий в коде прямо запрещает это —
  `start_x` сдвигается только в `render` на `Δw` после `window.resize`).
- `render`: после `window.resize` — `resize_start_x = map(|x| x + Δw)` только
  когда `old_w > RAIL_ONLY_WIDTH + 2.0` (rail→content уже сместил start_x в
  `start_resize`, повторный сдвиг был бы двойным).
- `resizing` peek-hold (T210) и mouse-up reset (`resize_start_x/width = None`)
  сохранены.

**Юнит:** `drag_left_grows_right_anchored_width` — закрепляет контракт
`new_w = start_w - (current_x - start_x)` (move left 20px, start 200 → 220).

## 2. Active line (P1/P0 dogfood)

gpui-component красил `editor_active_line` из stock highlight-темы (`#171717`)
— невидимо на ChronOS-буфере (`surfaces::editor ≈ bg.primary`). В
`sync_gpui_component_theme` после `Theme::change` (и после font-lock
JetBrains Mono) теперь мапится из shell-токенов:
- `editor_active_line = shell.interactive.hover.opacity(dark 0.5 / light 0.4)`;
- `editor_active_line_number = dark ? accent.primary : text.primary`.

Пайнт-path рисует полосу только при gutter line_number (code_editor) — без
него band'а нет; это задокументировано в коде.

## Verification

- `cargo test -p chronos --lib side_panel_right::tests` — **8/8 passed**
  (включая `drag_left_grows_right_anchored_width`, `resize_clamps`,
  `peek_close_suppressed_while_resizing`).
- `cargo build --release -p chronos` — **успешно**, pre-existing warnings
  only (`drop(state)` lint и пр.), ничего нового в изменённых файлах.
- **Live smoke — NOT VERIFIED в эту сессию.** Требует рук: перетащить правый
  хендл плавно (без колбасы, канва под курсором 1:1); в Edit каретка должна
  подсвечиваться полосой. Логика покрыта юнитами и код-ревью anchor-модели;
  сам render/window.resize round-trip живьём не гонялся — процесс
  пользователя не трогал, чтобы не разрывать чужую live-сессию.

**Коммит:** `panels : resize anchor fix thrash + active line (T214)` (`d2fa7c7`).
