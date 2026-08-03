# T213 — Editor: edit **all text** files (not markdown-only)

**Статус:** active **P0 dogfood**. Источник: Architect + daily driver  
(«в чём смысл редактора, если только md»).  
**Отменяет** T194c errata `e884411` **в части** «plain Text stays View-only».  
**Роль:** FRONTEND. **Модель: Sonnet / GLM.**  
**Правила:** `RULES.md`.

**Зона (узкая):**
- `crates/app/src/side_panel_right/tab/preview.rs` — `can_toggle_edit` / intent / chrome
- tests in same file that assert Text cannot toggle Edit
- **не** bar, left panel, ACP settings

## Контекст

T194c: dual Preview|Edit **only for markdown**; plain `Text` forced View  
even when `is_editable(Text)` is true. Follow/ACP open `.toml` → log  
`Edit intent on non-editable kind, forcing View` → blank utility.

User dogfoods ChronOS **instead of Kate**. md-only edit = useless for  
config/code/logs.

## Цель

1. **Any loaded text buffer** that is not truncated and is  
   `PreviewKind::Text` **or** `PreviewKind::Markdown` can enter **Edit**  
   (InputState + Save/dirty as today).
2. **Markdown:** keep dual chrome **Preview | Edit** (View = rendered md).
3. **Plain text** (toml, rs, log, txt, lua, py, html, sh and all editable files … classified as `Text`):  
   - open → **Edit by default** (or single mode without fake Preview render);  
   - no useless dual that pretends preview for plain mono text unless useful.
4. **Image / binary / truncated / error** — still **not** Edit (honest).
5. Follow / `PreviewIntent::Edit` on a text path **must not** force View.

## Не цель (T213)

- Syntax highlight / LSP / multi-file tabs (D2) — later.
- Current-line highlight / gutter flush (D1/D3) — separate unless one-liner.
- Re-open T194c philosophy debate — product call is **done**: editor = files.
- Port FM `PreviewEditor` as a crate or copy lines — **0 copied source**.

## Reference: Chronos-FM (не эталон, уже что-то)

Sibling repo — **rewrite-by-pattern only**, legal: не копировать строки.

| path (Chronos-FM) | зачем смотреть |
|---|---|
| `crates/chronos-fm-pages/src/explorer/preview.rs` | classify + async open + text→editor |
| `…/explorer/view/preview/editor.rs` | `InputState::code_editor` + line_number + language |
| `…/explorer/types.rs` | **почти не про preview** (sort/grid/search) — не трогать как образец Editor |

### Что у FM уже есть (полезно)

1. **Любой UTF-8 text → editor buffer**, не md-only (`PreviewOutcome::Text` → `PreviewEditor`).
2. **`detect_language(path)`** — ext → `"rust"|"toml"|"markdown"|…|"plain"` → `set_highlighter`.
3. **SVG = text**, raster image = binary short-circuit (`is_binary_image_path` excludes svg).
4. **TooLarge / Unsupported** honest messages; size cap from config.
5. **Async read** + discard if `preview_path` changed (stale click).
6. **Gutter** `line_number(true)`; soft_wrap default **false** (у нас T208 default true — не ломать без причины).

### Что FM **не** даёт (не копировать как product)

- Preview **read-only** — нет Save/dirty. ChronOS **должен** Edit+Save.
- `appearance(false)` без shell theme surface — у нас T205 `surfaces::editor` правильнее.
- Search/highlight stubs half-done.
- Explorer `types.rs` chrome — другой продукт.

### Mapping → ChronOS T213

| FM | ChronOS target |
|---|---|
| any UTF-8 → PreviewEditor (view) | any UTF-8 Text\|Md → **Edit+Save**; Md dual View\|Edit |
| `detect_language` + highlighter | **не в T213** — optional follow-up T213b / dogfood D-syntax |
| SVG as text | nice-to-have if `classify` today marks svg Image — only if one-liner |
| stale path discard | keep existing `generation` load |

**Минимум T213** = снять markdown-only gate + default Edit for Text + tests.  
Language map из FM — **фаза 2**, не раздувать diff.

## Implementation sketch

```rust
// Today (wrong for dogfood):
fn can_toggle_edit(kind, truncated) -> bool {
    matches!(kind, PreviewKind::Markdown) && !truncated
}

// Target:
fn can_edit_buffer(kind, truncated) -> bool {
    matches!(kind, PreviewKind::Text | PreviewKind::Markdown) && !truncated
}
// chrome: dual Preview|Edit only for Markdown;
// Text: always Edit when can_edit_buffer (or View=raw mono + Edit same buffer)
```

Update tests:
- `edit_intent_on_plain_text_also_forces_view` → **invert** (settles Edit).
- md dual still only for markdown-like extensions.
- image/truncated still force View.

## Verification

```
cargo test -p chronos --lib preview
cargo check -p chronos
```

Live (user or re-smoke):
- open `bar.toml` / `agents.toml` / `.rs` → Edit, type, Save
- open `README.md` → View default, Edit works
- open image → View only
- Follow write_file to `.txt` → opens editable if intent Edit

## Коммит

`editor : edit all text files not only md (T213)`.

**Отчёт:** `report/T213-editor-all-text-files-report.md`.
