# T121 — Volume Popup: Anchored Redesign + Fork Animation/Blur + Skill

**Дата:** 2026-07-25. **Агент:** Hermes (Lead Architect ↔ GLM).
**Задание:** переписать volume popup под стиль отработанных попапов
(`updates`/`notifications`), интегрировать крейт анимаций форка
(`gpui_animation`) + `backdrop-blur`, переключить проект на локальный
форк целиком, создать проектный скилл по попапам.
**Зоны:** `crates/app/src/volume_popup/{mod,view}.rs`,
`crates/app/src/bar/{mod,widgets/volume}.rs`, `crates/app/src/assets.rs`,
`Cargo.toml`, `Source/gpui-animation/{Cargo.toml,src/lib.rs,PATCHES.md}`,
`Source/skills/chronos-gpui-popup/`.

## Что сделано

### 1. Визуальный язык — под `updates_popup` / `notifications`
Попап переписан под тот же «дорогой» recipe, что у эталонов:
- Light C карточка: elevated `BoxShadow` + 1px inset accent ring +
  top accent glow line + `hexagon-sigil` watermark (тот же код, что в
  `updates_popup/view.rs`, под `theme.is_light`).
- `border_1` + `radius_lg` (12px) карточка; `radius` (6px) внутренние
  контролы. `font_mono` везде, `theme.font_sizes`, `border.default/subtle`.
- Разделители `border_b_1` между header / endpoints / footer.
- `card` `.overflow_hidden()` для rounded-clip + watermark.

### 2. Fork skills (требование «используй крейт анимаций»)
- **`gpui_animation`** (`vendored-gpui-animation`): `with_transition` +
  `transition_on_hover` на footer mute-кнопках и device-строках —
  border/color морфят в accent на hover с spring-эзингом. Device-picker
  пружинит open/close через `transition_when` с `EaseOutBack` (обёрнут,
  см. §5).
- **`backdrop-blur`** (`backdrop-blur`): реальное матовое стекло через
  `window.paint_blur(...)` в paint-фазе `canvas` позади карточки. Этого
  НЕТ у `updates`/`notifications` — панель читается как acrylic, а не
  плоский `bg`. Карточка сделана полупрозрачной (`bg.alpha(0.82)`),
  чтобы стекло проступало.

### 3. Якорение (anchored popup + LayerShell fallback)
- `AnchoredPopup` (`PopupAnchor::BottomRight`, `PopupGravity::BottomLeft`,
  `SLIDE_X | FLIP_X`) — привязка к bounds иконки звука в баре.
- Fallback на `LayerShell` (TOP|RIGHT, Overlay,
  `keyboard_interactivity: None`) на платформах без нативного popup.
- `POPUP_WIDTH = 360`. Bounds виджета захватываются через `canvas` +
  `Rc<Cell<Bounds>>` в `bar/widgets/volume.rs`, передаются в `toggle`.
- `open`/`close`/`toggle`/`close_this`/`resize_to_fit`/`init` — как в
  `updates_popup`. `close_this` (из колбэка, держащего `&mut Window`)
  снимает глобальный handle ДО `window.remove_window()` — без ре-входа
  в `handle.update`.

### 4. Проект переключён на локальный форк целиком
**Решение Архитектора (2026-07-25):** git-dep при разработке на форке —
глупо. Весь граф `gpui` теперь разрабатывается in-tree (`../Source/*`),
никаких pinned git-rev в активной разработке.

`ChronOS/Cargo.toml` `[patch."https://github.com/Dark-Ohm/Chronos-GPUI"]`
теперь редиректит **все 16 крейтов форка** на `../Source/`:
`gpui, gpui_collections, gpui_derive_refineable, gpui_linux, gpui_macros,
gpui_media, gpui_platform, gpui_refineable, gpui-rsx, gpui_scheduler,
gpui_shared_string, gpui_sum_tree, gpui_util, gpui_web, gpui_wgpu,
gpui-animation`.

`Source/gpui-animation/Cargo.toml` сохраняет `gpui = { path = "../gpui" }`
(канон скилла `vendored-gpui-animation` — должен быть path-dep, не
version-dep).

