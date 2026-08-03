# T217 — Отчёт: Верхняя кромка панелей честно стыкуется с баром

**Дата:** 2026-08-03
**Статус:** Код готов, юнит-тесты green (6/6 новых), release-билд OK (exit 0). Живой
Wayland-прогон (grim в трёх конфигурациях бара) НЕ выполнен — см. «Верификация».

## Что было

Замер из брифа: бар — пилюля `radius = 16`, `width = "fraction:0.7"` (x 384..2176 на
2560), панель живёт у правого края. Верх панели совпадал с низом бара случайно
(оба y=34), а над хвостом панели бара не было — стык читался как случайный, панель
начиналась с квадратной кромки там, где под ней не было бара.

Требование брифа: стык должен выглядеть намеренным при любой конфигурации бара.
Три вещи: (1) кромка в кромку по живой высоте, (2) скругление верхних углов панели
по `[appearance] radius`, когда бар их не накрывает (рифма с пилюлей), 0 — когда
накрывает (иначе полукруглый шов), (3) обе панели и обе hover-полосы ведут себя
одинаково.

## Решение

`bar` — bin-only, панели — lib: межкрэйтовое общение через атомики в `crate::state`
(тот же паттерн, что у живой высоты `bar_height_px()`). Бар публикует радиус и
горизонтальный охват; панели решают радиус каждого верхнего угла одной общей
функцией. Root правой панели больше не красит фон целиком — иначе скругление не
пробивалось бы наружу.

### `crates/app/src/state.rs`

Добавлены атомики `LIVE_BAR_RADIUS_BITS` / `LIVE_BAR_X0_BITS` / `LIVE_BAR_X1_BITS`
(паттерн f32-в-bits, как у высоты) + API:

- `bar_radius_px()` — живой `[appearance] radius`.
- `bar_x_extent() -> (f32, f32)` — горизонтальный охват бара `[x0, x1]` на pult-дисплее.
- `set_bar_geometry(radius, x0, x1)` — пишет все три сразу (вызывается баром).
- `panel_corner_radius(x)` — **единственное место с правилом стыка**: `x` внутри
  охвата бара → `0.0` (панель подтыкается под нижнюю кромку бара, скругление дало
  бы шов), вне охвата → `bar_radius_px()` (кромка свободна, рифмуется с пилюлей).

Дефолт статиков описывает полноширинный квадратный бар (охват `0..INF`): пока бар
не применился / дисплей не перечислен — панели сохраняют докантский квадратный
хром. Это же делает `publish_bar_geometry` при отсутствии pult-дисплея.

### `crates/app/src/bar/mod.rs`

- `bar_screen_x_extent(display_w, &appearance) -> (f32, f32)` — зеркалит margin/anchor
  математику из `window_options`, чтобы опубликованный охват совпадал с тем, что
  реально поставил композитор. Учтены три нюанса:
  - `BarWidth::Full` + `!floating` → `(0, display_w)` (тянется до краёв, покрывает всё);
  - floating-полоса вставляется margin.x с обоих краёв поверх выравнивания
    (`Center`/`End` считаются от `max(leftover/2, margin)` / `max(leftover, margin)`);
  - `Fraction(f)` → `display_w * f`.
- `publish_bar_geometry(cx)` — читает `cached_appearance()` + pult-дисплей, пишет
  радиус+охват, затем высоту (сохраняет контракт T200). Без дисплея — безопасный
  дефолт `(0.0, 0.0, INF)`.
- Вызов добавлен в **`apply_appearance`** (первым делом — до destroy/reopen, решение
  зависит только от санитизированного appearance) и в **`init`** (до открытия панелей,
  у которых hover-стрипы стартуют ~50 мс, бар ~100 мс). Дублирующий
  `set_bar_height_px(appearance.height)` из success-ветки `apply_appearance` удалён —
  теперь высоту публикует `publish_bar_geometry`.
- Тесты: `bar_screen_x_extent_full_non_floating_covers_display`,
  `bar_screen_x_extent_fraction_centered`, `bar_screen_x_extent_fraction_end_touches_right_edge`,
  `bar_screen_x_extent_floating_insets_every_edge` (в т.ч. кейс, где center-float с
  почти полной шириной не может влезть в margin и держит margin).

### `crates/app/src/side_panel_right/{mod.rs,view.rs}`

- **mod.rs** — `Root::new(view, window, cx).bordered(false).bg(gpui::transparent_black())`.
  gpui-component `Root` красит `tokens.background` на всю поверхность окна, и никакое
  скругление на view не пробилось бы через этот фон. Перенос всего видимого хрома на
  собственные div панели открыл скруглённые вырезы на рабочий стол.
