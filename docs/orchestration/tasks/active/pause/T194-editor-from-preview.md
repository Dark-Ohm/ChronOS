# T194 — Editor = Preview + edit (drop empty Editor tab)

**Статус:** BLOCKED — после **T192** (rail labels/for_mode settled).
**Роль:** FRONTEND. **Модель: Sonnet 5** (text input / gpui-component cost).
**Канон:** `docs/PRODUCT.md` — Files + Editor (view+edit), не IDE.

**Зона:**
- `tab/preview.rs` (evolve) and/or rename module → `editor.rs`
- `tab/mod.rs`, `tabs.rs` (enum rename **or** Preview stays id `preview` with
  label Editor — prefer stable id `preview`→document as editor to avoid
  scene.toml churn; **or** alias parse_id)
- `preview_target.rs` / Files open path
- `view.rs` arms

**НЕ:** full LSP; multi-buffer IDE; terminal/build.

**Отчёт:** `report/T194-editor-from-preview-report.md`.

---

## Цель

1. Пользователь **смотрит и правит** текст, который открыл из Files / agent.
2. Убрать из product path пустой `PanelTab::Editor` (IDE stub).
3. Preview capabilities (image/md/text/binary reject) **сохранить**; text =
   editable buffer + Save (write file with confirm on fail).

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
