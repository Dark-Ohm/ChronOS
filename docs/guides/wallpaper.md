# Wallpaper integration

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  ChronOS shell                                           │
│                                                          │
│  wallpaper_ctl  ──► WallpaperSubscriber  ──► awww CLI   │
│  (next/set/scan)    (reactive Mutable)     (daemon)      │
│                                                          │
│  IPC: wallpaper-next / wallpaper-set: / wallpaper-gallery│
│                                                          │
│  side_panel_right  ──► wallpaper card                   │
│  (System tab)         (Next, Open gallery, CTA)          │
└──────────────────────────────────────────────────────────┘
            │                              │
            ▼                              ▼
     awww daemon                   waytrogen (optional)
     (wallpaper setter)            (gallery GUI)
```

## What ChronOS owns

- **Engine:** `crates/services/src/wallpaper/` — `awww` CLI wrapper, reactive `WallpaperState`, `dispatch(WallpaperCommand)`
- **Hotpath:** `wallpaper_ctl::next()` — folder cycle `~/Pictures/Wallpapers` round-robin
- **IPC:** Unix socket commands for external triggers (Hyprland keybinds, scripts)
- **UI card:** System tab wallpaper card with Next, Open gallery, install CTA

## What waytrogen owns

- **Gallery GUI:** Full wallpaper browser — recursive library, GIF/video, transitions, per-monitor settings, JSON state persistence
- **External scripts:** waytrogen supports `--external-script` for custom workflows
- **Multi-backend management:** awww, hyprpaper, swaybg, mpvpaper, gslapper

## Install companion

waytrogen is optional. ChronOS runs without it.

```bash
# Arch AUR
yay -S waytrogen
```

When installed, the wallpaper card in the System tab shows "Open gallery" which
launches waytrogen's full GUI. When not installed, it shows an install hint.

## IPC table

| Payload | Action |
|---|---|
| `wallpaper-next` | Cycle to next wallpaper in `~/Pictures/Wallpapers` |
| `wallpaper-set:<abs-path>` | Set wallpaper to specific file |
| `wallpaper-gallery` | Open waytrogen gallery GUI; auto-resync on close |
| `wallpaper-refresh` | Force re-query `awww query` into service state |

Example Hyprland binds:

```lua
-- Next wallpaper (hotpath, no GUI)
bind = SUPER, W, exec, python3 -c "import socket; s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM); s.connect('/run/user/1000/chronos.sock'); s.sendall(b'wallpaper-next')"

-- Open gallery (waytrogen)
bind = SUPER SHIFT, W, exec, python3 -c "import socket; s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM); s.connect('/run/user/1000/chronos.sock'); s.sendall(b'wallpaper-gallery')"

-- Force resync (after external script changes wallpaper)
bind = SUPER CTRL, W, exec, python3 -c "import socket; s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM); s.connect('/run/user/1000/chronos.sock'); s.sendall(b'wallpaper-refresh')"
```

## External-script bridge

waytrogen supports `--external-script` for custom wallpaper-change hooks. If you
use this feature, your script can call `wallpaper-refresh` or `wallpaper-set:` to
keep ChronOS state aligned:

```bash
#!/bin/bash
# waytrogen external script example
# Called by waytrogen after each wallpaper change

SOCKET="${XDG_RUNTIME_DIR:-/tmp}/chronos-$(id -u).sock"
echo "wallpaper-refresh" | socat - UNIX-CONNECT:"$SOCKET" 2>/dev/null
```

This is optional — ChronOS auto-resyncs when you close the gallery.

## Smoke

```bash
# 1. waytrogen installed → Open gallery shows their full GUI
# 2. Set wallpaper in waytrogen → after close, ChronOS state matches awww
# 3. wallpaper-next cycles without opening GUI
# 4. waytrogen removed from PATH → UI shows install CTA
# 5. grim: panel card + waytrogen window
```
