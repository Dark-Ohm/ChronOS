# T205 — Editor: themed buffer + line numbers

**Статус:** active (отчёт подан, приёмка ждёт release build после T202). **Роль:** FRONTEND. **Модель: Sonnet 5 / GLM 5.2**.
**Спека:** `docs/superpowers/specs/2026-08-02-editor-themed-notepad-gutter.md`.
**Канон:** `docs/PRODUCT.md` — не IDE; Editor = view + raw edit + drawer.
**Предшественники:** T194 / T194c (view+edit+drawer) — **done**.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно:** T202 bar presets — **не** трогай `bar/`, `bar_settings*`.
Правый resize — **не** эта задача (отдельный residual).

**Зона:**
- `crates/app/src/side_panel_right/tab/preview.rs`
- `crates/app/src/side_panel_right/surfaces.rs` (editor surface helpers)
- gpui-component Input styling **only if** required and path is clear;
  fork `../Source/gpui-component` only with separate commit + justification

**НЕ:** Zed Editor port, LSP, syntax highlight, multi-file tabs, bar, Files buttons.

**Отчёт:** `docs/orchestration/tasks/report/T205-editor-theme-and-gutter-report.md`.

---

## Цель

1. **Edit mode не белый прожектор** на dark shell — buffer bg/text из Theme.
2. **Line numbers** (gutter) + **monospace** buffer.
3. View / Save / dirty / Terminal drawer / Preview|Edit **без регрессий**.

## Must

- Explicit editor surface tokens (see spec §5).
- `Input` (or wrapper) themed; mono font on buffer.
- Gutter `1..N` synced scroll if possible; honest residual if API blocks.
- Tests: at least surface helpers + “edit path sets mono/themed” if testable; existing preview tests green.

## Verification

```
cargo test -p chronos side_panel_right::tab::preview::
cargo test -p chronos side_panel_right::
cargo build --release -p chronos
```

Live: dark theme → open `.md` → Edit → buffer not white; numbers visible; Save; back to Preview render. grim optional. NOT VERIFIED ok if honest.

Коммит: `editor : themed buffer + line gutter (T205)`.

---

## Отчёт

```markdown
# T205 report
## Theme / Input approach
## Gutter + scroll
## Verification
## Что НЕ сделано
```
