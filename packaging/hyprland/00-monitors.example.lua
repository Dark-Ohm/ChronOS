-- packaging/hyprland/00-monitors.example.lua
-- Copy to your tree as 00-monitors.lua and edit outputs to match `hyprctl monitors`.
-- This example is dual-head: primary left 2560x1440, secondary right 1920x1200.

hl.monitor({
    output   = "DP-1",
    mode     = "2560x1440@144.00",
    position = "0x0",
    scale    = 1,
})
hl.monitor({
    output   = "HDMI-A-1",
    mode     = "1920x1200@59.95",
    position = "2560x0",
    scale    = 1,
})
