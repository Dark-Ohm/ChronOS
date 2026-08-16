# T266 Surface Transparency and Blur Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Добавить одну пользовательскую ось прозрачности для внешних поверхностей ChronOS и честный Hyprland-only переключатель compositor blur, сохранив пиксельно неизменный дефолт.

**Architecture:** `theme.toml` хранит запрошенную alpha и blur-флаг; `theme_config` накладывает их на эффективный `Theme`, а один helper умножает исходную alpha только у корневых surface-заливок. Hyprland-модуль владеет named layer-rule handle и экспортирует глобальный setter; compositor service лишь проверяет и вызывает этот API через `hyprctl eval`, никогда не редактируя пользовательский конфиг.

**Tech Stack:** Rust 2024, gpui-ce `Hsla::opacity`, TOML/serde, GPUI globals/hot reload, Hyprland 0.56.2 Lua config, `hyprctl eval`, `grim`.

## Global Constraints

- Канон требований: `docs/orchestration/tasks/active/T266-surface-transparency-and-blur.md` at `ea8f1bb` or newer.
- `surface_alpha = 1.0` and blur off must reproduce the current pixels; existing per-color alpha is multiplied, not replaced.
- T267 borders and all unrelated `opacity(...)` calls remain unchanged.
- One alpha slider controls outer window surfaces; nested cards, hover washes, icons, accents and progress tracks are not globally faded.
- No `paint_blur`: it cannot see the wallpaper surface.
- No Niri/ext-background-effect work in T266.
- ChronOS never writes or patches the user's Hyprland config. It ships a module and documentation; the user imports it.
- Hover strips remain fully transparent cursor traps and are excluded from blur namespace matching.
- Live verification uses a release binary, both ChronOS themes, dark/light wallpapers, blur on/off, and no file drag from Chronos-FM.
- T263 and T266 both modify `crates/app/src/theme_config.rs`. Do not create the
  T266 worktree from the current dirty master: first land the T263 commit, then
  create a sibling worktree from that HEAD. Integration order is T263 → T266.
  This preserves the existing T263/T264/T265 WIP and keeps `../Source` path
  dependencies resolving.

---

### Task 0: Prove the Lua runtime bridge before shell code

**Files:**
- Evidence only: `/tmp/t266-lua-api-before.png`
- Evidence only: `/tmp/t266-lua-api-enabled.png`
- Final evidence destination: `docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md`

**Interfaces:**
- Produces: proven contract `_G.chronos_set_blur_enabled(enabled: bool)` may retain and toggle an `hl.layer_rule` handle across separate `hyprctl eval` calls.

- [x] **Step 1: Start the existing release binary without touching config**

```bash
CHRONOS_THEME=Default RUST_LOG=warn ./target/release/chronos
```

- [x] **Step 2: Capture the unchanged frame**

```bash
grim -g '0,0 2560x480' /tmp/t266-lua-api-before.png
```

- [x] **Step 3: Create a temporary disabled rule and persistent global setter**

```bash
hyprctl eval '_G.chronos_t266_probe_rule = hl.layer_rule({ name = "chronos-t266-probe", match = { namespace = "^bar$" }, dim_around = true }); _G.chronos_t266_probe_rule:set_enabled(false); _G.chronos_t266_probe_set = function(enabled) _G.chronos_t266_probe_rule:set_enabled(enabled) end'
```

Expected: `ok`.

- [x] **Step 4: Call the setter from a separate eval and capture visible evidence**

```bash
hyprctl eval '_G.chronos_t266_probe_set(true)'
grim -g '0,0 2560x480' /tmp/t266-lua-api-enabled.png
hyprctl eval '_G.chronos_t266_probe_set(false)'
```

Observed on Hyprland 0.56.2: both evals returned `ok`; the enabled frame visibly dims the desktop around `bar`; the rule was disabled after capture. This proves global persistence, `set_enabled`, and runtime invocation.

- [x] **Step 5: Capture the pre-change opaque baseline with an empty config root**

```bash
XDG_CONFIG_HOME=/tmp/t266-default-config-before ./target/release/chronos
grim -g '0,0 2560x480' /tmp/t266-default-before-bar.png
grim -g '0,0 700x1440' /tmp/t266-default-before-left.png
grim -g '1860,0 700x1440' /tmp/t266-default-before-right.png
```

