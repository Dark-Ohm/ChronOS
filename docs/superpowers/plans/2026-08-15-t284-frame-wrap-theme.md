# T284 Frame Hide/Wrap Appearance Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Добавить тему оформления Frame (`hide` | `wrap`) без смены дефолтного шелла: Hide = T268, Wrap = рамка по периметру, окна внутри, рельса внутри карточки.

**Architecture:** `frame.toml` хранит `style` и `wrap.inner_radius`. Hide оставляет нижнюю полоску T268 и гасит её, когда рельс нет. Wrap поднимает одну полноэкранную рамку-рисунок (слой Top) и три невидимых exclusive-полоски L/R/B. Бар не двигаем. Рельсы читают `wrap_inset()`, ставят L/R margin и урезают height снизу. Appearance пишет только `style` (RMW). Hyprland не трогаем.

**Tech Stack:** Rust 2024, gpui-ce layer-shell, TOML/serde, inotify 300 ms (уже в `frame.rs`), Theme tokens T267, Hyprland live `hyprctl layers` + grim.

## Global Constraints

- Канон: `docs/superpowers/specs/2026-08-15-desktop-frame-wrap-theme-design.md` и бриф `docs/orchestration/tasks/active/T284-frame-wrap-appearance-theme.md`.
- Это **тема**, не перепись шелла. Дефолт `style = "hide"`. Свежая установка без `frame.toml` пиксельно = T268 (`d572657`).
- Не писать Hyprland (`gaps_out`, rounding, `10-look.lua`).
- Не четвёртый цвет: `theme.bg.tertiary` + `theme.border.subtle`.
- Не пресет бара. Сегмент Frame в Appearance.
- Полноэкранная рамка **без** exclusive zone (fullscreen exclusive резервирует весь экран).
- Дырка и хром рамки: `set_input_region(Some(&[]))`.
- `.mx()` на `.size_full()` переполняет (баг T268) — только flex-спейсеры или якоря.
- `Source/gpui/` не трогать. `Cargo.lock` не коммитить.
- Worktree — sibling репо, не `/tmp` (`path = "../Source"`).
- T277 review-only — писать рядом можно. T281 OPEN: не параллелить `side_panel_left/mod.rs`. Пересечение — стоп, писать архитектору.
- `pkill -x chronos`, не `-f`. Live — release + grim + `hyprctl layers`.
- `Window::set_margin` в форке нет. Live inset рельс — recreate close+open, не охота в Source.
- `frame::apply` не импортирует панели. `after_apply` хук вешает `main.rs`.
- `style`: строка + sanitize, не serde-enum (иначе как junction — весь load → default).
- Запись `style`: `toml::Value` RMW, не дамп `FrameConfig`.
- Wrap + Top exclusive бар: top inset мата = высота бара. Bottom/floating: top inset = `wrap_inset()`.

## File map

| Файл | Роль |
|---|---|
| `crates/app/src/frame.rs` | `FrameStyle`, `WrapConfig`, `wrap_inset`, `set_rail_mapped`, `write_style`, оркестр `apply`, хук `after_apply` |
| `crates/app/src/frame/wrap.rs` **или** тот же `frame.rs` если < ~700 строк | рамка-рисунок + 3 exclusive dummy |
| `crates/app/src/main.rs` (точка init) | `frame::set_after_apply` → `side_panel_*::apply_frame_inset` |
| `crates/app/src/side_panel_left/mod.rs` | `set_rail_mapped(Left)` на open/close; L margin + height -= inset |
| `crates/app/src/side_panel_right/mod.rs` | то же справа; content margin += inset; height -= inset |
| `crates/app/src/bar/mod.rs` | в Wrap снять границу, обращённую в карточку |
| `crates/app/src/side_panel_right/tab/bar_settings.rs` | сегмент Frame → `frame::write_style`, не `bar.toml` |

Цикл модулей запрещён: панели зовут `frame::*` (inset, setter, write). `frame` не импортирует `side_panel_*`. Presence рельс — сеттер в глобал кадра. Нотификация панелей — хук из `main.rs`, не прямой вызов.

---

### Task 1: Config + чистая геометрия

**Files:**
- Modify: `crates/app/src/frame.rs` (`FrameConfig`, `BottomStripConfig`, tests ~L430+)
- Test: тот же `#[cfg(test)]` модуль

