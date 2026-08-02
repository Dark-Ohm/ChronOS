# T194 — Editor = Preview + edit (drop empty Editor tab)

**Статус: ЗАКРЫТА 2026-08-02 — ПРИНЯТА с residual** (архитектор). **Sonnet 5.**
Отчёт: `report-log/T194-editor-from-preview-report.md`. Коммит: `7d0be09`.
**Residual:** terminal drawer (Zed) → **T194b**; live grim N/V; no Ctrl+S; dirty switch discards.
**Канон:** `docs/PRODUCT.md` — Files + Editor (view+edit), не IDE.

**Зона:**
- `tab/preview.rs` (evolve) and/or rename module → `editor.rs`
- `tab/mod.rs`, `tabs.rs` (enum rename **or** Preview stays id `preview` with
  label Editor — prefer stable id `preview`→document as editor to avoid
  scene.toml churn; **or** alias parse_id)
- `preview_target.rs` / Files open path
- `view.rs` arms

**НЕ:** full LSP; multi-buffer IDE; build as rail.

**Terminal (продукт 2026-08-02):** не rail-tab. **Zed-style drawer**
внутри Editor — вытягивается снизу (toggle), PTY из `chronos_services::terminal`
/ существующий TerminalTab engine. Scope T194:
- **Must:** text view+edit+save (core).
- **Should in same PR or immediate follow T194b:** bottom terminal panel
  (collapsed by default, drag height, toggle button in editor chrome).
  If too large — ship edit first, terminal drawer as **T194b** same week,
  not T197 rail restore.

**Отчёт:** `report/T194-editor-from-preview-report.md`.

---

## Цель

1. Пользователь **смотрит и правит** текст, который открыл из Files / agent.
2. Убрать из product path пустой `PanelTab::Editor` (IDE stub).
3. Preview capabilities (image/md/text/binary reject) **сохранить**; text =
   editable buffer + Save (write file with confirm on fail).
4. **Terminal drawer (Zed-like):** снизу Editor — свёрнут по умолчанию;
   toggle (кнопка / shortcut later); drag height; PTY reuse existing
   terminal service. Референс UX: bottom panel under editor content
   (не отдельное layer-shell окно, не rail).

### Scope edit

- Text / markdown source (plain buffer), size cap (e.g. 256–512 KiB) — over =
  read-only + message
- Images stay view-only
- Binary / unavailable — как сейчас T179
- Dirty flag + Save button; optional Ctrl+S if key handling exists cheaply

### Wire

- Files click → open in this Editor tab (switch tab + load)
- T195 later: agent follow opens same path

## Верификация

cargo test + release + live: open README, edit a line, save, reopen.

Коммит: `editor : preview + text edit (T194)`.
