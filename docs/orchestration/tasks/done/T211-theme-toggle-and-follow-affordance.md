# T211 — Theme toggle crash + Follow toggle affordance

**Статус:** done (отчёт подан, коммит готов). **Источник: T209 FAIL S2 + F1.**  
**Роль:** FRONTEND.  
**Отчёт T209:** `report-log/T209-live-smoke-residuals-report.md`  
**Лог S2:** `/tmp/t209-smoke/20260803-0250/chronos.log` —  
`no state of type chronos_ui::theme::Theme exists` then wayland abort.

## 1. Theme Toggle panic (P0)

`bar_settings.rs` toggle uses `cx.update_global::<Theme, _>(…)`.  
Live: writes `theme.toml` then **panics** whole shell. Hand-edit `theme.toml`  
hot-reloads fine (`theme: hot-reloaded`).

**Must:** toggle must use the same path as file hot-reload / `theme_config::toggle`  
(or `App` context where Theme global is registered). No panic; dark↔light live.  
No `expect` on missing global — degrade + log.

## 2. Follow 👁 no visual state (P0 affordance)

`panel.rs` thread-follow: color-emoji `👁` + `text_color(accent|muted)`.  
Emoji is bitmap → **0 px diff** ON vs OFF  
(`F1-follow-on-nohover.png` ≡ `F1-follow-off-nohover.png`).

**Must:** tintable icon/SVG or text label ("Follow") whose color/bg changes.  
F2/F3 behaviour already works — only affordance.

## Зона

- `side_panel_right/tab/bar_settings.rs` + `theme_config.rs` (reuse, don't fork)
- `side_panel_left/panel.rs` (+ assets if SVG)

**Отчёт:** `report/T211-theme-toggle-and-follow-affordance-report.md`  
Коммит: `ui : theme toggle safe + follow affordance (T211)`.
