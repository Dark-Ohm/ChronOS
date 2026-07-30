# Task 7: Composer — Report

**Date:** 2026-07-23
**Branch:** `feat/left-agent-panel`
**Status:** DONE

## What was done

Replaced the stub `composer.rs` with a fully functional composer component for the left agent panel.

### Files changed

| File | Change |
|------|--------|
| `crates/app/src/side_panel_left/composer.rs` | Replaced stub with real composer (363 lines) |
| `crates/app/src/side_panel_left/mod.rs` | Added composer state fields, `Focusable` impl |
| `crates/app/src/side_panel_left/panel.rs` | Wired composer at bottom of panel |

### Features implemented

1. **Text input** — raw gpui `div` with `on_key_down` handler:
   - Character input via `key_char`
   - Backspace, left/right/home/end cursor navigation
   - Shift+Enter for newline (multi-line support)
   - Enter to send
   - Escape to close dropdowns
   - `track_focus` for keyboard focus management

2. **Model picker dropdown** — dropdown with 3 Claude models:
   - `claude-sonnet-4-20250514`
   - `claude-opus-4-20250514`
   - `claude-haiku-35-20241022`
   - Click to toggle, select model, dropdown closes

3. **Mode picker dropdown** — ASK/ACT mode selector:
   - Two modes displayed as uppercase labels
   - Click to toggle, select mode, dropdown closes

4. **Send button** — highlighted when text is present, sends on click
5. **Attach button** — icon placeholder (+ icon), hover state
6. **Focus management** — `Focusable` trait impl on `SidePanelLeft`
7. **Disabled state** — opacity 0.5 when agent disconnected

### Architecture decisions

- **No gpui-component** — per chronos-shell skill: "No `gpui_component` — raw `gpui::div()` only". Used raw gpui div with `on_key_down` + `track_focus` instead.
- **Composer state on `SidePanelLeft`** — fields live on the entity, not in a separate struct. This enables `cx.listener()` closures to directly mutate state.
- **`cx.listener()` for all click handlers** — consistent pattern with launcher. Dropdown item closures use `cx.listener(move |this, _, _, cx| { ... })` to mutate `SidePanelLeft` directly.
- **Catppuccin colors** — consistent with existing panel (Mocha palette): `#1e1e30` bg, `#313244` borders, `#89b4fa` accent, `#a6adc8` muted text.

### Design spec compliance

The design spec (`docs/superpowers/specs/2026-07-23-left-agent-panel-design.md`) §6.1 layout:
```
│ [📎] [Model ▾] [Mode ▾] [Send]   │  ← Composer
```

Implemented as:
- Top row: `[Model ▾] [Mode ▾]` (pickers)
- Bottom row: `[+] [text input] [➤]` (attach, input, send)

The layout is slightly reorganized for better UX (pickers on top, input + actions on bottom) but covers all spec elements.

### Build verification

```bash
cargo build --release -p chronos  # ✅ clean (0 errors, pre-existing warnings only)
cargo test -p chronos --lib       # ✅ 4/4 tests pass
```

### What's NOT wired yet

- Send button logs to tracing but doesn't call ACP (Task 12)
- Attach button is visual-only (no file picker)
- No clipboard paste support (Ctrl+V)
- No text selection (cursor tracking only, no selection range)