The baseline uses no user `theme.toml`; left/right panels were opened through IPC after waiting more than two seconds for their animations. These exact geometries and the same empty-config contract are reused after implementation.

### Task 1: Add effective surface tokens and config schema

**Files:**
- Create: `crates/ui/src/theme/surface.rs`
- Modify: `crates/ui/src/theme/mod.rs`
- Modify: `crates/app/src/theme_config.rs`

Do not edit `crates/ui/src/theme/base16.rs` or `schemes.rs` in this task:
Base16 and Light build from `Theme::default()` and inherit the new opaque token.
Per-scheme changes belong only to Task 5 after live calibration.

**Interfaces:**
- Produces: `SurfaceTokens { alpha, min_alpha, blur_enabled }`.
- Produces: `Theme::surface_color(Hsla) -> Hsla`, which calls `color.opacity(self.surface.alpha)`.
- Produces: `ThemeConfig { scheme, surface_alpha, blur_enabled }` with defaults `None/false`.
- Produces: `persist_surface_settings(alpha: f32, blur_enabled: bool)` and `effective_surface_alpha(requested, min)`.
- Preserves: T263's gpui-component popup mapping, but applies the effective
  surface alpha to `popover` because tray and dock menus are rendered by
  `PopupMenu`, not by their host view roots.

- [ ] **Step 1: Add failing config/default tests**

```rust
#[test]
fn empty_config_preserves_opaque_blurless_default() {
    let cfg: ThemeConfig = toml::from_str("").unwrap();
    let theme = resolve_theme(None, &cfg);
    assert_eq!(theme.surface.alpha, 1.0);
    assert!(!theme.surface.blur_enabled);
}

#[test]
fn env_scheme_still_applies_file_surface_settings() {
    let cfg = ThemeConfig {
        surface_alpha: Some(0.72),
        blur_enabled: true,
        ..Default::default()
    };
    let theme = resolve_theme(Some("Default".into()), &cfg);
    assert_eq!(theme.surface.alpha, 0.72_f32.max(theme.surface.min_alpha));
    assert!(theme.surface.blur_enabled);
}

#[test]
fn surface_color_multiplies_existing_alpha() {
    let mut theme = Theme::default();
    theme.surface.alpha = 0.5;
    let color = parse_hex("1e1e2ecc").unwrap();
    assert!((theme.surface_color(color).a - 0.4).abs() < 1e-6);
}
```

- [ ] **Step 2: Run tests and verify the new API is absent**

Run: `cargo test -p chronos-ui theme::surface && cargo test -p chronos theme_config`

Expected: compile failure for missing `surface`/`surface_color` fields.

- [ ] **Step 3: Implement the minimal theme token**

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceTokens {
    pub alpha: f32,
    pub min_alpha: f32,
    pub blur_enabled: bool,
}

impl SurfaceTokens {
    pub const fn opaque(min_alpha: f32) -> Self {
        Self { alpha: 1.0, min_alpha, blur_enabled: false }
    }
}

impl Theme {
    pub fn surface_color(&self, color: Hsla) -> Hsla {
        color.opacity(self.surface.alpha)
    }
}
```

Task 1 commits a conservative temporary `min_alpha = 1.0`, so no uncalibrated
transparency ships between commits. Task 5 changes it to `0.0` only as an
uncommitted worktree probe, measures both schemes, and commits the measured
floors. No commit may contain the calibration value `0.0`.

- [ ] **Step 4: Extend config resolution on every scheme-selection path**

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ThemeConfig {
    pub scheme: Option<String>,
    pub surface_alpha: Option<f32>,
    #[serde(default)]
    pub blur_enabled: bool,
}

fn apply_surface_config(mut theme: Theme, cfg: &ThemeConfig) -> Theme {
    let requested = cfg.surface_alpha.unwrap_or(1.0).clamp(0.0, 1.0);
    theme.surface.alpha = requested.max(theme.surface.min_alpha);
    theme.surface.blur_enabled = cfg.blur_enabled;
    theme
}
```

`ThemeConfig` currently derives `Eq`; remove only `Eq` because `f32` cannot
implement it. Every existing `ThemeConfig { scheme: ... }` test literal gains
`..Default::default()`.

