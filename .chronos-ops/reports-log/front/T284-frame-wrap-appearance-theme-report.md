# T284 — тема оформления Frame: Hide и Wrap — отчёт

**Статус:** код готов, юниты и release зелёные. **Live release-прогон
(Task 6) НЕ выполнен** — требует перезапуска шелла; договорённость с
владельцем не была выбрана (вопрос задан, ответ не получен). Чек-лист
живого прогона — в конце.

**Коммитов нет** — дерево оставлено для ревью архитектора.

## Что сделано (по задачам плана)

### Task 1 — Config + чистая геометрия (`crates/app/src/frame.rs`)
- `FrameStyle { Hide, Wrap }` — **строка** + `deserialize_style` +
  `from_str_sanitized` (unknown → Hide + warn, не падение и не молчаливый
  Wrap; НЕ serde-enum — ловушка junction). `#[serde(default)]` + поле,
  отсутствие ключа = Hide.
- `WrapConfig { inner_radius }` — дефолт 16, кламп 0..=64.
- `FrameConfig.style` + `wrap`; `cached_config()` санитайзит оба.
- `wrap_inset_for(&cfg)` / `wrap_inset()` — 0 в Hide, `bottom_strip.height`
  в Wrap.
- `FrameSide` + `RAIL_MAPPED` (AtomicU8) + `set_rail_mapped(side, mapped, cx)`
  (триггерит `apply`).
- `hide_strip_wanted(enabled, l, r)` / `hide_strip_insets(l, r)` — предикаты
  §4 (0/1/2 рельсы).
- `wrap_inner_rect(display_w, display_h, bar_h, inset)` — дырка мата.
- 10 новых тестов (16/16 зелёные, старые 6 T268 целы).

### Task 2 — Hide: гасить полоску без рельс
- `apply` разбит на `apply_hide` / `apply_wrap`; в Hide полоска живёт
  только пока `hide_strip_wanted`, спаны `hide_strip_insets` вместо
  хардкода `RAIL_INSET` с обеих сторон.
- Панели: `set_rail_mapped(Left/Right, true)` в CommitBoth, `false` в
  `close()` и `close_this()`. `init_hover_strip` не трогал — hover-strip не
  рельса.
- `init` больше не открывает полоску вслепую на 40 ms — просто `apply(cx)`;
  с обеими закрытыми рельсами полоски нет, как требует §4.

### Task 3 — Wrap: рамка и exclusive-полоски
- 4 поверхности: matte (Layer::Top, fullscreen, **без** exclusive —
  fullscreen exclusive резервирует весь экран), excl L/R/B (Overlay,
  exclusive = `height`, `exclusive_edge` выставлен). Все
  `set_input_region(Some(&[]))` каждый render. Partial-open откатывается.
- Мат красит только хром: 3 полосы + 4 угловые заплатки с per-corner
  радиусом (`rounded_tl/tr/bl/br` — генерируются макросами форка). Дырка
  = обои, никаких «заливка + прозрачный ребёнок» (§5.1).
- Тoggle обратно закрывает все 4 окна (`close_wrap_windows`), Hide-логика
  Task 2 возвращается.

### Task 4 — Рельсы/контент едут внутрь только в Wrap
- Левая: `rail_window_options` margin = `(0,0,0,inset)` в Wrap;
  `content_window_margin` = `(top, 0, 0, RAIL_WIDTH + inset)`;
  `panel_height` = display − bar − inset. Правая зеркально
  (`(0, inset, 0, 0)` / right = `RAIL_ONLY_WIDTH + inset`).
- **Отклонение от примера плана:** в примере Task 4 Step 1 левый margin
  рельсы = `(32,0,0,4)` (top = 32). Не стал — top-margin на TOP-anchored
  поверхности поверх top-exclusive бара даёт **двойное смещение**
  (замерено, `gpui-layer-shell` Part A: «TOP|RIGHT + margin top =
  BAR_HEIGHT → double offset»). Верхний отступ рельсы даёт exclusive бара
  автоматически, как сейчас; margin меняет только боковой слот.
- `frame::set_after_apply` + хук в `main.rs` (после init панелей) →
  `apply_frame_inset` обеих панелей. `frame.rs` панели не импортирует
  (цикл модулей запрещён — соблюдено).
- `apply_frame_inset`: закрытые панели просто подхватят геометрию при
  следующем открытии; открытые — recreate close+open. Левая сохраняет
  `panel_width`/`dock_content` (dock не схлопывается); правая открывается
  заново rail-only (её штатный open) и восстанавливает `width`.
  Hover-strip не сдвинут (физическая кромка).
- **Live-сдвиг — recreate, а не `Window::set_margin`** (в форке нет),
  `Source/` не тронут.

### Task 5 — Бар сливается с рамкой + Appearance
- `bar/mod.rs`: при `style == Wrap` карточная граница не ставится
  (top-bar → без `border_b_1`, bottom-bar → без `border_t_1`); edit-mode
  акцент не тронут.
