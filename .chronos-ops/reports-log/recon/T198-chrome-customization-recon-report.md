# T198 report — RECON: hardcoded chrome props → gap to bar appearance schema

**Роль:** RECON. **План:** `docs/superpowers/plans/2026-08-02-live-customization.md`.
**Правила:** `docs/orchestration/agents/RULES.md`. **Код не тронут.**
**Fork:** `../Source` (gpui-ce chronos edition) — факты по API из кода, не из live-сессии.

---

## 1 Property table (file:line)

| # | property | status | где живёт |
|---|---|---|---|
| 1 | **bar edge** (top/bottom) | **hardcoded TOP** | `crates/app/src/bar/mod.rs:241` `Layer::Top`, `:242` `anchor: Anchor::LEFT\|RIGHT\|TOP` |
| 2 | **height** | **hardcoded `BAR_HEIGHT = 30.0`** | `crates/luau/src/bar.rs:16`; потребление: `bar/mod.rs:235` (window_bounds size), `:243` (exclusive_zone) |
| 3 | **width** full vs fraction | **full only** — `window_bounds.size.width = display_size.width` | `bar/mod.rs:233-235` (`display_size` из `cx.find_display`) |
| 4 | **align** (start/center/end) | **не существует** — всегда растяжение `LEFT\|RIGHT` | `bar/mod.rs:242`; origin фиксирован `point(px(0.), px(0.))` `:234` |
| 5 | **margin** | **hardcoded `margin: None`** | `bar/mod.rs:244` |
| 6 | **floating** | **понятия нет** — бар всегда edge-stretched + exclusive | `bar/mod.rs:239-245` (LayerShellOptions целиком) |
| 7 | **exclusive zone** | **hardcoded `Some(px(BAR_HEIGHT))`** | `bar/mod.rs:243`; live-сеттер в форке **есть**: `Window::set_exclusive_zone` `../Source/gpui/src/window.rs:2005` (бар его не зовёт) |
| 8 | **radius / clip** | **0, без clip** — корневой div без `.rounded()`/`overflow_hidden()`; есть `border_b_1()` (bottom-бар захочет `border_top`) | `bar/mod.rs:88-97` (`bg(theme.bg.tertiary)` `:89`, `border_b_1()` `:90`, `border_color(theme.bg.elevated)` `:91`, `.px(px(10.))` `:96`) |
| 9 | **shadow / elevation / blur** | **на баре нет**; токены существуют только для попапов | `crates/ui/src/elevation.rs:33-83` — `ElevationTokens { shadows, blur (18px), radius, glow, watermark }`, `Theme::elevation_popup()` `:52`; консьюмеры: volume/updates/notifications/system_popup view.rs |
| 10 | **bg color / theme binding** | **theme-bound, не из конфига**: `bg(theme.bg.tertiary)` + `border_color(theme.bg.elevated)`; `theme.toml` управляет только `scheme` | `bar/mod.rs:89-95`; `crates/app/src/theme_config.rs` (resolve `:77-98`, apply `:100-106`) |
| 11 | **widget lists L/C/R** | **✅ config + hot-reload** (единственное, что уже живёт в файле) | `bar/layout_config.rs:27-39` struct (`left/center/right/known`), `:383` `apply()`, `:431` `spawn_watcher()` |
| 12 | **hot-reload re-apply** | **только widgets (registry) и theme scheme**; геометрия/внешность НЕ переприменяются | `layout_config.rs:383-395` — registry rebuild + `cx.refresh_windows()`, WindowOptions не трогаются |

Реальный пользовательский файл (read-only): `~/.config/chronos/bar.toml` — только
`left/center/right/known`, без `version`, без `appearance`. `theme.toml`:
`scheme = "Default"`. `dock.toml`: только `pinned`.

---

## 2 Hot-apply path today

Когда меняется `bar.toml`:

