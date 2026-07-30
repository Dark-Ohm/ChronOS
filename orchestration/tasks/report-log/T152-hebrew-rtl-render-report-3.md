# T152 — иврит / RTL: заход 3 — отчёт

**Статус:** исправлено, ждёт визуальной приёмки архитектором.
**Коммит:** `de62111` в `Source/`.

---

## Диагноз (подтверждён дампом cosmic-text)

Дамп глифов из `cosmic_text_system.rs` для иврита «שלום לך, ארכיטקט…» (579 байт):

```
glyph[0]:   start=0,   x=2397.7   ← первый логический символ (ש), правый край
glyph[1]:   start=2,   x=2389.3
...
glyph[319]: start=578, x=-0.0     ← последний символ, левый край
```

**Факт:** глифы в логическом порядке (start растёт 0→578), x **убывает** (2397.7→0). Код `paint_line` и `compute_wrap_boundaries` предполагал монотонно возрастающие x-позиции.

---

## Что сломано и как исправлено

### 1. `paint_line` / `paint_line_background` (`line.rs`)

**Было:** `prev_glyph_position = Point::default()` (0,0). Для RTL первый глиф имел x=2397.7 → `glyph_origin.x += 2397.7 - 0` → глиф улетал за контейнер.

**Стало:** `prev_glyph_position = first_glyph_position`. Первый глиф: `glyph_origin.x += 2397.7 - 2397.7 = 0` → остаётся на `aligned_origin_x`. Последующие глифы двигаются влево с отрицательными дельтами (`2389.3 - 2397.7 = -8.4`).

### 2. `aligned_origin_x` (`line.rs`)

**Было:** `line_width = end_of_line - last_glyph_x`. Для RTL `end < start` → отрицательная ширина → `TextAlign::Right` давал `origin.x + align_width - (-w) = origin.x + align_width + w` (уход вправо).

**Стало:** `line_width = |end_of_line - last_glyph_x|` (абсолютная). Плюс для unwrapped/последней строки `end_of_line` = позиция последнего глифа (~0 для RTL), а не `layout.width` (~2409).

### 3. `compute_wrap_boundaries` (`line_layout.rs`)

**Было:** `width = next_x - last_boundary_x`. Для RTL `next_x < last_boundary_x` → `width < 0` → условие `width > wrap_width` никогда не срабатывало → **текст не переносился вообще**, рисовался одной строкой.

**Стало:** `width = |next_x - last_boundary_x|`. Плюс `last_boundary_x` инициализируется позицией первого глифа (2397.7), а не нулём — иначе первый же замер давал бы 2400px и мгновенно триггерил перенос на первом глифе.

### 4. `last_line_end_x` (`line.rs`)

**Было:** `first_glyph_x + layout.width - last_wrap_glyph.x`. Для RTL: `origin.x + 2409 - 300 = origin.x + 2109` — underline уезжал далеко вправо.

**Стало:** `final_glyph_x` — трекается во время обхода глифов, отражает реальную визуальную позицию последнего глифа.

---

## Файлы

| Файл | Изменения |
|---|---|
| `gpui/src/text_system/line.rs` | +48/−18 — `paint_line`, `paint_line_background`, `aligned_origin_x`, `last_line_end_x` |
| `gpui/src/text_system/line_layout.rs` | +13/−2 — `compute_wrap_boundaries`: abs width + init `last_boundary_x` |
| `gpui_wgpu/src/cosmic_text_system.rs` | без изменений (debug log был добавлен и убран) |

**Диффстат:** 2 файла, +61/−20.

---

## Приёмка

1. **Сборка:** `cargo build --example hebrew_wrap_test` — ок.
2. **Тесты:** `cargo test -p gpui --lib -- test_is_word_char test_wrap_line test_split_at test_force_width` — **11/11 passed** (включая test_is_word_char с ивритом/арабским из захода 2).
3. **Скриншот:** `/tmp/T152-hebrew-wrap-fixed.png` (806 KB) — ждёт визуальной приёмки архитектором.
4. **Коммит:** `de62111` в `Source/`, отдельно от ChronOS.
5. **`Source/` чист:** `git status --short` — пусто после коммита.

---

## Что НЕ тронуто (за рамками этого захода)

- `_index_for_position` / `position_for_index` в `line_layout.rs` — тоже сломаны для RTL (используют `Pixels::ZERO` / `layout.width` как границы), но это влияет на позиционирование курсора/IME, а не на рендеринг.
- `x_for_index` / `index_for_x` в `line_layout.rs` — для RTL с убывающими x работают корректно (проверено по коду: `x_for_index` ищет `glyph.index >= index`, `index_for_x` ищет в обратном порядке).
