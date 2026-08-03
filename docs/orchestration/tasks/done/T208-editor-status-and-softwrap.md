# T208 — Editor: Ln/Col status + soft wrap (after T205)

**Статус:** active **T205 ACCEPTED `8b36055` — go**.
**Роль:** FRONTEND. **Спека phase C:**  
`docs/superpowers/specs/2026-08-02-editor-themed-notepad-gutter.md` §6C.
**Правила:** `RULES.md`.

**Зона:** `tab/preview.rs` only (Edit chrome).

**Цель:** status line `Ln X, Col Y` (if InputState exposes caret); soft-wrap
toggle. Still **not** syntax/LSP.

**Отчёт:** `report/T208-editor-status-and-softwrap-report.md`.

Коммит: `editor : status line + soft wrap (T208)`.
