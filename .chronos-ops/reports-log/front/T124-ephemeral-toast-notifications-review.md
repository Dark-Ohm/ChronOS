# T124 review — ACCEPTED WITH CAVEATS (2026-07-25)

**Report:** `report/T124-ephemeral-toast-notifications-report.md`  
**Verdict:** **ACCEPTED WITH CAVEATS**

## Evidence

| Claim | Result |
|---|---|
| Width 340, LIST_MAX_H 480, margin top 12 right 16 | ✅ `mod.rs` |
| `render_toast_card` independent of history | ✅ history uses `render_history_card` |
| Icon monogram, ✕ Close+detach, actions InvokeAction+detach | ✅ |
| Progress from `expire_at` + first_seen; sticky = no bar | ✅ |
| Critical border/text `#f38ba8` | ✅ |
| Low/Normal → info blue progress | ✅ (mapping table) |
| Success green skipped (no field) | ✅ |
| No outer stack border | ✅ |
| 100ms tick via `cx.spawn` | ✅ |
| Enter/exit anim deferred | ✅ honest |
| `cargo build --release -p chronos` | ✅ forced rebuild |
| Commits by minion | ❌ uncommitted — Architect committed |
| Live grim | partial: shell + notify-send after start; no grim paths in report |

## Commit

```
813b3aa notifications : ephemeral toast stack per mockup (T124)
```

Only `notifications/{mod,view}.rs`. History rustfmt noise and untracked
`crates/app/src/toast/` **not** included.

## Caveats

1. Live visual acceptance is on the user (cards, progress, critical, dismiss).
2. Enter/exit animation + critical pulse shadow = debt (brief allowed).
3. Progress uses `first_seen` at first **render**, not daemon create time —
   bar may start mid-width if open delayed; acceptable.
4. Progress fill width hardcodes `340.0` — keep in sync with `POPUP_WIDTH`.
5. Leftover WIP `crates/app/src/toast/` untracked — do not wire until design
   matches or delete; not part of T124 accept.
6. Infinite tick while window open is fine; window closes when empty → entity drop ends task.

## Accept checklist

- [x] Independent toast cards, mockup geometry  
- [x] Close/actions detach  
- [x] Progress when TTL known  
- [x] Critical distinct  
- [x] History not broken (separate renderer)  
- [ ] User grim / click-through (recommended)  
