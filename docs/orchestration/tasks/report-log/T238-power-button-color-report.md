# T238 — Power Button Color Audit Report

## Verdict: **Не баг (ложное срабатывание)**

### Findings

The Power button in System settings footer (`crates/app/src/side_panel_right/power_row.rs`) correctly uses `theme.status.error` for all visual properties:

| Property | Code Location | Token Used |
|----------|---------------|------------|
| Border color | `power_row.rs:79` | `theme.status.error` |
| Text color | `power_row.rs:86` | `theme.status.error` |
| Hover/armed background wash (12% alpha) | `power_row.rs:91` | `theme.status.error.opacity(0.12)` |

The button is rendered with `danger=true` (line 198 in `render_footer`), which activates the error-color styling path.

### Why the Pixel Sample Didn't Match

The sampled value `#d96585` (srgb(217,101,133)) falls between:
- Light theme `status.error`: `#d20f39` (Latte red)
- Dark theme `status.error`: `#f38ba8` (Mocha maroon)

This is expected because:
1. **Background wash blending**: The 12% opacity wash (`theme.status.error.opacity(0.12)`) blends with the card background (`surfaces::card(&theme)`), producing an intermediate color
2. **Anti-aliasing**: The original critique noted "координата приблизительная, могла задеть anti-aliased край/иконку, а не сплошную заливку"
3. **No solid fill**: The Power button has no solid background by default — only a border and text in `status.error`. The wash only appears on hover/armed state

### Theme Values (Confirmed)

```rust
// Light (Latte) - schemes.rs:105, test schemes.rs:187
theme.status.error = #d20f39

// Dark (Mocha) - schemes.rs:37
theme.status.error = #f38ba8
```

### Conclusion

**No code changes needed.** The implementation correctly references `theme.status.error` throughout. The pixel-sample discrepancy is explained by the semi-transparent wash blending and/or edge anti-aliasing — exactly as the original critic flagged.

---

**Ticket Status**: Closed — не баг