Refactor resolution into two phases: first choose the scheme from env → file →
default, then call `apply_surface_config(theme, cfg)` exactly once. There must be
no early return before that second phase. Cover all three scheme sources in
unit tests, including `env_value = Some("Default")` with file alpha.

`toggle()` must not call `Theme::select_scheme` and install that raw value. It
loads the current config, chooses the next scheme, overlays the same surface
settings, then installs/syncs the resulting theme. The env remains authoritative
on the next normal `apply`, as today. Add a test that the toggle's selected
scheme retains non-default alpha/blur settings rather than resetting them.

Replace typed whole-struct serialization with one TOML-document merge helper.
Both `persist_scheme` and `persist_surface_settings` update only their owned
keys, preserving each other's fields and unknown keys. Add round-trip tests for
`scheme`, `surface_alpha`, `blur_enabled`, and an unknown sentinel key.

In `sync_gpui_component_theme`, change the T263 mapping to:

```rust
gpui_theme.popover = shell.surface_color(shell.bg.elevated);
```

Update T263 assertions from `gt.popover == shell.bg.elevated` to the effective
surface color, while keeping alpha `1.0` pixel-identical. Do not edit
`tray_menu/view.rs` or `dock/context_menu.rs`; those files do not paint the menu
card.

- [ ] **Step 5: Run focused tests**

Run: `cargo test -p chronos-ui theme::surface && cargo test -p chronos theme_config`

Expected: all pass; empty config resolves to alpha `1.0`, blur `false`.

- [ ] **Step 6: Commit**

```bash
git add crates/ui/src/theme/surface.rs crates/ui/src/theme/mod.rs crates/app/src/theme_config.rs
git commit -m "feat(theme): add configurable surface effects"
```

### Task 2: Apply alpha only to outer surface fills

**Files:**
- Modify: `crates/app/src/bar/mod.rs`
- Modify: `crates/app/src/side_panel_left/panel.rs`
- Modify: `crates/app/src/side_panel_right/view.rs`
- Modify: `crates/app/src/side_panel_right/rail.rs`
- Modify: `crates/app/src/volume_popup/view.rs`
- Modify: `crates/app/src/system_popup/view.rs`
- Modify: `crates/app/src/updates_popup/view.rs`
- Modify: `crates/app/src/notifications/view.rs`
- Modify: `crates/app/src/notifications/history_popup/view.rs`
- Modify: `crates/app/src/osd/view.rs`
- Modify: `crates/app/src/launcher/view.rs`
- Modify: `crates/app/src/desktop_terminal/view.rs`

**Interfaces:**
- Consumes: `Theme::surface_color` from Task 1.
- Produces: every named visible plate uses the effective alpha exactly once.
  Do not add a second alpha wrapper around the whole panel/window; the existing
  body/content/rail structure remains otherwise unchanged.

- [ ] **Step 1: Use this closed call-site inventory — no regex-driven expansion**

Apply `theme.surface_color(...)` only at these roots (line numbers are the
pre-T266 anchors; element ids/function names are authoritative after T263):

- `bar/mod.rs:110`, `render`: bar root `.bg(theme.bg.tertiary)`.
- `side_panel_left/panel.rs:463`, `main-content`: main thread plate.
- `side_panel_left/panel.rs:511` and `:814`,
  `sessions-sidebar-{collapsed,expanded}`: the two mutually exclusive sidebar
  roots. Do not touch `agent-dropdown` (`:111`), `thread-context-menu` (`:727`),
  or transparent `resize-handle` (`:478`).
- `side_panel_right/view.rs:684`, `side-panel-body`: chrome only while content
  is open; `:700`, `side-panel-content-column`: content plate.
- `side_panel_right/rail.rs:199`, `render_rail`: rail chrome. Keep
  `surfaces::card`, `surfaces::well`, editor buffers, and all nested tab cards
  opaque; do not move alpha into `surfaces.rs` helpers.
- `volume_popup/view.rs:144` and `system_popup/view.rs:89`: card roots; wrap the
  existing `bg.alpha(0.82)` so `opacity` multiplies 0.82.
- `updates_popup/view.rs:399`, `notifications/history_popup/view.rs:175`,
  `osd/view.rs:134`, and `desktop_terminal/view.rs:468`: their outer card/root.
  Leave desktop-terminal title strip `:490` unchanged.