### 5. Fork deltas (фиксируются в `Source/gpui-animation/PATCHES.md`)
**Delta 4 — публичный `init` (НОВОЕ).** `TransitionRegistry::init` был
`pub(crate)`, поэтому крейт оставлял `animation_tick` **замороженным** —
каждый `with_transition` / `transition_on_hover` / `transition_when`
оставался мёртвым, пока кто-то не вызовет `init`. Upstream снаружи его
не вызывает. Добавлено в `Source/gpui-animation/src/lib.rs`:
```rust
pub fn init(window: &mut gpui::Window, cx: &mut gpui::App) {
    transition::TransitionRegistry::init(window, cx);
}
```
Забутано один раз за сессию из `Bar::render` (idempotent, `AtomicBool`
guard): `gpui_animation::init(window, cx);`. Это правка, без которой
крейт анимаций не работает на деле, и блокер публикации форка
(приватный boot-entry).

**EaseOutBack адаптер** (в `volume_popup/view.rs`, не в крейте):
`EaseOutBack` отсутствует в `gpui_animation::transition::general` (там
только quad/cubic/sine/expo). Обёрнут `gpui::easing::EasingCurve::
EaseOutBack` в локальный `impl Transition` (`struct SpringBack(f32)`),
чтобы picker мог давать overshoot-spring.

### 6. Ловушка «footer обрезается» (исправлено)
Попап имеет **фиксированную высоту** окна из `estimate_popup_height`.
Контент (header + 2 endpoint + divider + footer dual-mute) выше, чем
старая `BASE_HEIGHT = 240.` → окно отсекало нижний край (кнопки mute).
Поднял до `BASE_HEIGHT = 290.` (header ~37 + Volume ~66 + divider 1 +
Microphone ~66 + footer ~52 ≈ 222, + запас). `resize_to_fit` и подписчик
в `init` считают высоту из той же `estimate_popup_height`, так что фикс
закрывает и open, и expand.

### 7. Проектный скилл `chronos-gpui-popup`
Создан `Source/skills/chronos-gpui-popup/SKILL.md` (канон форка, как
`anchored-popups`/`backdrop-blur`) + симлинк `ChronOS/skills/
chronos-gpui-popup → ../../Source/skills/chronos-gpui-popup`.
Содержит: skeleton попапа (`mod.rs` lifecycle + `view.rs` Render +
anchoring через `Rc<Cell<Bounds>>`), Window-Bounds Trap (footer-clip),
visual language, fork animation boot (`init` + `TransitionExt` +
`on_click` trap + `EaseOutBack` адаптер), `backdrop-blur` паттерн,
rsx-vs-div split, verification (live Wayland обязателен).

## Верификация

Ad-hoc (НЕ suite-green — визуал/анимации требуют живой Wayland-сессии):
- `cargo build --release -p chronos` → **BUILD_EXIT=0**, 0 ошибок.
  Весь локальный форк (16 крейтов) собирается из `../Source/*`.
- `cargo test -p chronos volume` → **12 passed; 0 failed**.
- `grep`: `paint_blur` blur-layer в `view.rs:133`, `blur_layer` встроен
  (view.rs:156); публичный `init` (lib.rs:30), проводка `Bar::render`
  (mod.rs:64); `[patch]` редиректит 16 крейтов; `BASE_HEIGHT = 290.`
  (mod.rs:35).

**Блокер (Архитектор):** нет живой Hyprland/Wayland здесь. Frosted
glass, hover-glow spring, spring-reveal picker и корректность anchor
проверяются запуском шелла (`chronos rebuild` + клик по иконке звука).
Архитектор прогоняет смоуки, отпишется.

## Файлы
- `crates/app/src/volume_popup/view.rs` — blur-layer + `gpui_animation` + стиль.
- `crates/app/src/volume_popup/mod.rs` — anchored popup + `BASE_HEIGHT` fix.
- `crates/app/src/bar/widgets/volume.rs` — захват bounds.
- `crates/app/src/bar/mod.rs` — `gpui_animation::init` boot.
- `crates/app/src/assets.rs` + `assets/icons/microphone*.svg` — иконки микрофона.
- `Cargo.toml` — whole-fork `[patch]` на `../Source/*`.
- `Source/gpui-animation/src/lib.rs` — публичный `init` (Delta 4).
- `Source/gpui-animation/PATCHES.md` — Delta 4 записан.
- `Source/skills/chronos-gpui-popup/` + симлинк `ChronOS/skills/` — скилл.
