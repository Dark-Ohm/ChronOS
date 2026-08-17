-- packaging/hyprland/hyprland.ship.lua
-- Entry point for a *shipped* ChronOS Hyprland profile (Hyprland 0.55+ Lua).
--
-- Install (example):
--   mkdir -p ~/.config/hypr/chronos
--   cp -a packaging/hyprland/* ~/.config/hypr/chronos/
--   install -m755 packaging/hyprland/chronos-ipc ~/.local/bin/
--   # Point Hyprland at this file, or dofile modules from your hyprland.lua:
--
--   local CHRONOS_HYPR = os.getenv("HOME") .. "/.config/hypr/chronos"
--   dofile(CHRONOS_HYPR .. "/00-monitors.lua")  -- copy from .example first
--   dofile(CHRONOS_HYPR .. "/10-input.lua")
--   dofile(CHRONOS_HYPR .. "/20-look.lua")
--   dofile(CHRONOS_HYPR .. "/30-autostart.lua")
--   dofile(CHRONOS_HYPR .. "/40-windowrules-chronos.lua")
--   dofile(CHRONOS_HYPR .. "/50-binds-chronos.lua")
--
-- Monitors MUST be customized — 00-monitors.example.lua is not loaded by default.

local here = debug.getinfo(1, "S").source:sub(2):match("(.*/)") or "./"

-- Optional: only load monitors if the user created 00-monitors.lua
do
    local mon = here .. "00-monitors.lua"
    local f = io.open(mon, "r")
    if f then
        f:close()
        dofile(mon)
    end
end

dofile(here .. "10-input.lua")
dofile(here .. "20-look.lua")
dofile(here .. "30-autostart.lua")
dofile(here .. "40-windowrules-chronos.lua")
-- T266: optional compositor blur bridge (45-surface-effects-chronos.lua).
-- Loaded by the shipped profile because it is ChronOS-owned; an existing
-- user config must dofile it manually (the shell never edits that config).
dofile(here .. "45-surface-effects-chronos.lua")
dofile(here .. "50-binds-chronos.lua")