- `notifications/view.rs:256`, `render_toast_card`: toast card only. Leave icon,
  progress, action-button, and legacy/history helper fills untouched.
- `launcher/view.rs:202`, `render_card`: fade the `bg.primary` card exactly once.
  Do not fade outer window fill `render:160` (`bg.tertiary`) or footer/key/icon
  wells; the window is card-sized, so fading both would compound alpha.
- tray and dock: no view call-site. Their sole menu plate is
  `sync_gpui_component_theme`'s `gpui_theme.popover`, handled in Task 1.

`project-popup` is outside T266's surface inventory. Do not edit
`project_switcher/view.rs` and remove `project-popup` from the Lua namespace
regex in Task 4.

Representative expressions:

```rust
// bar root
.bg(theme.surface_color(theme.bg.tertiary))

// popup that already uses 0.82 alpha: preserve it multiplicatively
let bg = theme.surface_color(theme.bg.primary.alpha(0.82));
```

Do not change nested cards, buttons, icon wells, selection, hover or progress fills.

- [ ] **Step 2: Add pure regression tests where roots already expose color helpers**

```rust
#[test]
fn opaque_surface_keeps_existing_popup_alpha() {
    let theme = Theme::default();
    let original = theme.bg.primary.alpha(0.82);
    assert_eq!(theme.surface_color(original), original);
}
```

- [ ] **Step 3: Replace only the inventoried root fills**

Right-panel `surfaces::chrome/content/card/well` retain their light/dark role
mapping; only the three named body/content/rail call-sites receive alpha.
Hover strips and both ghost handles remain untouched and transparent.

- [ ] **Step 4: Verify exact scope**

Run:

```bash
git diff --check
rg -n 'surface_color' crates/app/src
rg -n 'border_color\(theme\.border\.subtle\)' crates/app/src/{bar/mod.rs,side_panel_left/panel.rs,side_panel_right/view.rs}
```

Expected: exactly the named roots above—three plates on each panel, one on each
other owned surface, and gpui-component popover from Task 1; T267 borders
unchanged.

- [ ] **Step 5: Check and test**

Run: `cargo check -p chronos && cargo test -p chronos`

Expected: success, existing warnings only.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/bar/mod.rs crates/app/src/side_panel_left/panel.rs crates/app/src/side_panel_right/view.rs crates/app/src/side_panel_right/rail.rs crates/app/src/volume_popup/view.rs crates/app/src/system_popup/view.rs crates/app/src/updates_popup/view.rs crates/app/src/notifications/view.rs crates/app/src/notifications/history_popup/view.rs crates/app/src/osd/view.rs crates/app/src/launcher/view.rs crates/app/src/desktop_terminal/view.rs
git commit -m "feat(ui): apply surface alpha across shell chrome"
```

### Task 3: Add the live alpha slider

**Files:**
- Modify: `crates/app/src/side_panel_right/tab/bar_settings.rs`
- Modify: `crates/app/src/theme_config.rs`

**Interfaces:**
- Consumes: existing `slider_control`, `slider_frac`, theme persistence from Task 1.
- Produces: `SurfaceAlphaSliderDrag` and live persistence/application.

- [ ] **Step 1: Add failing state and persistence tests**

```rust
#[test]
fn slider_fraction_maps_to_theme_floor_and_one() {
    assert_eq!(alpha_from_frac(0.0, 0.62), 0.62);
    assert_eq!(alpha_from_frac(1.0, 0.62), 1.0);
}
```

- [ ] **Step 2: Add a third unique drag marker and reuse the existing control**

```rust
pub struct SurfaceAlphaSliderDrag;

