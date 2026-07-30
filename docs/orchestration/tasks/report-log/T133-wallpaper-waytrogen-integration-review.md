# T133 review — ACCEPTED WITH CAVEATS

**Date:** 2026-07-25

## Verified in tree

| Claim | Evidence |
|---|---|
| waytrogen detect + open | `wallpaper_ctl::{waytrogen_available, open_waytrogen_gallery}` |
| `refresh()` on service | `WallpaperSubscriber::refresh` |
| IPC gallery/refresh | messages + ipc/mod handlers |
| System tab card | `wallpaper_card.rs` + wire in `view.rs` System tab |
| Docs | `docs/wallpaper.md` |
| Unit tests wallpaper IPC | 15 pass filtered |
| No GPUI gallery rewrite | yes |
| Missing companion CTA | `"waytrogen not found — yay -S waytrogen"` |

## Errata applied on review

1. Button label **Open gallery** → **Open waytrogen** (name their product).  
2. Card click had **no resync** after open — added delayed `refresh()` like IPC arm.

## Report vs code nits

| Report | Reality |
|---|---|
| Async wait-for-exit resync | **Delayed 3s refresh** (Send-safe; child-wait dropped earlier) |
| Button “Open gallery” | now **Open waytrogen** |
| “DONE” / live smoke checked | **manual smoke still open** |

## Live smoke (PENDING on this host)

`waytrogen` **not on PATH** here — cannot verify their GUI opens. You must:

```text
Super+G → System → Open waytrogen  (or wallpaper-gallery IPC)
next cycles ~/Pictures/Wallpapers
uninstall/hide waytrogen → CTA text
```

## Verdict

**ACCEPTED WITH CAVEATS** — integration shape correct; commit code; live grim/waytrogen on your session for full close.
