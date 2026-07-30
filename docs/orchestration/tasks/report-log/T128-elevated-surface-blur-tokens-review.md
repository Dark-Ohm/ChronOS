# T128 review — ACCEPTED WITH CAVEATS

**Date:** 2026-07-25  
**Code:** real and usable. **Report:** partially stale / wrong vs tree.

## Verified

| Claim | Tree |
|---|---|
| `crates/ui/src/elevation.rs` + export | yes |
| `Theme::elevation_popup()` dark/light | yes — dark blur-only, light shadows+glow |
| Blur from tokens in paint_blur | volume, system, updates, history — use `elev.blur.*` |
| Shadows from tokens | `.shadow(elev.shadows.to_vec())` on those cards + panels |
| Panels only when content open | left `chat_open`, right `content_open` |
| 4 unit tests | `cargo test -p chronos-ui elevation` → 4 pass |
| No Source fork edits | yes |
| Toast cards skipped | ok (intentional) |

**Actual tokens (trust code, not report prose):**

```text
BlurSpec { radius: 18px, tint white a=0.06, sat 1.15 }  // both schemes
dark:  shadows=[], glow=None
light: drop y6/blur24 indigo + inset accent ring; glow=accent
```

## Report errors (minion doc debt)

| Report said | Reality |
|---|---|
| blur radius 14, tint_alpha 0.45 | **18 / a=0.06** |
| `blur: Option` — light None | **always `BlurSpec`**; light still has blur params (paint may still run) |
| `elevation_blur_layer` / `elevation_glow_bar` / `elevation_watermark` helpers | **not in `elevation.rs`** — glow+sigil still **copy-pasted** in views |
| `watermark: bool` on tokens | **no such field** |
| `lazy_static` | **`OnceLock`** |

## Caveats (not reject)

1. **Live grim PENDING** — cannot ACCEPT visual “glass works” without screen.  
2. **Helper extraction incomplete** — tokens for blur/shadows done; glow/sigil blocks still duplicated (brief wanted helpers; residual).  
3. volume still has unrelated `BoxShadow::new` on a control (~492) — fine.  
4. Light scheme still paints blur_layer in popups (uses elev.blur even when Light C is shadow-first) — may be intentional “both”; report claimed light blur None.

## Verdict

**ACCEPTED WITH CAVEATS** for code merge.  
Follow-ups: live grim; optional T128.1 extract glow/watermark helpers; fix report if archived.

## Architect action

Commit implementation if uncommitted; move brief toward done on accept after grim optional.
