-- packaging/hyprland/50-binds-chronos.lua
-- Product binds for ChronOS IPC only. User/app kitchen binds live elsewhere.
-- Requires `chronos-ipc` on PATH (packaging/hyprland/chronos-ipc).

local mainMod = "SUPER"

local function ipc(cmd)
    return hl.dsp.exec_cmd("chronos-ipc " .. cmd)
end

-- OSD launcher (not the Start menu).
hl.bind(mainMod .. " + SPACE", ipc("toggle-launcher"))
-- Classic Start: tap Super. `release` so Super+R / Super+A still work.
hl.bind(mainMod .. " + Super_L", ipc("toggle-start-menu"), { release = true })
hl.bind(mainMod .. " + Super_R", ipc("toggle-start-menu"), { release = true })
hl.bind(mainMod .. " + A", ipc("toggle-side-panel-left"))
hl.bind(mainMod .. " + G", ipc("toggle-side-panel-right"))
hl.bind(mainMod .. " + SHIFT + T", ipc("toggle-theme"))
hl.bind(mainMod .. " + SHIFT + E", ipc("toggle-edit-mode"))