- **view.rs** — на каждый рендер: `corner_tl = panel_corner_radius(display_w - panel_width)`
  (правый край экрана минус ширина панели → левый верхний угол панели),
  `corner_tr = panel_corner_radius(display_w)`. На корневой div
  `when(corner_tl > 0 || corner_tr > 0, …)` → `.rounded_tl/.rounded_tr/.overflow_hidden()`.
  Пересчёт каждый рендер → хот-релоад бара (radius/width/floating) и resize панели
  применяются живьём, без пересоздания окна. Когда оба угла накрыты (полноширинный
  бар) — без скругления и без clip, тени elevation не режутся.

### `crates/app/src/side_panel_left/{mod.rs,panel.rs}`

- **panel.rs** — левая панель лево-анкореная, screen x идёт `0..width`:
  `corner_tl = panel_corner_radius(0.0)`, `corner_tr = panel_corner_radius(panel.state.width)`.
  Тот же `when(...)` → rounded + overflow_hidden. База окна у левой панели и так
  прозрачная (нет gpui-component Root) — вырезы показывают рабочий стол.
- **mod.rs** — расширен кросс-панельный тест консистентности T204:
  `panel_top_corners_follow_the_same_bar_junction_rule` — левый и правый углы одной и
  той же screen x не могут разъехаться, т.к. оба считаются через общий
  `state::panel_corner_radius`.

### Hover-полосы

Стрипы живут поверх панелей (rail-only по краю), поэтому наследуют скругление панели
бесплатно — отдельного кода не потребовалось, что и требовал пункт 3 брифа.

## Тесты

Новые (все прошли):
- `bar::tests::bar_screen_x_extent_*` — 4 теста охвата бара (full / fraction center /
  fraction end / floating inset).
- `state::tests::bar_geometry_round_trips_and_junction_rule` — round-trip атомиков +
  правило угла (свободный край → radius, под баром → 0, граница охвата → 0). Единственный
  тест, мутирующий process-wide статики; в конце восстанавливает дефолт для других тестов.
- `side_panel_left::tests::panel_top_corners_follow_the_same_bar_junction_rule` — общий
  инструмент для обеих панелей.

## Верификация

- `cargo check -p chronos --lib` и `--bin` → exit 0 (варнинги pre-existing, не из правок).
- `cargo build --release -p chronos` → **Finished release**, exit 0 (7m36s, LTO). Бинарь
  новее последней правки.
- **Важный нюанс команды из брифа:** `cargo test -p chronos --lib side_panel` НЕ покрывает
  часть тестов T217. `mod bar` и `mod side_panel_left` объявлены только в bin-таргете
  (`crates/app/src/main.rs`), в `lib.rs` их нет, поэтому `--lib` их не компилирует.
  Полный прогон: `cargo test -p chronos --bin chronos` → **`421 passed; 2 failed`**, из
  них все 6 новых тестов T217 — green.
- Два падающих теста — **НЕ связаны с T217**:
  1. `side_panel_right::tab::files::tests::files_view_edit_buttons_match_preview_contract`
     (`files.rs:527`) — известный WIP T222 (незакоммиченные правки files.rs/preview.rs,
     проверено ранее через `git stash`).
  2. `wallpaper_ctl::tests::scan_wallpapers_sorted` (`wallpaper_ctl.rs:205`) — окружение:
     в реальной `~/Pictures/Wallpapers` есть пары `Musely.ai-generation-1(1)` vs
     `...-1`, ломающие наивную проверку сортированности. Файл не тронут в этом дереве.

## Следующий шаг

Живой прогон обязателен (из брифа, рецепт ниже) — «зелёные тесты для оконного кода
ничто»:

```bash
hyprctl layers | grep -E 'namespace: (bar|side_panel)'
```

Проверить три конфигурации бара (правятся в `~/.config/chronos/bar.toml`, хот-релоад
при сохранении):
- `width = "full"` — верх панели == низ бара, углы квадратные, шва нет;
- `width = "fraction:0.7"` — верх панели == низ бара, углы скруглены по `radius`,
  шва нет, сверху справа нет ощущения обрезка;
- `floating = true` — углы скруглены (бар не накрывает кромку), панель не перекрывает
  пилюлю.

`grim` в обеих темах. Если визуально расходится — вернусь править.

**Коммит:** `panels : top edge meets the bar cleanly (T217)` — 6 файлов, +296/−6
(`state.rs`, `bar/mod.rs`, `side_panel_left/{mod.rs,panel.rs}`, `side_panel_right/{mod.rs,view.rs}`).
