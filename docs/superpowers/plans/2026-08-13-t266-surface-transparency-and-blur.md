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
- Work in a sibling worktree so `../Source` path dependencies resolve; preserve all unrelated dirty state.

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
- Modify: `crates/ui/src/theme/schemes.rs`
- Modify: `crates/app/src/theme_config.rs`

**Interfaces:**
- Produces: `SurfaceTokens { alpha, min_alpha, blur_enabled }`.
- Produces: `Theme::surface_color(Hsla) -> Hsla`, which calls `color.opacity(self.surface.alpha)`.
- Produces: `ThemeConfig { scheme, surface_alpha, blur_enabled }` with defaults `None/false`.
- Produces: `persist_surface_settings(alpha: f32, blur_enabled: bool)` and `effective_surface_alpha(requested, min)`.

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
fn surface_color_multiplies_existing_alpha() {
    let mut theme = Theme::default();
    theme.surface.alpha = 0.5;
    let color = parse_hex("1e1e2ecc").unwrap();
    assert!((theme.surface_color(color).a - 0.4).abs() < 1e-6);
}
```

- [ ] **Step 2: Run tests and verify the new API is absent**

Run: `cargo test -p chronos-ui theme::surface theme_config`

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

During calibration, use `min_alpha = 0.0` only on the isolated worktree so the complete range can be measured; Task 5 replaces it with measured dark/light floors before acceptance, and no commit containing that calibration value may be integrated.

- [ ] **Step 4: Extend config resolution without changing absent-config behavior**

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

Persist by merging only `surface_alpha` and `blur_enabled` into the existing TOML document; do not erase `scheme` or unknown keys.

- [ ] **Step 5: Run focused tests**

Run: `cargo test -p chronos-ui theme::surface && cargo test -p chronos theme_config`

Expected: all pass; empty config resolves to alpha `1.0`, blur `false`.

- [ ] **Step 6: Commit**

```bash
git add crates/ui/src/theme/surface.rs crates/ui/src/theme/mod.rs crates/ui/src/theme/schemes.rs crates/app/src/theme_config.rs
git commit -m "feat(theme): add configurable surface effects"
```

### Task 2: Apply alpha only to outer surface fills

**Files:**
- Modify: `crates/app/src/bar/mod.rs`
- Modify: `crates/app/src/side_panel_left/panel.rs`
- Modify: `crates/app/src/side_panel_right/surfaces.rs`
- Modify: `crates/app/src/side_panel_right/view.rs`
- Modify: `crates/app/src/side_panel_right/rail.rs`
- Modify: `crates/app/src/volume_popup/view.rs`
- Modify: `crates/app/src/system_popup/view.rs`
- Modify: `crates/app/src/updates_popup/view.rs`
- Modify: `crates/app/src/notifications/view.rs`
- Modify: `crates/app/src/notifications/history_popup/view.rs`
- Modify: `crates/app/src/tray_menu/view.rs`
- Modify: `crates/app/src/dock/context_menu.rs`
- Modify: `crates/app/src/osd/view.rs`
- Modify: `crates/app/src/launcher/view.rs`
- Modify: `crates/app/src/desktop_terminal/view.rs`

**Interfaces:**
- Consumes: `Theme::surface_color` from Task 1.
- Produces: every listed window's outer chrome uses the effective alpha exactly once.

- [ ] **Step 1: Inventory and label one root fill per window**

Record the chosen expression in the report before editing. Examples:

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

Right-panel `surfaces::chrome/card/well` retain their light/dark role mapping; apply alpha at the window chrome call site, not inside every card helper. Hover strips contain no fill and must remain untouched.

- [ ] **Step 4: Verify exact scope**

Run:

```bash
git diff --check
rg -n 'surface_color' crates/app/src
rg -n 'border_color\(theme\.border\.subtle\)' crates/app/src/{bar/mod.rs,side_panel_left/panel.rs,side_panel_right/view.rs}
```

Expected: one root application per covered surface; T267 borders unchanged.

- [ ] **Step 5: Check and test**

Run: `cargo check -p chronos && cargo test -p chronos`

Expected: success, existing warnings only.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/bar/mod.rs crates/app/src/side_panel_left/panel.rs crates/app/src/side_panel_right/surfaces.rs crates/app/src/side_panel_right/view.rs crates/app/src/side_panel_right/rail.rs crates/app/src/volume_popup/view.rs crates/app/src/system_popup/view.rs crates/app/src/updates_popup/view.rs crates/app/src/notifications/view.rs crates/app/src/notifications/history_popup/view.rs crates/app/src/tray_menu/view.rs crates/app/src/dock/context_menu.rs crates/app/src/osd/view.rs crates/app/src/launcher/view.rs crates/app/src/desktop_terminal/view.rs
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
local namespaces = "^(bar|side_panel_left|side_panel_right|volume-popup|system-popup|updates-popup|notifications|notif-history-popup|tray-menu|dock-menu|osd|project-popup|desktop-terminal)$"

_G.chronos_surface_blur_rule = hl.layer_rule({
    name = "chronos-surface-blur",
    match = { namespace = namespaces },
    blur = true,
    blur_popups = true,
    ignore_alpha = 0.1,
})
_G.chronos_surface_blur_rule:set_enabled(false)

-- Launcher is an XDG toplevel, not a layer surface. Window blur is normally
-- compositor-global, so keep an inverse no_blur rule enabled by default.
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

The final `ignore_alpha` must remain lower than the measured surface floor and higher than fully transparent hover/shadow pixels. Hover-strip namespaces are intentionally absent.

- [ ] **Step 3: Add the module to the shipped profile only**

`hyprland.ship.lua` may `dofile` the module because it is ChronOS-owned. Documentation for an existing user config must show a manual `dofile`; Rust code must never edit that config. Verify the launcher rule separately because it uses `hl.window_rule`, while all other listed surfaces use the layer rule or its `blur_popups` path.

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

With the module loaded, toggle blur on/off at the same alpha and geometry. Capture both states over a detailed wallpaper; confirm the off frame restores the unblurred wallpaper and logs show successful eval calls.

- [ ] **Step 4: Verify namespaces and hover strips**

Run `hyprctl layers`; every covered surface namespace matches the rule, neither hover-strip namespace matches, and panel/bar exclusive zones and bounds remain unchanged across alpha/blur states.

### Task 7: Full acceptance matrix and report

**Files:**
- Create: `docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md`

**Interfaces:**
- Produces: auditable acceptance result with exact commands, commits, frames, contrast measurements and any deferred blur ticket.

- [ ] **Step 1: Capture the required matrix**

For every scope surface: dark/light wallpaper × Default/Light theme × blur off/on. Pair equivalent `grim -g` geometries and compare with the design HTML side by side. Include default alpha `1.0` before/after evidence.

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

- [ ] **Step 4: Commit report and final fixes**

```bash
git add docs/orchestration/tasks/report/T266-surface-transparency-and-blur-report.md
git commit -m "docs: report surface transparency acceptance (T266)"
```

### Task 8: Review and integration

**Files:** none beyond reviewed changes.

- [ ] **Step 1: Inspect named staged/committed scope and worktree status**

Run: `git status --short && git diff <base>...HEAD --check && git diff <base>...HEAD --stat`.

- [ ] **Step 2: Re-run the final test suite on the feature branch**

Run: `cargo test --workspace --lib --bins`.

- [ ] **Step 3: Use `requesting-code-review` and resolve findings**

Review must explicitly challenge root-fill coverage, alpha multiplication, config preservation, render-thread blocking, Lua response parsing, and false-positive capability UI.

- [ ] **Step 4: Use `finishing-a-development-branch` for merge/PR/keep choice**

After integration, rerun tests on the merged result before any owned-worktree cleanup.