1. **inotify-тред** (`layout_config.rs:431+` `spawn_watcher`): watch **parent dir**
   (не файл), mask `CLOSE_WRITE|MOVED_TO|CREATE|DELETE|MODIFY`, фильтр по
   basename `bar.toml` → `mpsc::unbounded_channel<()>`.
2. **GPUI-таск** (`layout_config.rs:491`): trailing-debounce **300 ms** →
   `apply(cx)`.
3. **`apply`** (`layout_config.rs:383`): `BarLayoutConfig::load().sanitized()` →
   `update_cache` → `widgets::apply_layout` (clear + re-register по `slots()`) →
   `reregister_plugin_widgets` → `cx.refresh_windows()`.

Итог: меняется только содержимое `BarWidgetRegistry`. **`WindowOptions`
(anchor/bounds/exclusive/margin) заданы один раз в `window_options()`
(`bar/mod.rs:220-250`) при `open_window` и больше не обновляются.**

Что можно менять mid-session **уже сегодня** (fork API):

| операция | API | где |
|---|---|---|
| размер surface | `Window::resize(size)` | `../Source/gpui/src/window.rs:2318` (используется notifications — Part B паттерн) |
| exclusive zone | `Window::set_exclusive_zone(px)` | `window.rs:2005` |
| exclusive edge | `Window::set_exclusive_edge(Anchor)` | `window.rs:2014` |

**Re-anchor (top→bottom) mid-session — публичного API НЕТ:**
- `PlatformWindow` trait (`../Source/gpui/src/platform.rs`): только `resize` `:722`,
  `set_exclusive_zone` `:803`, `set_exclusive_edge` `:805`. Ни `set_anchor`,
  ни `set_margin`, ни `set_layer`.
- Wayland-impl ставит anchor/margin **один раз при создании**:
  `../Source/gpui_linux/src/linux/wayland/window.rs:169` (`set_anchor`), `:175`
  (`set_margin`).
- При этом **протокол `zwlr_layer_shell_v1` это умеет** — `set_anchor`,
  `set_margin`, `set_layer` валидны в любом коммите. Значит ре-anchor = маленький
  fork-патч (3-4 метода в trait + Window-wrapper + wayland-impl + commit), либо
  `remove_window`+re-open (рискованно: ghost-window сага / «window not found» —
  skill `wayland-window-lifecycle`; бар — persistent surface).

Вывод: hot-reload путь существует и работает для widgets/theme, но **для
appearance его не существует** — T200 строит с нуля поверх существующего
inotify-скелета.

---

## 3 Schema gap — план §4 `[appearance]`

| поле плана | статус сегодня | gap для T199/T200 |
|---|---|---|
| `edge = "top"` | hardcoded TOP (`bar/mod.rs:241-242`) | **новое поле** + re-anchor (fork `set_anchor` или destroy/recreate) |
| `height = 30` | hardcoded const (`luau/bar.rs:16`) | **новое поле**; apply = `window.resize` — уже есть (`window.rs:2318`) |
| `width = "full"` | full only (`bar/mod.rs:233-235`) | **новое поле**; `fraction` = `display_w × f` + resize; `hug` = measure-then-resize (см. риск 3) |
| `align = "center"` | растяжение (`bar/mod.rs:242`) | **новое поле**; центр-якоря в wlr нет — center = `margin_left = (display_w − bar_w)/2` при anchor `LEFT\|TOP`; start/end = anchor + margin по одному краю |
| `margin = {x, y}` | `margin: None` (`bar/mod.rs:244`) | **новое поле**; live `set_margin` в форке нет — тот же fork-gap, что и у anchor |
| `floating = false` | понятия нет (`bar/mod.rs:239-245`) | **новое поле**; обязательная связка: floating ⇒ `exclusive_zone: None`; вдобавок **нет `input_region`** в `LayerShellOptions` (`../Source/gpui/src/platform/layer_shell.rs:59-77`) → клики бьют по всей ширине окна даже при визуальной «пилюле» |
| `exclusive = true` | `Some(px(BAR_HEIGHT))` (`bar/mod.rs:243`) | **новое поле**; live `set_exclusive_zone` есть (`window.rs:2005`) — cheapest win |
| `radius = 0` | 0, без clip (`bar/mod.rs:88-97`) | **новое поле** → `.rounded(px(r))` + `.overflow_hidden()` в корне render. **Не заблокирован композитором** — чисто наш рендер |
| `elevation = "none"` | на баре нет (`elevation.rs` только попапы) | **новое поле** → маппинг на `ElevationTokens` (`elevation.rs:33-83`); `window.paint_blur` работает на любой layer-shell surface (попапы доказывают) — бар может взять `elevation_blur_layer`/`elevation_glow_bar` |
| `[widgets] L/C/R` | **уже есть + hot-reload** | — (переиспользовать как есть) |
| `bg` override | theme-bound (`bar/mod.rs:89`) | theme.toml token-overrides — следующая волна; бар уже theme-driven |
| `version = 2` | нет | новое поле; v1-файлы без `version` грузятся как сегодня (план §4) |

