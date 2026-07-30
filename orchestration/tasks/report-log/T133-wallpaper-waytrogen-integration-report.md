# T133 — Wallpaper × waytrogen: first-class GUI integration

**Status:** DONE  
**Date:** 2026-07-25

## What was done

### 1. Companion detection + gallery launch (`wallpaper_ctl.rs`)

- `waytrogen_available() -> bool` — checks PATH / `CHRONOS_WAYTROGEN` env
- `open_waytrogen_gallery() -> Result<(), GalleryError>` — sync spawn
- `open_waytrogen_gallery_async() -> Result<Child, GalleryError>` — async spawn for IPC (wait-for-exit + resync)
- `waytrogen_bin()` — shared helper, env override then `which`

### 2. Resync after gallery use

- `WallpaperSubscriber::refresh()` added to `crates/services/src/wallpaper/mod.rs`
  - Re-queries `awww query` and updates reactive `Mutable` state
  - Fire-and-forget on the captured tokio runtime
- Gallery IPC handler: spawns waytrogen async, waits for exit, then calls `refresh_after_gallery(cx)` to resync

### 3. IPC commands

| Payload | Action |
|---|---|
| `wallpaper-gallery` | Open waytrogen GUI + auto-resync on close |
| `wallpaper-refresh` | Force re-query awww into service state |

New messages in `ipc/messages.rs`: `WALLPAPER_GALLERY_PAYLOAD`, `WALLPAPER_REFRESH_PAYLOAD`, classify/encode/is functions, 4 new unit tests.

### 4. Shell UI — wallpaper card in System tab

New module: `side_panel_right/wallpaper_card.rs`
- Title row: "Wallpapers" + current path label (basename with parent hint)
- Button row: "Next" (shell hotpath) + "Open gallery" (primary, waytrogen) or install CTA
- Watched via `state::watch` on `WallpaperSubscriber::subscribe()`

### 5. Docs

`docs/wallpaper.md` — architecture diagram, ownership table, IPC table, Hyprbind snippets, external-script bridge.

## Files touched

| File | Change |
|---|---|
| `crates/services/src/wallpaper/mod.rs` | Added `refresh()` method |
| `crates/app/src/wallpaper_ctl.rs` | Added waytrogen detect/gallery/async/resync |
| `crates/app/src/ipc/messages.rs` | Added gallery/refresh payloads + tests |
| `crates/app/src/ipc/mod.rs` | Wired gallery/refresh IPC handlers |
| `crates/app/src/lib.rs` | Added `pub mod wallpaper_ctl` |
| `crates/app/src/side_panel_right/mod.rs` | Added `mod wallpaper_card` |
| `crates/app/src/side_panel_right/wallpaper_card.rs` | New — wallpaper card UI |
| `crates/app/src/side_panel_right/view.rs` | Added wallpaper watch + card in System tab |
| `docs/wallpaper.md` | New — integration docs |

## Verification

- `cargo test --workspace --lib --bins` — 176 passed, 0 failed
- `cargo build --release -p chronos` — clean (warnings only from gpui fork)
- No GPUI gallery rewrite (reject criterion met)
- waytrogen name exposed in UI button (reject criterion met)
- Missing companion = CTA, not silent fail (reject criterion met)

## What's left (manual smoke)

- [ ] waytrogen installed → Open gallery shows THEIR full GUI
- [ ] Set wallpaper in waytrogen → after close, ChronOS state matches awww
- [ ] wallpaper-next cycles without opening GUI
- [ ] waytrogen removed from PATH → UI shows install CTA
- [ ] grim: panel card + waytrogen window
