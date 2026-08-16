# Spec: Editor — themed buffer + line numbers (not full IDE)

**Дата:** 2026-08-02. **Статус:** approved draft (архитектор + user go).  
**Задача:** `docs/orchestration/tasks/active/T205-editor-theme-and-gutter.md`.  
**Канон:** `docs/PRODUCT.md` — ChronOS **не IDE**; Editor = Files preview + raw edit + Terminal drawer.

---

## 1. Problem

Live (dark shell):

1. **Edit mode** fills with a near-white `Input` canvas — eye strain; panel chrome is dark, buffer is not themed.
2. Buffer has **no line numbers / gutter / monospace editor chrome** — reads as a blank page, not a code/markdown editor.

Root cause (code):

- Edit body: `render_editor_input_body` → `Input::new(...).bordered(false).h_full()` with **no** ChronOS theme bg/text/font on the widget.
- Panel column uses `surfaces::content` (`theme.bg.primary`); gpui-component `Input` keeps its **default** (often light) fill.
- No gutter UI; PRODUCT never required Zed-class editor, T194 shipped notepad-level raw edit.

---

## 2. Goals

1. Edit buffer **matches shell theme** (dark and light) — no pure white glare on dark.
2. **Minimal code chrome:** monospace buffer, 1-based line numbers, optional soft current-line highlight.
3. Preserve View mode (markdown render / image / text), Save/dirty, Terminal drawer, T194c Preview|Edit.

## 3. Non-goals

- Port Zed `Editor` / MultiBuffer / LSP / syntax highlight (phase 2+).
- Multi-file tabs, git gutter, minimap, multi-cursor.
- Changing PRODUCT “not an IDE”.

---

## 4. UX target

```
┌─ chrome: path · Preview|Edit · Terminal · Save ─────────┐
├─ gutter (nums) ─┬─ buffer (mono, themed) ────────────────┤
│  1              │  content…                              │
│  2              │  |                                     │
├─────────────────┴────────────────────────────────────────┤
│ [optional terminal drawer]                               │
└──────────────────────────────────────────────────────────┘
```

- **View:** existing render paths (md/image/text) — no white notepad.
- **Edit:** themed buffer + gutter; not a bare white `Input`.
- Gutter: mono, `text.muted`; width grows with digit count (min ~3 digits).
- Scroll: gutter and buffer **synced** if APIs allow; if not, document residual (buffer-only scroll + static line list is unacceptable for long files — prefer shared scroll or recompute visible range).

---

## 5. Color tokens

Introduce panel surface helpers (names free; prefer next to `surfaces.rs`):

| Role | Dark | Light |
|---|---|---|
| `editor` (buffer bg) | `bg.primary` or slightly elevated dark | **not** `#ffffff`; `bg.secondary` / elevated lavender |
| `editor_gutter` | `bg.tertiary` | `bg.elevated` |
| buffer text | `text.primary` | `text.primary` |
| gutter text | `text.muted` | `text.muted` |
| current line (opt) | low-opacity `interactive.hover` | same |

**Hard rule:** Edit `Input` (or wrapper) must set **explicit** bg + text from `Theme`. Component default white is a bug for this surface.

**Acceptance visual:** dark theme + Edit on `.md` → buffer luminance ≈ panel, not A4 white.

---

## 6. Implementation plan

### Phase A — theme (must ship)

1. Audit gpui-component `Input` for style hooks (bg, text, font family).  
2. Apply mono (`theme.font_mono`) + editor bg/text on Edit path.  
3. If Input always paints opaque light fill: wrapper `div` with editor bg + force Input transparent **or** small fork/component fix (justify in report).

### Phase B — gutter (must ship)

1. Derive `line_count` from buffer text (`lines().count().max(1)`); respect existing 128 KiB truncate.  
2. Left column of numbers `1..=n`, mono, muted.  
3. Sync scroll with buffer (preferred).  
4. Optional: highlight current line if caret line is available from `InputState`; else skip without blocking.

### Phase C — later (out of T205 unless free)

- Status `Ln X, Col Y`  
- Soft wrap toggle  
- Feature-gated syntax (syntect) for md/code  

---

## 7. Files (expected)

- `crates/app/src/side_panel_right/tab/preview.rs` — Edit layout, gutter, Input styling  
- `crates/app/src/side_panel_right/surfaces.rs` — `editor` / `editor_gutter` helpers  
- optional: gpui-component path only if fork fix required (`../Source/…`) — separate commit  

**Not:** `bar/`, rail resize, Files dual buttons (done T194c).

---

## 8. Acceptance checklist

- [ ] Dark Edit: buffer not pure white / not eye-searing  
- [ ] Light Edit: readable, soft paper ok, not glare-white if avoidable  
- [ ] Buffer monospace  
- [ ] Line numbers 1-based, visible  
- [ ] View markdown/image/text unchanged in behavior  
- [ ] Save / dirty / Preview↔Edit guard / Terminal drawer still work  
- [ ] No “full IDE editor” claim in report  

## 9. Product decision (locked)

**Editor v1.5 = themed notepad + line numbers.**  
Syntax/LSP = separate epic.