fn alpha_from_frac(frac: f32, floor: f32) -> f32 {
    floor + frac.clamp(0.0, 1.0) * (1.0 - floor)
}
```

Use the same `slider_control(...)` geometry and visual helper as Height and Radius; do not duplicate it.

- [ ] **Step 3: Apply alpha immediately and persist**

On drag: update requested alpha, merge it into `theme.toml`, call `theme_config::apply(cx)`, then notify. The watcher may reapply after debounce; both paths must be idempotent.

Task 3 does not require a pixel-change smoke: the committed Task 1 floor remains
`min_alpha = 1.0`, so pixels are intentionally unchanged here. Verify the env
early-return regression with focused unit tests plus the persisted TOML value.
The live `CHRONOS_THEME=Default` slider/pixel gate runs in Task 5 only after its
temporary uncommitted calibration floor is set to `0.0`.

- [ ] **Step 4: Run tests and commit**

Run: `cargo test -p chronos bar_settings theme_config`

```bash
git add crates/app/src/side_panel_right/tab/bar_settings.rs crates/app/src/theme_config.rs
git commit -m "feat(settings): add live surface transparency controls"
```

### Task 4: Ship the opt-in Hyprland module and service bridge

**Files:**
- Create: `packaging/hyprland/45-surface-effects-chronos.lua`
- Modify: `packaging/hyprland/hyprland.ship.lua`
- Modify: `crates/services/src/compositor/hyprland.rs`
- Modify: `crates/services/src/compositor/mod.rs`
- Modify: `crates/services/src/compositor/types.rs`
- Create: `crates/app/src/surface_effects.rs`
- Modify: `crates/app/src/lib.rs`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/side_panel_right/tab/bar_settings.rs`

**Interfaces:**
- Produces Lua: `_G.chronos_set_blur_enabled(enabled)` and retained layer/window rule handles.
- Produces Rust: `BlurCapability::{Available, ModuleMissing, Unsupported}`.
- Produces Rust service API: `probe_shell_blur() -> BlurCapability` and `set_shell_blur_enabled(bool) -> anyhow::Result<()>`.
- Produces app global: `SurfaceEffectsState { capability, requested_blur, error }`, initialized once after `theme_config::init` and consumed by Bar settings.

- [ ] **Step 1: Add pure command-rendering tests before I/O**

```rust
#[test]
fn blur_eval_lines_are_lua_not_legacy_dispatch() {
    assert_eq!(blur_probe_code(), "assert(type(_G.chronos_set_blur_enabled) == 'function', 'chronos blur module missing')");
    assert_eq!(blur_set_code(true), "_G.chronos_set_blur_enabled(true)");
}
```

- [ ] **Step 2: Create the opt-in Lua module**

```lua
local namespaces = "^(bar|side_panel_left|side_panel_right|volume-popup|system-popup|updates-popup|notifications|notif-history-popup|tray-menu|dock-menu|osd|desktop-terminal)$"

_G.chronos_surface_blur_rule = hl.layer_rule({
    name = "chronos-surface-blur",
    match = { namespace = namespaces },
    blur = true,
    blur_popups = true,
    ignore_alpha = 0.1,
})
_G.chronos_surface_blur_rule:set_enabled(false)

-- Launcher is an XDG toplevel, not a layer surface. This rule only protects
-- blur-off from a user's global window blur; it cannot enable window blur.
_G.chronos_launcher_no_blur_rule = hl.window_rule({
    name = "chronos-launcher-no-blur",
    match = { class = "^chronos-launcher$" },
    no_blur = true,
})

_G.chronos_set_blur_enabled = function(enabled)
    _G.chronos_surface_blur_rule:set_enabled(enabled)
    _G.chronos_launcher_no_blur_rule:set_enabled(not enabled)
end
```

The final `ignore_alpha` must remain lower than the measured surface floor and higher than fully transparent hover/shadow pixels. Hover-strip namespaces and `project-popup` are intentionally absent.

- [ ] **Step 3: Add the module to the shipped profile only**

`hyprland.ship.lua` may `dofile` the module because it is ChronOS-owned.
Documentation for an existing user config must show a manual `dofile`; Rust
code must never edit that config.

The layer rule guarantees blur only for the named layer surfaces. Launcher is
an XDG toplevel and `20-look.lua` does not enable `decoration.blur`; the inverse
`no_blur` rule therefore guarantees blur-off, but blur-on affects the launcher
only when the user has independently enabled Hyprland window blur. State this
next to the toggle and in the report. Do not claim the shipped profile enables
launcher blur, and do not mutate global compositor decoration settings from
ChronOS. If unconditional launcher blur becomes a requirement, it is a separate
ticket.

- [ ] **Step 4: Implement probe/set with the proven `hyprctl eval` CLI**

