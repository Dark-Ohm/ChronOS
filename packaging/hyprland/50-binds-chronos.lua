-- packaging/hyprland/50-binds-chronos.lua
-- Product binds for ChronOS IPC only. User/app kitchen binds live elsewhere.
-- Requires `chronos-ipc` on PATH (packaging/hyprland/chronos-ipc).

local mainMod = "SUPER"

local function ipc(cmd)
    return hl.dsp.exec_cmd("chronos-ipc " .. cmd)
end

hl.bind(mainMod .. " + L", ipc("toggle-launcher"))
hl.bind(mainMod .. " + A", ipc("toggle-side-panel-left"))
hl.bind(mainMod .. " + G", ipc("toggle-side-panel-right"))
hl.bind(mainMod .. " + SHIFT + T", ipc("toggle-theme"))
hl.bind(mainMod .. " + SHIFT + E", ipc("toggle-edit-mode"))
