# T194b — Editor: Zed-style terminal drawer

**Статус:** active. **Роль:** FRONTEND. **Модель: Sonnet 5 / GLM 5.2**.
**Канон:** `docs/PRODUCT.md` — terminal **inside Editor**, not rail.
**Предшественник:** T194 `7d0be09` (edit/save) — **принят**; drawer не сделан.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно:** T198 RECON (read-only `bar/`) — **не** трогай `bar/`.

**Зона:**
- `tab/preview.rs` (Editor chrome + bottom split) **or** small
  `tab/editor_terminal.rs` composed from preview
- reuse `chronos_services::terminal` / patterns from `tab/terminal.rs`
  (do **not** duplicate PTY engine)
- `tab/mod.rs` / `view.rs` only if needed for layout

**НЕ:** rail `PanelTab::Terminal` restore; layer-shell terminal window;
full multi-tab terminal IDE.

**Отчёт:** `docs/orchestration/tasks/report/T194b-editor-terminal-drawer-report.md`.

---

## UX (референс пользователя — Zed)

1. Editor content сверху; **terminal strip снизу**, default **collapsed** (0 height
   or ~1 toolbar row).
2. Toggle open/close (button in editor header near Save).
3. When open: resizable height (drag handle), min ~80px, max ~50% of tab.
4. PTY: lazy spawn on first open; reuse session while Editor tab entity lives
   (same cache rules as TerminalTab).
5. Keyboard focus: click terminal → input goes to PTY; click editor → InputState
   (don't steal keys while typing in editor).

## Implementation notes

- Prefer flex column: `editor flex_1` + `terminal flex_none` with explicit height.
- Reuse `Terminal` service / `TerminalTab` grid paint if possible by extracting
  shared view helper — avoid copy-paste 500 lines; if extract needs
  `tab/terminal.rs` touch, OK if still one terminal engine.
- Desktop `desktop_terminal` layer-shell surface — **out of scope**.

## Верификация

```
cargo test -p chronos side_panel_right::
cargo build --release -p chronos
```

Live: open Editor → toggle terminal → type `echo ok` → output visible; resize;
collapse. grim желателен.

Коммит: `editor : terminal drawer under editor (T194b)`.