- `bar_settings.rs`: сегмент **Frame** в сетке Appearance (row после
  Exclusive zone): `Hide` | `Wrap` через `segmented`/`seg_chip`, активное
  состояние = `frame::cached_config().style`, клик → `frame::write_style`
  (RMW в `frame.toml`, не в `bar.toml`, не пресет бара). Watcher 300 ms.

### Прочее
- `crates/app/src/lib.rs`: **lib-близнец `frame`** (паттерн `dock`/
  `start_menu`) — без него `cargo test --lib` не собирается: панели (lib)
  зовут `crate::frame`. В release-бинаре lib не линкуется — состояние
  кадра живёт один раз в bin-копии; дублирования статиков нет.
- `write_style`: RMW через `as_table_mut().insert()`. Найдена ловушка:
  `doc["style"] = ..` в toml 0.8.23 **паникует** «index not found» на
  отсутствующем ключе (IndexMut не вставляет) — тест это поймал.

## Верификация (что прогнано)

```
cargo check -p chronos                                        — чисто
cargo test -p chronos --lib frame::                           — 16/16
cargo test -p chronos --lib side_panel_left                   — 119/119
cargo test -p chronos --lib side_panel_right                  — 198/198
cargo test -p chronos --lib (весь)                            — 592/592
cargo build --release -p chronos                              — чисто (3m 07s)
rg -n 'window\.resize\(' crates/app/src/frame.rs              — только старый
    resize высоты Hide-полоски (строка 806), холст панелей не трогается
```

Тесты по плану Task 1: `missing_style_is_hide`, `unknown_style_falls_back_to_hide`,
`wrap_inset_zero_in_hide_height_in_wrap`, `hide_strip_wanted_*`,
`hide_strip_insets_one_rail`, `wrap_radius_clamped`,
`wrap_inner_rect_matches_spec` (2560×1440, bar 32, inset 4 →
`{x:4, y:32, w:2552, h:1404}`), `write_style_preserves_unknown_keys`,
`write_style_overwrites_existing_style_key`.

## Что НЕ сделано (честно)

- **Live release-прогон (Task 6) не выполнен.** Юнит без пикселей —
  мало (план прямо это говорит); код прошёл только статику и юниты.
  Живой прогон требует перезапуска шелла (chronos-start) и работы с
  Hyprland (hyprctl layers/clients, grim 4 угла, обе темы, клик-навылет,
  toggle туда-обратно). Вопрос «кто гоняет» был задан владельцу — ответа
  не было. Либо владелец гоняет сам (прецедент T298/T287-B), либо скажет
  мне — перезапущу шелл.
- Коммиты не делал (по задачам плана, поимённый `git add` — как только
  владелец скажет).
- Известный кавеат recreate: toggle Wrap при открытой левой панели =
  закрыть/открыть панель (ACP-реконнект, та же дыра T285 холодного
  старта); правая панель открывается rail-only (штатно). Это следствие
  принятого решения «recreate close+open» плана.

## Чек-лист живого прогона (Task 6, из брифа)

1. Hide + обе рельсы: `hyprctl layers` как T268 (exclusive рамки = 0),
   клиенты не сдвинуты, grim угла = break.
2. Hide + рельсы закрыты: нижней полоски нет.
3. Wrap: клиенты отступили на `height` L/R/B, сверху бар; grim 4 углов —
   радиус виден, обои в «укусе», хром без четвёртого оттенка, обе темы.
4. Wrap + открытая рельса: рельса внутри карточки (не x=0 /
   x=display−40), не на нижнем хроме.
5. Клик в дырке и по хрому доходит до клиента / peek, не до рамки.
6. Обратный тумблер: `frame_wrap_*` слоёв в `hyprctl layers` нет,
   клиенты вернулись, граница бара вернулась.

---

## Приёмка архитектора — 2026-08-18: ПРИНЯТ (с оговоркой)

Сверено по дереву, а не по отчёту:
- Код **в git** — `d01820e6` («frame,theme : T284 wrap style (matte ring + rail
  geometry) + T266 surface alpha/blur»). Заявление отчёта «коммитов нет, дерево
  оставлено для ревью» на момент приёмки устарело: изменения закоммичены
  (тем же коммитом, что и T266 — несамодостаточный коммит, HEAD не собирался
  сутки; отдельный урок, в вину T284 не ставится).
- Символы на месте: `FrameStyle` (`frame.rs:94`), `from_str_sanitized:113`,
  `set_rail_mapped:316`, `hide_strip_wanted:338`, `wrap_inner_rect:358`;
  потребители — `bar/mod.rs:141`, `side_panel_{left,right}/mod.rs`,
  `bar_settings.rs:294-300,649-656`.
- Тесты прогнал сам: `cargo test -p chronos --bins frame` → **16 passed,
  0 failed** (включая `unknown_style_falls_back_to_hide`,
  `wrap_inner_rect_matches_spec`, `write_style_preserves_unknown_keys`).

**Оговорка:** живой прогон (Task 6) отчёт честно не заявлял — и был сделан
позже архитектором с grim-разбором. Найденные хвосты (толщина рамки не
разведена с радиусом, debug-лог, обрезка полоски по краям) вынесены в
**T303**, а не возвращены в T284. Геометрия к моменту заведения T303 уже
переписана в одно кольцо тем же `d01820e`.