**Interfaces:**
- Produces:
  ```rust
  // FrameStyle is Copy/Eq; Deserialize MUST go through a string helper
  // (unknown → Hide + warn). Do not #[derive(Deserialize)] on the enum —
  // that is the junction trap (unknown_junction_value_fails_parse).
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum FrameStyle { #[default] Hide, Wrap }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(default)]
  pub struct WrapConfig { pub inner_radius: f32 } // default 16, clamp 0..=64

  impl FrameConfig {
      pub style: FrameStyle,           // unknown string → Hide + warn
      pub bottom_strip: BottomStripConfig,
      pub wrap: WrapConfig,
  }

  /// 0 in Hide, `bottom_strip.height` in Wrap (after sanitize).
  pub fn wrap_inset() -> f32;

  #[derive(Clone, Copy)]
  pub enum FrameSide { Left, Right }

  /// Panels call this on rail open/close. Triggers `apply`.
  pub fn set_rail_mapped(side: FrameSide, mapped: bool, cx: &mut App);

  /// Hide-strip insets: 0 or RAIL_INSET per side from mapped rails.
  pub fn hide_strip_insets(left_mapped: bool, right_mapped: bool) -> (f32, f32);

  /// Hide strip should exist?
  pub fn hide_strip_wanted(enabled: bool, left_mapped: bool, right_mapped: bool) -> bool;
  ```

- [ ] **Step 1: Красные тесты** в `frame.rs` `mod tests`:

```rust
#[test]
fn missing_style_is_hide() {
    let cfg: FrameConfig = toml::from_str("[bottom_strip]\nenabled=true\n").unwrap();
    assert_eq!(cfg.style, FrameStyle::Hide);
}

#[test]
fn unknown_style_falls_back_to_hide() {
    // implement via sanitize/load helper, not raw deny-unknown
}

#[test]
fn wrap_inset_zero_in_hide_height_in_wrap() { /* ... */ }

#[test]
fn hide_strip_wanted_false_when_no_rails() {
    assert!(!hide_strip_wanted(true, false, false));
    assert!(hide_strip_wanted(true, true, false));
    assert!(!hide_strip_wanted(false, true, true));
}

#[test]
fn hide_strip_insets_one_rail() {
    assert_eq!(hide_strip_insets(true, false), (RAIL_INSET, 0.0));
}

#[test]
fn wrap_radius_clamped() { /* 99 → 64, -1 → 0 */ }
```

- [ ] **Step 2:** `cargo test -p chronos --lib frame::` — FAIL (типов нет).
- [ ] **Step 3:** Минимальные типы + sanitize + `hide_strip_*`. Не открывать окна.
- [ ] **Step 4:** тесты PASS. Старые 6 тестов T268 зелёные.
- [ ] **Step 5:** Commit `feat(frame): style hide/wrap config and hide-strip predicates`

---

### Task 2: Hide — гасить полоску без рельс

**Files:**
- Modify: `crates/app/src/frame.rs` (`apply`, `open`, `BottomStripView::render`)
- Modify: `crates/app/src/side_panel_left/mod.rs` — `set_rail_mapped(Left, true/false)` в open/close рельсы
- Modify: `crates/app/src/side_panel_right/mod.rs` — то же Right

**Interfaces:**
- Consumes: Task 1 predicates + `set_rail_mapped`
- Produces: Hide path respects mapped rails; junction только при видимых рельсах

- [ ] **Step 1:** В `apply` для `FrameStyle::Hide`: если `!hide_strip_wanted(...)` → `close` нижней полоски. Если wanted → open и в render ставить спейсеры `hide_strip_insets`, не хардкод `RAIL_INSET` с обеих сторон.
- [ ] **Step 2:** Панели: при успешном `open` рельсы `set_rail_mapped(..., true)`; в `close`/`close_this` — `false`. Не из `init_hover_strip` — hover-strip не рельса.
- [ ] **Step 3:** `cargo test -p chronos --lib frame::` + `side_panel_left` + `side_panel_right` (регрессия).
- [ ] **Step 4:** Commit `fix(frame): hide bottom strip when side rails are gone`

Дефолт с обеими рельсами не меняется.

---

### Task 3: Wrap — рамка и exclusive-полоски

**Files:**
- Create or extend: `crates/app/src/frame/wrap.rs` (если `frame.rs` раздувается)
- Modify: `crates/app/src/frame.rs` `apply` / `init`

**Interfaces:**
- Consumes: `FrameStyle::Wrap`, `wrap_inset()`, `WrapConfig.inner_radius`, bar height из `bar` appearance / `panel_edge_gap()`
- Produces: namespaces `frame_wrap_matte`, `frame_wrap_excl_left|right|bottom`

Контракт поверхностей:

| surface | layer | exclusive | input | paint |
|---|---|---|---|---|
| matte fullscreen | **Top** (ниже Overlay бара/рельс) | none | empty | только хром, дырка = обои |
| excl L/R/B | Overlay ок, можно Bottom | `height` | empty | ничего |

Дырка: **не** `parent.bg + transparent child` — ребёнок не пробивает заливку. Красить только пиксели хрома (полосы + внутренние четверти радиуса) либо custom Element. Приёмка угла: в укусе обои, не `#181825`/`#ECEEFA`.

Верх мата не закрашивает виджеты бара (бар Overlay сверху). Top exclusive бар: inset сверху = живая высота бара. Bottom/floating: inset сверху = `wrap_inset()`.

