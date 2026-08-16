# T211 report — theme toggle crash (S2) + Follow affordance (F1)

**Отчёт:** 2026-08-03. **Зона:** bar_settings.rs, panel.rs + assets (follow.svg).
**Коммит:** `ui : theme toggle safe + follow affordance (T211)`.
**Источник:** T209 live-smoke — S2 (P0 crash) + F1 (P0 no visual state).

## 1. Theme toggle panic (S2 — P0)

**Симптом:** клик «Toggle» в System settings → `theme.toml` записывается, затем панк:
```
thread 'main' panicked at Source/gpui/src/app.rs:1872:32:
no state of type chronos_ui::theme::Theme exists
```
Ручная правка `theme.toml` и IPC `toggle-theme` работают — только кнопка падает.

**Исправление (bar_settings.rs):** toggle-обработчик шёл через
`cx.update_global::<Theme, _>(...)` — прямое чтение/обновление глобала в контексте,
где он не резолвится. Заменён вызовом **`crate::theme_config::toggle(cx)`** — тот же
путь, что IPC/hot-reload: `set_global` (никогда не паникует — вставляет), persist
схемы, `sync_gpui_component_theme`, refresh окон. Ноль `expect` на недостающий
глобал — `toggle` сам заводит глобал при старте.

## 2. Follow 👁 без визуального состояния (F1 — P0)

**Причина (0 px diff ON vs OFF):** кнопка рендерила **color-эмодзи `👁`** с
`text_color(accent|muted)`. Эмодзи — цветной bitmap, `text_color` на него не влияет.

**Исправление (panel.rs + assets/follow.svg):**
- Новый `currentColor` SVG `assets/icons/follow.svg` (глаз) — тинт через `text_color`, тот же
  паттерн, что `x.svg`/`power.svg`.
- Кнопка: `.child(img("icons/follow.svg").w(16px).h(16px))` вместо `"👁"`.
- Когда **enabled**: `.bg(theme.accent.primary.opacity(0.16))` + `text_color(accent)`
  — фон-подложка + тинт иконки акцентным.
- Когда **off**: `text_color(muted)`, без подложки.
- Поведение (F2/F3) не тронуто — `thread_follow_handler` один в один.

## Verification

- `cargo test -p chronos --lib` — **239/239 зелёных**.
- `cargo build --release -p chronos` — **успешно** (3m 21s).
- `cargo check` — мои файлы (bar_settings.rs, panel.rs) без unused/dead-code
  предупреждений; единственное новое — pre-existing `theme_config.rs` unused import
  (не моя зона, не трогал).
- Иконка `follow.svg` — `fill="currentColor"`, тин expressable, матчит паттерн иконок.
- Диагностика переполнения билдом не внесла новых P0.

## Live smoke — НЕ ПРОВОДИЛСЯ (нужны руки)

Клик Toggle в System settings: тёмный↔светлый без краша. Follow: выкл → серый/без
подложки, вкл → акцентная подложка + акцентная иконка; F2/F3 поведение прежнее.
NOT VERIFIED — честно по правилам (grim опционален).

## Residuals

- S7 (agents.toml live reload) и R2/R3 (half-rate drag) — вне зоны T211, остаются
  за open задачами; T211 их не касается.
- `BorrowAppContext` unused import в theme_config.rs:22 — pre-existing бездействие,
  чистка не в этой задаче.