Жёстко заблокировано композитором: **ничего** — протокол умеет всё. Реальные
gap'ы: (а) отсутствие live `set_anchor`/`set_margin`/`set_layer` в форке, (б)
отсутствие `input_region` в `LayerShellOptions`, (в) центр-выравнивание = ручная
margin-математика.

---

## 4 Risks T199/T200

1. **Top→bottom flip.** Live re-anchor API нет (см. §2). Рекомендация: маленький
   fork-патч `set_anchor`/`set_margin`/`set_layer` (протокол поддерживает,
   plumbing в `gpui_linux` уже есть — `wayland/window.rs:169/175` вызываются при
   создании) вместо `remove_window`+re-open. Re-open persistent bar = ghost-window
   и «window not found» расы (skill `wayland-window-lifecycle`).
2. **Floating + exclusive.** Если floating не снимает exclusive — зона резервируется
   от края при плавающем баре, зазоры окон неправильные. Плюс `input_region`:
   в форке нет поля → невидимая кликабельная область на всю ширину окна.
   Варианты: fork `set_input_region` (wlr умеет) или осознанно full-width hover.
3. **Hug width.** gpui задаёт размер surface из `WindowBounds` при open; реальная
   ширина контента известна только после layout/paint. Нужен feedback-loop
   render→measure→resize (паттерн notifications, Part B `gpui-layer-shell`).
   Бар — тонкая flex-полоса; вторая фаза может дать фликер. **v1: full/fraction,
   hug — позже.**
4. **Потребители top-bar-допущения.** `BAR_HEIGHT` (и «бар сверху») зашит вне
   bar/mod.rs:
   - `side_panel_right/mod.rs:52` `PANEL_EDGE_GAP = BAR_HEIGHT`,
   - `side_panel_right/hover_strip.rs:18`,
   - `side_panel_left/mod.rs:41`, `side_panel_left/hover_strip.rs:16`,
   - OSD **bottom-anchored** (`osd/mod.rs:112` `Anchor::BOTTOM|LEFT|RIGHT`) —
     bottom-бар пересечётся с OSD.
   T200 обязан аудитить всех потребителей, а не только окно бара. `refresh_windows`
   WindowOptions не переприменяет — нужен per-window handle-store и явные
   resize/set_exclusive_zone.
5. **Agent пишет невалидный toml** (план §6.3). У layout уже есть `sanitized()`
   (`layout_config.rs:318`) + warn-not-panic load (`:114-130`). Для appearance —
   тот же паттерн + keep-last-good + `version`. Пресеты — отдельный файл/секция.
6. **Один бар на pult-дисплее, не per-display.** `bar::init` (`bar/mod.rs:266-290`)
   открывает одно окно через `pult_display_id_or_primary` (`monitor.rs:204`,
   monitor.toml uuid → largest → primary). `docs/ARCHITECTURE.md` §4 говорит «every
   display» — код фактически pult-only. Для multi-bar/geometry-per-display — отдельно.

---

## 5 Out of scope (подтверждено)

