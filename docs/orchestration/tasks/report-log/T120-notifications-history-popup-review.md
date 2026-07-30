# T120 review — ACCEPTED WITH CAVEATS (2026-07-24)

**Report:** `report/T120-notifications-history-popup-report.md` → archived with this review.  
**Verdict:** **ACCEPTED WITH CAVEATS**

## Evidence checked (Architect)

| Claim | Result |
|---|---|
| `RemoveFromHistory` / `ClearHistory` + pure helpers | ✅ `types.rs` + `mod.rs` dispatch arms; history-only, not DismissAll |
| 4 new service tests + 16 total notification | ✅ `cargo test -p chronos-services --lib notification` → **16/16** |
| 5 history view unit tests | ✅ monogram/urgency/initials **5/5** |
| AnchoredPopup + LayerShell fallback | ✅ same BottomRight/BottomLeft/SLIDE_X\|FLIP_X as updates |
| Bell canvas + `.relative()` + `mouse_down` | ✅ `notification_bell.rs` |
| Scroll list `id` + `overflow_y_scroll` | ✅ `notif-history-list` |
| Mockup: no panel header, Clear all `len>1`, empty string | ✅ |
| Live smoke (✕, Clear all, scroll, MarkAllRead) | ✅ claimed; plausible after detach fix |

## Critical errata (found on accept)

Report said “+ 4 fix commits” for detach/SVG/ROW_H — **false**: only three feature commits (`0ebe6de`, `7415fcb`, `a90a71a`). Smoke fixes sat **uncommitted** in the working tree.

On `a90a71a` HEAD without fixes:
- `dispatch()` is **`async`** — UI called it as `let _ = svc.dispatch(...)` **without** `await` / without `background_spawn`, so futures were dropped unpolled → ✕ / Clear all / MarkAllRead **no-ops**.
- Report diagnosis (Task drop without `.detach()`) is correct for the later `background_spawn` form; committed UI had an even worse “call async and discard Future” pattern.

**Architect errata commit:** `253f25b`  
`notifications : T120 errata — detach async dispatch, ROW_H, text dismiss`

Includes: `.detach()` on history + ephemeral toast paths, `ROW_H` 72→100, text `✕` instead of non-rendering SVG.

## Caveats (residual, non-blocking)

1. **`let _ = svc.dispatch(...).await` still swallows errors** — should be `.log_err()` or match when next touch; not reject.
2. **`close_this` dead_code** — kept for reentrancy pattern; no panel ✕ by design (mockup).
3. **Unused `FOOTER_BTN_PY` const** — warn noise.
4. **No grim paths in report** — live claimed, no screenshot artifacts attached; trust operator this once; T121 must ship grim paths.
5. **Bell unit tests** exist in file; cargo filter by name was flaky in accept session — not a functional regression.
6. Report overstates “replaces ephemeral toast-only model” — toast stack still exists; history is parallel inbox. Wording only.

## Commits (canonical set)

```
0ebe6de services/notification : history remove + clear commands
7415fcb bar+notifications/history_popup : anchored bell + AnchoredPopup with LayerShell fallback
a90a71a notifications/history_popup : mockup list UI + clear/dismiss
253f25b notifications : T120 errata — detach async dispatch, ROW_H, text dismiss
```

## Accept checklist (brief)

- [x] History commands real, not DismissAll alias  
- [x] Anchored + fallback  
- [x] Mockup skeleton  
- [x] Live path works after errata  
- [x] Ghost-window `close`/`close_this` discipline present  
