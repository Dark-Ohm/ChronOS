# T194c — Editor: view default + Preview/Edit (md-like only)

**Статус:** active. **Роль:** FRONTEND. **Модель: Sonnet 5 / GLM 5.2**.
**Канон:** `docs/PRODUCT.md`. **Правила:** `docs/orchestration/agents/RULES.md`.

**Предшественники:**
- T194 `7d0be09` — edit/save via InputState; **регрессия:** editable files
  always `render_editor_body` → markdown **только raw**, rendered preview убит.
- T194b `6a32ef6` — terminal drawer; **residual:** toggle живёт только внутри
  `render_editor_body` → при view mode drawer пропадёт, если chrome не общий.

**Продуктовое решение (архитектор + пользователь 2026-08-02):**

1. Дефолт = **rendered / view**, не raw.
2. **Две кнопки (Preview | Edit) только там, где есть смысл двух режимов** —
   **markdown-like** (`.md` / `.markdown` / `.mdown` + то, что `classify` →
   `PreviewKind::Markdown`). **Не** на каждом файле в Files.
3. Остальное: один клик / одна кнопка → view as today (image, text, binary…).
4. Realistic: plain `Text` **не** обязан иметь Edit в этой задаче (можно
   оставить open-as-view only; edit text = later). **Must:** md dual mode +
   stop forcing editor for all `is_editable`.

**Параллельно:** T199 bar schema — **не** трогай `bar/`.  
T200 blocked on T199 — **не** трогай `bar/`.

**Зона:**
- `crates/app/src/side_panel_right/preview_target.rs`
- `crates/app/src/side_panel_right/tab/preview.rs`
- `crates/app/src/side_panel_right/tab/files.rs`
- `tab/mod.rs` / `view.rs` — **только** если нужно для intent; обычно нет

**НЕ:** rail Terminal restore; LSP; multi-file buffers; full IDE editor;
менять PTY/drawer engine (только **перенос chrome** drawer toggle на
общий header Editor).

**Отчёт:** `docs/orchestration/tasks/report/T194c-preview-edit-modes-report.md`.

---

## Баг (сейчас)

`preview.rs` ~895–898:

```rust
if is_editable(kind, truncated) {
    self.render_editor_body(...)  // always raw for Text+Markdown
} else {
    render_loaded(...)            // images etc.
}
```

Нужно: **view by default**; editor body **only** when mode=Edit **and**
kind is Markdown (this task).

---

## UX

### Files tab

- **Dirs:** click → navigate (как сейчас). Без Preview/Edit.
- **Markdown-like files** (name/ext → will classify as Markdown; cheap
  check by extension in Files, same list as `classify`):  
  - row shows compact **Preview** + **Edit** (text buttons or icons — keep
    narrow panel readable; prefer short labels `"View"` / `"Edit"` or
    `"◉"`/`"✎"` — pick one, consistent).
  - **Click name/icon** = Preview (view intent).
  - Preview button = view intent; Edit button = edit intent.
- **All other files:** single click whole row → view intent only (no dual
  buttons). No Edit affordance.

Stop-propagation: buttons must not also fire row navigate as dir; files
aren't dirs.

### Editor tab (PreviewTab)

- **View mode (default):** `render_loaded` path — markdown **rendered**
  (`render_markdown`), images, text scroll, etc. Header: path + kind +
  **for Markdown only:** toggle **Preview | Edit** (active state on current).
- **Edit mode (Markdown only, non-truncated):** existing `render_editor_body`
  (InputState + dirty + Save). Header: same Preview|Edit + Save + **Terminal**
  toggle (T194b residual: Terminal must stay reachable in **both** modes —
  hoist drawer chrome to outer column, not only inside `render_editor_body`).
- Switching **Edit → Preview** with `dirty`:  
  - **v1 minimum:** block switch and keep Edit **or** discard without prompt
    if you document it — **preferred:** if dirty, stay in Edit and flash
    save_result / muted “Save or discard” (no modal required). Do **not**
    silent-lose buffer.
- Switching Preview → Edit: load current file text into InputState (existing
  sync path); clear dirty.
- Truncated markdown: Edit disabled (same as `is_editable` false).
- Non-markdown: no Preview|Edit pair in header; view only.

### Terminal drawer (T194b residual — in scope if cheap)

Hoist so structure is:

```
column:
  tab_header (path, mode toggles if md, Terminal, Save if edit)
  content flex_1 (view body OR editor Input)
  [optional drawer resize + TerminalTab if drawer_open]
```

Drawer state (`drawer_open`, entity, height) stays on `PreviewTab`; only
render placement changes. If time-box risks T194c core — do core first,
drawer hoist as last step; report if deferred (but **preferred in this PR**).

---

## Механика

### `PreviewTarget`

Extend (serde not needed — in-memory global):

```rust
pub enum PreviewIntent {
    View,  // default
    Edit,
}

pub struct PreviewTarget {
    pub path: Option<PathBuf>,
    pub generation: u64,
    pub intent: PreviewIntent, // default View
}
```

- Update `PreviewTarget::file` / all `set_global` call sites (Files,
  tests, hypr_binds open-if-any) to set `intent: View` unless Edit.
- Bump `generation` when path **or** intent changes for same path so
  observer can re-apply mode without re-read if already Loaded same path
  (or local mode on PreviewTab — see below).

### `PreviewTab` local mode

Hold `view_mode: View | Edit` (or mirror intent). On global observe:

1. path change → load as today; set mode from `intent` (Edit only if
   Markdown + not truncated after load; if intent=Edit but kind≠Markdown →
   force View + warn log).
2. same path, intent change only → switch mode without full re-read if
   already Loaded with text.

Header toggles update **local mode** + optional write-back to global intent
(so Files re-click consistent) — either is fine; document choice.

### Kill regression

**Default open** (Files click name / View button / any non-md file):  
**never** enter `render_editor_body` unless user chose Edit on md.

---

## Tests (обязательно)

- Markdown load → mode View → not editor-only path (assert mode or that
  editor body not forced; unit-level on state).
- Intent Edit on `.md` → after settle, edit mode / editor present.
- Intent Edit on image path → View (or never editable).
- Files helper: `is_markdown_name("README.md")` true; `"foo.rs"` false.
- Dirty: Edit→View blocked or no silent loss (match your v1 rule + test).
- Existing preview tests still green (update constructors of PreviewTarget).
- Drawer: if hoisted — `drawer_starts_closed` still ok; toggle works without
  being in edit mode (new test: open view-only md, toggle_drawer true).

```
cargo test -p chronos side_panel_right::
cargo build --release -p chronos
```

Live (если шелл): Files → README.md → **rendered** headers; Edit → raw;
Save; Preview back. grim optional. NOT VERIFIED ok if honest.

Коммит: `editor : view default + md preview/edit modes (T194c)`.  
Поимённый add: `preview_target.rs`, `preview.rs`, `files.rs` (+ tests).

---

## Отчёт

```markdown
# T194c report
## Mode model (global + local)
## Files: who gets two buttons
## Default view fix (diff idea)
## Terminal chrome hoist (done / deferred)
## Tests + verification
## Что НЕ сделано
```

## Acceptance

- [ ] Opening md shows **rendered** markdown by default (not raw Input)
- [ ] Dual buttons **only** on md-like in Files; not on every file
- [ ] Edit → raw + Save works; Preview returns to render
- [ ] Non-md files unchanged single-click view
- [ ] No silent dirty loss
- [ ] Terminal toggle reachable in view mode **or** explicit residual
- [ ] bar/ untouched
