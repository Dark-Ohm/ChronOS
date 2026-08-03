<!-- Архив 2026-08-02: SUPERSEDED / не исполнять. См. notes/superseded. -->

# T197 — SUPERSEDED 2026-08-02

**Было:** вернуть Terminal в default rail (T192 cut).

**Продукт:** Terminal **не** rail-tab. Живёт **внутри Editor** как
Zed-style bottom drawer (референс UX пользователя).

- Drawer: **T194 / T194b** — `active/T194-editor-from-preview.md`
- Engine: `crates/services/src/terminal/` + patterns from `tab/terminal.rs`
- Rail `PanelTab::Terminal` may stay in ALL dormant; not in `for_mode`

**Не выполнять этот бриф.**