- **Vertical bar** (left/right edge) — план: «later vertical bar»; schema `edge`
  резервирует значения, но T199/T200 не обязаны их реализовывать.
- **Multi-bar / per-display bars** — вне плана v1; сейчас одно окно на pult
  (см. риск 6).
- **Dock geometry.** Dock **не** отдельное окно (skill-текст «Dock: Layer::Top,
  BOTTOM» — исторический): dock = bar-widget (`bar/widgets/mod.rs` instantiate
  `"dock"` → `bar/widgets/dock.rs`; старый dock-window lifecycle удалён —
  `docs/MEMORY.md` задание #8). `dock/config.rs` — только `pinned`,
  **без hot-reload watcher**, без геометрии/размера/edge.
- **Полные token-overrides в theme.toml** (radius/shadow/цвета темы) — следующая
  волна после appearance.
- **Панели** (default widths/open state) — план упоминает, но T199/T200 — бар.

---

## Что НЕ сделано

- **Продуктовый код не тронут** (RECON-мандат): ни bar/, ни dock/, ни fork.
- **Живой прогон hot-reload/re-anchor не выполнялся** — нет compositor-сессии
  (Terminal Shell). Все fork-факты — из исходников `../Source`, не из live:
  `window.rs:2005/2014/2318`, `platform.rs:722/803/805`, `layer_shell.rs:59-77`,
  `gpui_linux .../wayland/window.rs:169/175`. «Ре-anchor без remove_window работает
  на Hyprland 0.56» — НЕ ПРОВЕРЕНО, требует live-проверки в T200.
- **Коммит не делал** — отчёт оставлен архитектору. Если коммитить:
  `recon : T198 chrome customization map`.

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED WITH NOTE**

Сверка с деревом:

| claim | проверка |
|---|---|
| BAR_HEIGHT const + window | ✅ `luau/bar.rs:16`, bar window_bounds/exclusive |
| anchor TOP\|L\|R, margin None, Layer::Top | ✅ `bar/mod.rs:241-244` |
| render: bg tertiary, border_b, no root radius/clip | ✅ `:88-97` |
| layout hot-reload only widgets + refresh_windows | ✅ `apply` `:383`, watcher parent+basename, 300ms |
| live resize / set_exclusive_zone / set_exclusive_edge | ✅ `window.rs:2318/2005/2014` |
| no live set_anchor/set_margin/set_layer on PlatformWindow | ✅ trait — только resize + exclusive + input_region |
| LayerShellOptions no input_region at create | ✅ `layer_shell.rs` fields |
| BAR_HEIGHT consumers panels/strips | ✅ left/right mod + hover_strip |
| OSD bottom anchor | ✅ `osd/mod.rs:112` |
| dock = bar widget, not window; pinned only | ✅ `dock/mod.rs`, config |
| pult single bar | ✅ `bar::init` + `pult_display_id_or_primary` |
| user bar.toml widgets only | ✅ live file |
| live re-anchor not smoke-tested | ✅ honest residual for T200 |

**NOTE (поправка к §2/§3 risk floating):**
отчёт: «нет `input_region` в форке».  
Факт: **create-time** поля в `LayerShellOptions` нет — верно.  
**Live API есть:** `Window::set_input_region` (`window.rs:2029`) + Wayland impl
(`gpui_linux .../window.rs:1854`). Floating-клики через full-width — не «надо
писать fork с нуля», а wire mid-session `set_input_region` после resize/geometry.
T200: использовать, не ждать нового fork-API.

**Мелочь:** line-cites `layout_config` struct «:27-39» слегка уехали (struct ~40+);
суть верна.

**Код не тронут** — RECON-мандат соблюдён. Коммит не требовался.

**Следом (не стартовать без отдельного go):**
- T199 schema `[appearance]` + load/sanitize/version
- T200 apply path (height/exclusive/radius cheap first; edge → fork set_anchor)
- hug width / multi-bar / vertical / theme token overrides — out of v1

