# T214 — Right resize thrash + editor active line

**Статус:** done `d2fa7c7` (code in tree), verify passed 2026-08-03; live smoke pending hands.  
**Источник:** live dogfood 2026-08-03 — «колбасит при resize»; «выбранная строка не подсвечивается».

## Resize thrash (P0)

**Root:** T210 frame-to-frame `width - delta` + re-base `start_x = current_x`
**and** `render` `start_x += Δw` → double correction → oscillation.

**Fix:** restore **anchor** model: fixed `start_w` + `start_x` for drag;
`new_w = start_w - (current_x - start_x)`; only `start_x += Δw` after
`window.resize`. Keep T210 `resizing` peek-hold.

## Active line (P1/P0 dogfood)

gpui-component paints `editor_active_line` from highlight theme; default dark
`#171717` invisible on ChronOS buffer. **Fix:** after `Theme::change` in
`sync_gpui_component_theme`, set `editor_active_line` /
`editor_active_line_number` from shell `interactive.hover` / accent.

## Verify

```
cargo test -p chronos --lib side_panel_right::tests
cargo build --release -p chronos
```

Live: drag right handle smoothly; caret line band visible in Edit.

**Коммит:** `panels : resize anchor fix thrash + active line (T214)`.
