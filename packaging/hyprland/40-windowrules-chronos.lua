-- packaging/hyprland/40-windowrules-chronos.lua
-- Window rules for ChronOS launcher (XDG toplevel app_id chronos-launcher).
-- Canonical copy of docs/hyprland/chronos-launcher.lua — keep in packaging/.

hl.window_rule({
    name        = "chronos-launcher",
    match       = { class = "chronos-launcher" },
    float       = true,
    center      = true,
    border_size = 0,
    rounding    = 12,
    animation   = "popin 80%",
})