Keep compositor I/O in `crates/services`; do not spawn `hyprctl` from render handlers. Run `Command::new("hyprctl").args(["eval", code])` only on a background task. Parse exit status and output: exit success plus stdout `ok` means available/success, a missing-global error maps to `ModuleMissing`, and non-Hyprland maps to `Unsupported`. This deliberately uses the exact path proven in Task 0 instead of guessing the private socket framing for `/eval`.

- [ ] **Step 5: Initialize persisted blur state on cold start**

`surface_effects::init(cx)` runs after `theme_config::init(cx)`: set a Checking state, probe in the background, then—only when capability is Available—apply the persisted `Theme::global(cx).surface.blur_enabled`. Update the global and call `cx.refresh_windows()` on completion. This prevents `blur_enabled = true` from working only after the settings page has been opened.

- [ ] **Step 6: Wire the settings control**

Observe `SurfaceEffectsState` off the render path and notify the settings entity when it changes. Render these states:

- Hyprland bridge available: enabled toggle with current persisted state.
- Hyprland detected but module absent: disabled toggle + `Import packaging/hyprland/45-surface-effects-chronos.lua`.
- Non-Hyprland backend: disabled toggle + `Compositor does not support blur`.

Toggle persistence happens only after the compositor call succeeds; on error keep the previous state and show the error banner. Never render an enabled control whose action cannot reach the bridge.

- [ ] **Step 7: Run focused and package tests**

Run: `cargo test -p chronos-services compositor && cargo test -p chronos bar_settings`

Expected: all pass without a live compositor; I/O tests remain pure command/response fixtures.

- [ ] **Step 8: Commit**

```bash
git add packaging/hyprland/45-surface-effects-chronos.lua packaging/hyprland/hyprland.ship.lua crates/services/src/compositor/hyprland.rs crates/services/src/compositor/mod.rs crates/services/src/compositor/types.rs crates/app/src/surface_effects.rs crates/app/src/lib.rs crates/app/src/main.rs crates/app/src/side_panel_right/tab/bar_settings.rs
git commit -m "feat(hyprland): bridge compositor blur controls"
```

### Task 5: Calibrate readable alpha floors with live evidence

**Files:**
- Modify: `crates/ui/src/theme/mod.rs`
- Modify: `crates/ui/src/theme/schemes.rs`
- Modify: tests in the same files
- Evidence: `/tmp/t266-calibration-*`

**Interfaces:**
- Consumes: working alpha slider and all surface roots.
- Produces: measured `min_alpha` for Default and Light; the worktree-only `0.0` calibration floor is gone.

- [ ] **Step 1: Build release and capture baseline at alpha 1.0**

Run: `cargo build --release -p chronos`.

Capture identical geometries before/after the feature with untouched config. Compare via ImageMagick `compare -metric AE`; expected difference is `0` for owned shell pixels, allowing only dynamic clock/service text to be masked out.

- [ ] **Step 2: Sweep alpha on dark and light wallpapers in both themes**

For each scheme/wallpaper pair, capture the same surfaces at descending alpha values. Measure text/background contrast from pixel samples; minimum accepted contrast is WCAG AA `4.5:1` for ordinary text and `3:1` for large text/UI boundaries.

Start this sweep by setting `min_alpha = 0.0` only in the dirty calibration
worktree. With `CHRONOS_THEME=Default` and a non-opaque `surface_alpha` in an
isolated `XDG_CONFIG_HOME`, drag the slider and capture before/after frames plus
the resulting TOML. Both pixels and persisted value must change; this is the
live regression gate against the former env early return. Do not commit the
`0.0` floor.

- [ ] **Step 3: Set per-scheme floor to the first value that passes every covered surface**

Dark and Light may differ. Record sample coordinates, luminance calculation, winning value and first rejected value in the report. Do not choose by visual preference alone.

- [ ] **Step 4: Add exact floor tests**

```rust
#[test]
fn schemes_expose_measured_surface_floors() {
    let dark = Theme::default();
    let light = Theme::select_scheme(Some("Light".into()));
    assert!((0.0..=1.0).contains(&dark.surface.min_alpha));
    assert!((0.0..=1.0).contains(&light.surface.min_alpha));
    assert_eq!(dark.surface.alpha, 1.0);
    assert_eq!(light.surface.alpha, 1.0);
}
```