- [ ] **Step 1:** Тест на inset-прямоугольник (чистая fn): display 2560×1440, bar 32, height 4, radius 16 → inner `Bounds { x:4, y:32, w:2552, h:1404 }`.
- [ ] **Step 2:** Open/close matte + 3 dummy в `apply` при `style==Wrap`. Hide-полоску закрыть. Toggle обратно — закрыть все четыре wrap-окна, вернуть Hide-логику Task 2.
- [ ] **Step 3:** `set_input_region(Some(&[]))` каждый render (как T268).
- [ ] **Step 4:** `cargo test -p chronos --lib frame::`
- [ ] **Step 5:** Commit `feat(frame): wrap matte window and exclusive edge strips`

---

### Task 4: Рельсы и контент едут внутрь только в Wrap

**Files:**
- Modify: `side_panel_left/mod.rs` `rail_window_options`, `content_window_margin`, `panel_height`, open/close setter. Не начинать, если T281 уже в поле.
- Modify: `side_panel_right/mod.rs` то же (`content_window_margin` сейчас `(top, RAIL_ONLY_WIDTH, 0, 0)`)
- Modify: точка init шелла — зарегистрировать `after_apply`

**Interfaces:**
- Consumes: `frame::wrap_inset() -> f32`
- Produces: left `margin.left = inset`; right `margin.right = inset`; **height = display - bar - inset**; content едет с рельсой. Exclusive рельсы **не** включает толщину рамки (её держат dummy).

Hover-strip **не** сдвигать (физическая кромка).

Live: `Window::set_margin` нет. Recreate close+open уже открытых рельс/контента (как `frame::close`+`open`). Не `window.resize` холста. Не звать `side_panel_*` из `frame.rs`.

- [ ] **Step 1:** Чистый тест `rail_geom(side, inset, top_gap, display_h) -> {margin, height}`: inset=4, bar=32, display=1440 → height=1404, left margin `(32,0,0,4)`.
- [ ] **Step 2:** Подключить. `frame::set_after_apply` в init. `apply_frame_inset` — тонкая fn, не редьюсер вкладок.
- [ ] **Step 3:** Регрессия `cargo test -p chronos --lib side_panel_left` и фильтр `side_panel_right`.
- [ ] **Step 4:** Commit `feat(panels): offset rails by frame wrap inset`

---

### Task 5: Бар сливается с рамкой + Appearance

**Files:**
- Modify: `crates/app/src/bar/mod.rs` ~L123–134 — если `frame::cached_config().style == Wrap` и edge Top: **не** ставить `border_b_1`. Bottom-edge бар: не ставить `border_t_1`. Edit-mode акцент не трогать.
- Modify: `crates/app/src/side_panel_right/tab/bar_settings.rs` — строка Frame (`Hide`/`Wrap`) через `segmented` + `seg_chip`. Клик → `frame::write_style` (RMW ключа в `frame.toml`). Watcher 300 ms. Пресеты бара не трогать.

- [ ] **Step 1:** UI + persist. Нет нового пресета в `PRESETS`.
- [ ] **Step 2:** `cargo test -p chronos --lib bar_settings` и `frame::`
- [ ] **Step 3:** Commit `feat(appearance): Frame hide/wrap control writes frame.toml`

---

### Task 6: Live release (без этого отчёт = отказ)

**Files:** только evidence в отчёте.

- [ ] Release: `cargo build --release -p chronos`
- [ ] `rg -n 'window\.resize\(' crates/app/src/frame.rs` — только старый resize высоты Hide-полоски, не холста панелей.
- [ ] Hide + обе рельсы: `hyprctl layers` как T268, клиенты не сдвинуты, grim угла = break.
- [ ] Hide + обе рельсы закрыты: нижней полоски нет.
- [ ] Wrap: клиент отступил на `height` L/R/B; сверху бар; grim четырёх углов — радиус, обои в укусе, токены T267, обе темы.
- [ ] Wrap + рельса открыта: рельса внутри карточки, не на x=0 / x=display-40.
- [ ] Клик в дырке и по хрому доходит до клиента / peek.
- [ ] Обратный тумблер: exclusive dummy исчезли, клиенты вернулись, лишних `frame_wrap_*` в layers нет.
- [ ] Отчёт: `docs/orchestration/tasks/report/T284-frame-wrap-appearance-theme-report.md` (inbox). Не двигать в `done/` / `report-log/`.

---

## Spec coverage

| Спека | Задача |
|---|---|
| §1 таблица Hide/Wrap | 2, 3 |
| §2 non-goals | Global Constraints |
| §3 frame.toml | 1, 5 |
| §4 Hide predicate | 1, 2 |
| §5 wrap surfaces | 3, 4 |
| §6 Appearance | 5 |
| §7 races | Constraints |
| §8 verification | 6 |
| §9 decisions | весь план |

T268 бриф не переписывать и не исполнять заново.
