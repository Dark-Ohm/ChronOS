# T225 report — Selection paints as per-line quads (D5 grid fix)

**Отчёт:** 2026-08-03. **Зона:** ФОРК `Source/gpui-component/crates/ui/src/input/element.rs`
(+ новый `Source/gpui-component/PATCHES.md`). База форка `57f582f` (`component/feature-gates`),
версия `0.5.2`. **Статус:** code in tree; fork-check + геометрия-юниты + ChronOS-check зелёные;
**live grim не гонялся** (нужны руки + композитор).

## Что было (причина бага)

`layout_match_range` (был L518–660) возвращал **один** `Path<Pixels>` — контур,
обведённый вокруг ВСЕХ выделенных строк:
- прямой проход — `top_right/bottom_right/bottom_left` на строку;
- обратный проход — `top_left` + мостовая точка **только** когда следующая строка
  начинается правее (`next.top_left.x > corners.top_left.x`);
- `end_x = end_x.max(start.x + 6px)` — костыль для пустых строк.

Отсюда: (1) самопересечение контура при разных x-границах строк → заливка по
even-odd/nonzero даёт «сетку» (live D5); (2) мост только в одну сторону — при
строке слева заливка уходит по диагонали; (3) 6px-култышки.

## Правка (per-line quads, один путь на видимую строку)

`TextElement`:
- `layout_match_range → layout_match_ranges`, возвращает `Vec<Path<Pixels>>` —
  по одному закрытому выпуклому прямоугольнику на видимую строку. Самопересечение
  структурно невозможно, мостовые точки не нужны, порядок не важен.
- Чистая геометрия вынесена в окно-независимый хелпер
  `match_line_spans(start, end, line_height, line_width) -> Vec<(top_left, bottom_right)>`
  — софт-врапы дают N шпанов; **zero-width** (пустые строки, continuation-row,
  кончающаяся на колонке 0) рисуются на **всю ширину видимой строки** вместо 6px-костыля.
- `6px`-костыль и односторонний мост удалены.
- Общие вызывающие приведены к `Vec`: `layout_search_matches` (поиск с тем же кодом),
  `layout_selections`, `layout_hover_highlight` (lsp), `layout_document_colors` (lsp).
- `PrepaintState.selection_path` / `.hover_highlight_path`: `Option<Path>` → `Vec<Path>`;
  `paint` итерирует векторы. Пустой `Vec` эквивалентен `None` для всех вызывающих.
- `layout_match_ranges`/`layout_search_matches` — общий код для выделения и поиска
  (фикс касается обоих, как и задумано в карточке).

## Verification

- `cargo check -p gpui-component` (default) — чисто.
- `cargo check -p gpui-component --features lsp` — чисто (затронутые lsp-ветки).
- `cargo test -p gpui-component --lib input::element::tests::test_match_line` —
  **4/4**:
  - `single_line_mid_start` — одна строка, старт с середины → 1 прямоугольник;
  - `next_line_starts_left` — случай «следующая строка левее» (тот, что был без
    моста) → 2 непересекающихся прямоугольника;
  - `empty_line_full_width` — пустая строка → вся ширина строки, не 6px;
  - `soft_wrapped` — перенос: 3 визуальные строки, ряд-кончающийся-на-0 → вся ширина.
- `cargo check -p chronos` — зелёно (ChronOS берёт форк через `[patch]`
  `path = ../Source/gpui-component/crates/ui`; изменения подхватываются автоматически).

## Не сделано (честно)

- **Live grim в обеих темах — остаётся за руками.** Задача прямо требует: открыть
  длинный файл в правой панели, тянуть мышью выделение вниз/вверх/через пустые
  строки/переносы, затем то же для поиска; снять кадры в Default и Light. Я не
  гонял — требуется композитор + интерактивная сессия. Структурно сетка исключена
  (выпуклые quads), но «зелёные тесты для оконного кода ничто» — live обязателен
  до закрытия.
- Слоистость заливки (`selection` + `secondary_selection`, paint L2030–2047)
  **проанализирована, не менялась**: ChronOS не переопределяет цвет выделения,
  источник — стоковая тема форка; разнотон от двух полупрозрачных слоёв на одной
  области — это отдельная тема, не D5-сетка.

**Коммит (в форке):** `input : selection paints per-line quads, not one polygon (T225)`
— **не делал** (по правилам форка коммит — проводка архитектора). `PATCHES.md` создан и
документирует дельту (база, почему, API-изменение, что перепроверить при бампе).