- [ ] **Step 5: Commit calibration**

```bash
git add crates/ui/src/theme/mod.rs crates/ui/src/theme/schemes.rs
git commit -m "fix(theme): enforce measured surface contrast floors"
```

### Task 6: Verify the real blur module live

**Files:**
- Evidence: `/tmp/t266-blur-*`
- Final evidence destination: `docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md`

**Interfaces:**
- Consumes: user-imported module, bridge, translucent surface roots.
- Produces: live proof that toggle changes compositor blur and off truly disables it.

- [ ] **Step 1: Import the repo module manually for the test session**

The user performs the import, or the smoke uses `hyprctl eval` to load the repo module transiently. Do not write `~/.config/hypr` from ChronOS.

- [ ] **Step 2: Prove missing-module behavior first**

Without the module, open Bar settings: toggle disabled, reason visible, alpha slider functional.

- [ ] **Step 3: Prove enabled bridge behavior**

With the module loaded, toggle blur on/off at the same alpha and geometry.
Capture both states over a detailed wallpaper; confirm the off frame restores
the unblurred wallpaper and logs show successful eval calls. This is the first
test that proves actual blur—Task 0 proved only handle persistence and runtime
enable/disable through `dim_around`.

- [ ] **Step 4: Verify namespaces and hover strips**

Run `hyprctl layers`; every covered layer-surface namespace matches the rule,
neither hover-strip namespace nor `project-popup` matches, and panel/bar
exclusive zones and bounds remain unchanged across alpha/blur states. Verify
launcher alpha in all cases. Record launcher blur as conditional diagnostics:
off must remain unblurred through `no_blur`; on is expected only when the user's
global Hyprland window blur is enabled and is not a T266 acceptance gate.

### Task 7: Full acceptance matrix and report

**Files:**
- Create: `docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md`

**Interfaces:**
- Produces: auditable acceptance result with exact commands, commits, frames, contrast measurements and any deferred blur ticket.

- [ ] **Step 1: Capture the required matrix**

For every scope layer surface: dark/light wallpaper × Default/Light theme × blur
off/on. Pair equivalent `grim -g` geometries and compare with the design HTML
side by side. Include default alpha `1.0` before/after evidence. Launcher joins
the full alpha/default matrix, but not the guaranteed blur-on matrix; report
whether host-global window blur was available instead of turning that condition
into a false failure or false promise.

- [ ] **Step 2: Run final static verification**

```bash
cargo fmt --check
cargo test --workspace --lib --bins
cargo build --release -p chronos
git diff --check
```

If global fmt fails on pre-existing unrelated drift, run targeted `rustfmt --check` on changed Rust files and document both facts.

- [ ] **Step 3: Write the report outcome-first**

The report must state:

- default pixel result;
- effective alpha floors and measurement method;
- all surface roots covered;
- T267 borders unchanged;
- hover strips checked, not changed;
- Lua API gate result and Hyprland version;
- module-missing/unsupported UI behavior;
- no user Hyprland config was modified;
- exact screenshot paths and any unverified claim.
- launcher blur limitation: ChronOS controls only inverse `no_blur`; blur-on is
  conditional on user-enabled global window blur.

- [ ] **Step 4: Commit report and final fixes**

```bash
git add docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md
git commit -m "docs: report surface transparency acceptance (T266)"
```

### Task 8: Review and integration

**Files:** none beyond reviewed changes.

- [ ] **Step 1: Inspect named staged/committed scope and worktree status**

Run: `git status --short && git diff <base>...HEAD --check && git diff <base>...HEAD --stat`.

`<base>` must be the T263-containing commit used to create the sibling worktree.
If T263 is not committed and present in that base, stop: rebasing T266 over the
dirty master version of `theme_config.rs` is forbidden.

- [ ] **Step 2: Re-run the final test suite on the feature branch**

Run: `cargo test --workspace --lib --bins`.

- [ ] **Step 3: Use `requesting-code-review` and resolve findings**

Review must explicitly challenge root-fill coverage, alpha multiplication, config preservation, render-thread blocking, Lua response parsing, and false-positive capability UI.

- [ ] **Step 4: Use `finishing-a-development-branch` for merge/PR/keep choice**

After integration, rerun tests on the merged result before any owned-worktree cleanup.
