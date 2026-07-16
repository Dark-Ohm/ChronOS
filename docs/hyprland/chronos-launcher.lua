-- ======================================================================
-- chronos-launcher.lua
-- ----------------------------------------------------------------------
-- Hyprland 0.55+ (Lua config) window rules for the ChronOS launcher.
--
-- The launcher is a normal XDG toplevel opened by `chronos` with
-- `xdg_toplevel.set_app_id("chronos-launcher")`, which Hyprland exposes
-- as the window's `initialClass`. The rules below turn it into the
-- overlay-like surface we used to fake with `zwlr_layer_shell_v1`
-- (Layer::Overlay + KeyboardInteractivity::OnDemand) — without any of
-- that protocol's focus pitfalls:
--
--   * `KeyboardInteractivity::Exclusive` froze the Hyprland/Niri input
--     stack because the exclusive layer-surface never acks keyboard focus.
--   * `OnDemand` opened the surface but never granted focus by itself
--     (layer-shell surfaces don't participate in `xdg_activation_v1`),
--     so the user had to click before typing.
--
-- A regular toplevel sidesteps both: Hyprland's normal focus policy
-- drives the focus, and the rules below make it float and dim everything
-- behind it. Focus-lost-close is handled by `observe_window_activation`
-- in `crates/app/src/launcher/mod.rs::open()` — the launcher closes when
-- it loses focus (click away, workspace switch), matching rofi/fuzzel UX.
-- No `stay_focused` or `pin` windowrules; no per-frame focus re-assert.
--
-- USAGE — from your `~/.config/hypr/hyprland.lua`:
--
--   dofile(os.getenv("HOME") .. "/projects/chronos-ecosystem/ChronOS/docs/hyprland/chronos-launcher.lua")
--
-- or copy this file into your Hyprland config tree and `dofile` it from
-- there. The `hl` global is provided by Hyprland's Lua runtime — no
-- import needed.
-- ======================================================================

-- Main rule: every ChronOS launcher window.
-- ID = "chronos-launcher" matches the Rust app_id set in
-- `crates/app/src/launcher/mod.rs::window_options` (→ xdg_toplevel
-- `set_app_id` → Hyprland `initialClass`).
hl.window_rule({
    name        = "chronos-launcher",
    match       = { class = "chronos-launcher" },

    -- Overlay feel, but via XDG toplevel rules (no layer-shell).
    float       = true,    -- float instead of tiling
    center      = true,    -- center on the current monitor
    -- NOTE: `stay_focused` intentionally REMOVED — incompatible with
    -- focus-lost-close. The launcher closes when the user clicks away
    -- or switches focus, matching rofi/fuzzel UX.
    -- NOTE: `pin` intentionally REMOVED — useless with focus-lost-close;
    -- switching workspaces loses focus and closes the launcher anyway.

    -- Visual: no compositor border + prominent rounding. Hyprland 0.55 Lua
    -- API has no `noborder` field; `border_size = 0` does the same thing.
    border_size = 0,
    rounding    = 12,

    -- Pop-in animation for a rofi-like appear. The optional percentage is
    -- the minimum scale before the animation finishes (see Hyprland wiki
    -- /Configuring/Basics/Window-Rules, effect `animation`).
    animation   = "popin 80%",
})

-- Optional: dim the rest of the workspace while the launcher is open.
-- Uncomment if you want a modal, rofi-like backdrop. Disabled by default
-- because it interacts with whatever else is on screen and some users
-- prefer the launcher to feel weightless (no dim behind it).
--
-- hl.window_rule({
--     name       = "chronos-launcher-dim",
--     match      = { class = "chronos-launcher" },
--     dim_around = true,
-- })