-- packaging/hyprland/45-surface-effects-chronos.lua
-- T266: compositor blur for ChronOS shell surfaces (OPT-IN).
--
-- The shell NEVER writes or patches your Hyprland config. This module is the
-- opt-in bridge: import it from your hyprland.lua (or the shipped
-- hyprland.ship.lua profile) and the shell's «Blur» toggle in Bar settings
-- reaches it through `hyprctl eval _G.chronos_set_blur_enabled(...)`.
--
-- Import from an existing config:
--   dofile(os.getenv("HOME") .. "/.config/hypr/chronos/45-surface-effects-chronos.lua")
--
-- What it does:
--   * Enables Hyprland's GLOBAL blur (decoration.blur). Measured live on
--     0.56.2 (2026-08-17): with the global switch off, even a correct
--     per-surface blur layer rule renders nothing. Importing this module is
--     the explicit opt-in — global blur is a compositor-wide switch and
--     stays ON while the module is imported (the shell toggle only turns the
--     ChronOS surfaces' blur on/off, not the compositor's).
--   * One named layer rule holding blur for exactly the ChronOS layer
--     surfaces that get the T266 surface alpha (bar, panels, popups, OSD,
--     desktop terminal, tray/dock menus). Disabled by default — the shell
--     toggle enables it.
--   * One inverse `no_blur` window rule for the launcher (an XDG toplevel,
--     not a layer surface) so the user's own global window blur can never
--     smear the launcher while the shell blur is off. Blur-ON for the
--     launcher is NOT controlled here: it only happens if the user has
--     independently enabled Hyprland's global window blur.
--
-- NOTE on re-imports: `hl.layer_rule` is idempotent by rule NAME — re-running
-- this file returns the already-registered rule and cannot change its baked-in
-- options. After updating this file, restart Hyprland for the new rule shape.
--
-- Excluded on purpose: hover-strip namespaces (transparent cursor traps),
-- the popup click catcher, and frame surfaces (chrome, not content).

-- Global blur is required for per-surface layer-rule blur (measured live,
-- Hyprland 0.56.2). Importing this module is the user's explicit opt-in.
hl.config({
    decoration = {
        blur = {
            enabled = true,
            size = 6,
            passes = 2,
        },
    },
})

local namespaces =
    "^(bar|side_panel_left_rail|side_panel_left_content|side_panel_right_rail|side_panel_right_content|volume-popup|osd|notifications|desktop-terminal|tray-menu|dock-menu)$"

_G.chronos_surface_blur_rule = hl.layer_rule({
    name = "chronos-surface-blur",
    match = { namespace = namespaces },
    blur = true,
    blur_popups = true,
    -- NOTE: NO ignore_alpha. Measured live on Hyprland 0.56.2 (2026-08-17):
    -- `ignore_alpha` in a layer rule silently disables the blur for the
    -- matched surface (zero pixels change on toggle). The option's 0.56
    -- semantics differ from the 0.5x docs; dropping it restores blur.
})
_G.chronos_surface_blur_rule:set_enabled(false)

-- Launcher is an XDG toplevel. This rule only guarantees blur-OFF (inverse
-- of the layer rule); it cannot enable window blur on a toplevel.
_G.chronos_launcher_no_blur_rule = hl.window_rule({
    name = "chronos-launcher-no-blur",
    match = { class = "^chronos-launcher$" },
    no_blur = true,
})

_G.chronos_set_blur_enabled = function(enabled)
    _G.chronos_surface_blur_rule:set_enabled(enabled)
    _G.chronos_launcher_no_blur_rule:set_enabled(not enabled)
end
