-- packaging/hyprland/10-input.lua
-- Keyboard layouts + mouse. Safe defaults for multi-layout machines.

hl.config({
    input = {
        kb_layout  = "us,ru",
        kb_variant = ",",
        kb_model   = "",
        kb_options = "grp:alt_shift_toggle",
        follow_mouse = 1,
        sensitivity = 0,
        touchpad = { natural_scroll = false },
    },
})
