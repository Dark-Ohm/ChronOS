# T303 — wrap-рамка: развести толщину/радиус, убрать debug-лог, живой grim

**Роль: FRONTEND.** Найдено архитектором живьём (2026-08-17), фидбек
владельца: «обёртка выглядит как дешёвая отмазка... полоска
обрезанная по краям, никакого сходства с тем что я просил».
**Приоритет:** P2 (снижено с P1 — геометрия уже переписана коммитом
`d01820e`, остался хвост тюнинга, не архитектурная правка).

## Что уже сделано (коммит `d01820e`, 2026-08-18)

T284-реализация `WrapSurfaceView::render` (frame.rs:518-533) была
переписана до одного кольца **до** заведения этого тикета — на момент
находки бага (grim-разбор живого шелла) диагноз был верный (толщина
рамки не читается, углы не связаны), но версия кода на диске уже была
другой (5 corner-patch div заменены на `div().border(px(inset))
.rounded(px(radius + inset))`, комментарий "One uniform ring instead
of strips + hand-drawn corner patches"). Тикет ниже переписан под
фактическое состояние дерева, а не под старый диагноз.

## Контекст

Живой прогон (`chronos-start`, релизный бинарь, монитор DP-1
2560×1440, `hyprctl monitors -j` reserved `[4, 30, 4, 4]`).
`~/.config/chronos/frame.toml`:

```toml
style = "wrap"
[bottom_strip]
enabled = true
height = 4.0
junction = "break"
# [wrap] отсутствует → inner_radius = DEFAULT_INNER_RADIUS = 16.0
```

Видео-референс владельца (`/home/neo/Videos/soramane.mp4`,
Noctalia-style desktop): рамка заметно толще 4px, радиус соразмерен
толщине — единая скруглённая карточка.

## Задача

1. **`wrap.thickness` — новое поле**, отдельное от
   `bottom_strip.height`. Сейчас `wrap_inset_for` (frame.rs) в Wrap-ветке
   берёт `cfg.bottom_strip.sanitized().height` — общее число с Hide-
   режимом (4px, `= gaps_out/2`, обоснованно для тонкой Hide-полоски,
   но не для Wrap-рамки). Добавить `WrapConfig::thickness` (дефолт
   ~16, отдельная граница sanitize, разумная — сверить с
   `inner_radius`, чтобы радиус не был кратно больше толщины по
   умолчанию), `wrap_inset_for` в Wrap-ветке читает его вместо
   `bottom_strip.height`. Обновить `wrap_inner_rect`/вызовы, где
   логика уже предполагает единое число — свести на новый источник.

   **Второй разъезд, найденный при ревью (не гипотеза — конкретные
   строки):** `WrapSurfaceView::render` (frame.rs:510) читает
   `cfg.bottom_strip.height` — сырое значение, без `.sanitized()` —
   а `wrap_inset_for` (frame.rs:347) читает
   `cfg.bottom_strip.sanitized().height`. Сегодня оба совпадают только
   потому, что `cached_config()` сам санитайзит `bottom_strip` перед
   тем как отдать `cfg` — это не гарантия, а везение текущего пути
   вызова. Если `height` когда-нибудь окажется вне
   `[MIN_HEIGHT, MAX_HEIGHT]` при обходе `cached_config()` (прямой
   `FrameConfig::load()`, будущий рефактор), рамка нарисуется одной
   толщиной, а exclusive-strips/inset рейлов зарезервируют другую —
   клиенты уедут не на ту величину, рамка ляжет поверх. После
   введения `wrap.thickness` оба места читают одно санитизированное
   значение через единый геттер; сырых `cfg.*.height` в рендере не
   остаётся.
2. **Убрать debug-мусор**: `tracing::error!("T303DEBUG matte
   bounds={:?} scale={}", ...)` в `WrapSurfaceView::render`
   (frame.rs:511-516) — это временная диагностика прошлой сессии,
   осталась в закоммиченном коде. Убрать полностью (не понижать до
   `debug!` — просто не нужна).
3. Живой grim на DP-1 до/после — `hyprctl monitors -j` для reserved
   zone, `grim` + `magick crop` на все 4 угла и середину каждого края.
   Сверить визуально с видео-референсом (единая рамка без разрывов,
   толщина/радиус не выглядят рассинхронизированными). Приложить к
   отчёту.
4. `cargo check`/`cargo test --lib` на пути — тесты `wrap_radius_clamped`,
   `wrap_inner_rect_matches_spec` уже существуют, добавить тест на
   новое поле `thickness` (default + clamp).

## Зона файлов

`crates/app/src/frame.rs` — `WrapConfig`/`wrap_inset_for`/
`WrapSurfaceView::render` (frame.rs:133-163, 496-540 — актуальные
номера строк сверить на месте, дерево двигалось). Не трогать Hide-
strip (`BottomStripView`) — отдельный режим, не в скоупе.

## Отчёт

`.chronos-ops/reports-fresh/T303-frame-wrap-border-geometry-mismatch-report.md`
