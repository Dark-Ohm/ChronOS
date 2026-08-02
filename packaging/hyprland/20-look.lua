-- packaging/hyprland/20-look.lua
-- Gaps / border / animations. Keep lean; theme chrome is ChronOS, not Hyprland.

hl.config({
    general = {
        gaps_in  = 3,
        gaps_out = 8,
        border_size = 2,
        layout = "dwindle",
    },
    decoration = {
        rounding = 10,
    },
    animations = { enabled = true },
})

hl.config({
    misc = {
        force_default_wallpaper = -1,
    },
})
