# ChronOS Hyprland ship profile

Modular **Lua** config pieces for Hyprland 0.55+ / 0.56. Goal: something we can
ship next to the `chronos` package — not a developer's `~/.config/hypr/hyprland.lua`.

See also `docs/PRODUCT.md` (Agent Shell DE; shell ships with Hyprland dots).

## Layout

| file | role |
|---|---|
| `00-monitors.example.lua` | copy → `00-monitors.lua`, edit outputs |
| `10-input.lua` | keyboard / mouse |
| `20-look.lua` | gaps / border / animations |
| `30-autostart.lua` | start ChronOS (`chronos-start` or `chronos` on PATH) |
| `40-windowrules-chronos.lua` | launcher float/center |
| `50-binds-chronos.lua` | SUPER+L/A/G + theme/edit via `chronos-ipc` |
| `chronos-ipc` | UNIX-socket CLI for the shell |
| `hyprland.ship.lua` | optional single entry that dofiles the modules |

Kitchen binds (workspaces, volume, apps) are **not** in this tree — user owns them.

## Dev machine

- Build: `chronos-rebuild` → `target/release/chronos`
- Symlink (once): `ln -sfn "$REPO/target/release/chronos" ~/.local/bin/chronos`
- Autostart: prefer `chronos-start` (always points at that release binary after rebuild)
- IPC: `install -m755 packaging/hyprland/chronos-ipc ~/.local/bin/`

## Install sketch (user)

```bash
install -Dm755 packaging/hyprland/chronos-ipc ~/.local/bin/chronos-ipc
mkdir -p ~/.config/hypr/chronos
cp packaging/hyprland/*.lua ~/.config/hypr/chronos/
cp packaging/hyprland/00-monitors.example.lua ~/.config/hypr/chronos/00-monitors.lua
# edit 00-monitors.lua
# from hyprland.lua:
#   dofile(os.getenv("HOME") .. "/.config/hypr/chronos/hyprland.ship.lua")
```
