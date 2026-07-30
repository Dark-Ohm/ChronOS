# ChronOS Adaptive Developer + Gamer Shell-IDE

**Date:** 2026-07-30  
**Status:** Approved design  
**Scope:** Product-level interface architecture for the first two ChronOS workspace modes  
**Implementation stack:** Rust, gpui-ce ChronOS fork, gpui-rsx, gpui-component, Hyprland 0.55.4+, mlua/LuauJIT

## 1. Decision summary

ChronOS will become a desktop-first adaptive Shell-IDE with two initial workspace modes:

1. **Developer mode** — a hybrid workbench combining native ChronOS tools with first-class external Wayland applications.
2. **Gamer mode** — a cinematic game hub at rest and a low-latency right-edge Game Deck while playing.

Both modes share the same shell geometry, navigation contracts, command system, component primitives, theme roles, accessibility behavior, plugin model, and workspace-scene model. A mode changes composition, priorities, defaults, shortcuts, and effects; it does not teach the user a different operating system.

Mode switching is explicit. ChronOS may detect a development or game session and offer a non-blocking one-click prompt, but it must never switch the workspace automatically.

This specification supersedes the narrower proposal to organize the product primarily around a unified System control center. The System Deck remains an important shared tool inside the right panel.

## 2. Product principles

### 2.1 Desktop first

ChronOS augments the Hyprland desktop rather than replacing every mature application. Normal Wayland clients remain first-class work surfaces. Shell panels overlay by default so opening a tool does not unexpectedly re-tile the workspace.

### 2.2 Shared muscle memory

The top bar, left intelligence rail, right tool rail, dock, launcher, command palette, focus language, and dismissal rules remain spatially stable across modes.

### 2.3 Adaptive rather than universal

Developer and Gamer modes expose different tool sets and density. ChronOS must not place every tool for every audience in one permanent cockpit.

### 2.4 Spectacle under discipline

ChronOS may use blur, springs, live previews, shader transitions, responsive wallpaper, edge illumination, audio visualization, and cinematic scene transitions. Effects must never delay input, conceal state, reduce legibility, or compromise the 144 FPS target.

### 2.5 Real state only

The interface must never invent connection, build, agent, device, game, capture, or service status. Unknown state is shown as unavailable, stale, disconnected, or omitted.

## 3. Shared shell architecture

ChronOS has five stable interface layers.

### 3.1 Top bar — global truth

The bar shows only state that remains meaningful across the desktop:

- ChronOS entry and launcher
- active workspace mode
- active project, game, or scene context
- important build, recording, streaming, or permission state
- workspaces and active-window context where useful
- notification state
- compact system cluster
- clock

The mode control always exposes an accessible text label, but it must respect the accepted bar composition in `docs/STYLE.md`: CAVA remains strictly centered and the clock remains the far-right item. The mode switch therefore extends the existing project/context control in the right cluster or opens from the ChronOS entry; it must not occupy or displace the center slot. Tool-specific controls belong in panels, not the top bar.

### 3.2 Left rail and intelligence panel

The left side owns intelligence and continuity:

- ACP agent conversations
- persistent threads and search
- project or game context
- permissions and plans
- task history and automations
- attachments and tool activity

The rail stays at the screen edge. Its content opens inward as an overlay by default and may be explicitly docked. Thread, composer, permission, and dismissal behavior is shared across modes.

### 3.3 Center workspace canvas

The center remains the primary work or play surface. It may contain:

- normal Hyprland-tiled or floating application windows
- native ChronOS workbench surfaces
- saved workspace scenes
- live previews or dashboards
- game content

ChronOS remembers scene composition without forcing all content into one monolithic native window.

### 3.4 Right rail and contextual tool panel

The fixed right-edge rail owns tools and inspectors. Content opens inward. Rail-only width remains the exclusive compositor zone; overlay content does not reserve desktop width unless the user explicitly enables dock mode.

Two existing constraints are load-bearing and must survive the refactor. The hover-reveal strip is deliberately created with `exclusive_zone: None` (`side_panel_right/hover_strip.rs`) — it must never claim compositor width. And on Hyprland's Overlay layer, combining an exclusive zone with a stretched anchor skews the top/bottom gaps (`side_panel_right/mod.rs:51`); the panel therefore computes only the top gap and relies on the bar's own exclusive zone. Any new panel or deck follows the same rule rather than re-deriving it.

The active tool follows explicit user selection. Contextual data may follow the focused resource, but the panel must not switch tabs unexpectedly.

### 3.5 Dock and launcher

The dock and launcher provide movement between:

- applications
- projects
- games
- workspace scenes
- recent resources
- commands and system actions

The dock may use magnification and live state, but targets must not move so aggressively that muscle memory becomes unreliable.

### 3.6 Outputs and the pult monitor

All shell chrome — bar, both side panels, popups, launcher and the Game Deck — lives on a single designated output, the **pult monitor**. ChronOS does not replicate panels per output.

`crates/app/src/monitor.rs::pult_display()` is the only resolver. It reads a display UUID from `~/.config/chronos/monitor.toml`, falls back to the largest display by area, then to the first display, and auto-designates the winner on first run. Surfaces must not call `cx.primary_display()` directly.

Scene and per-game output references are stored as display UUIDs, never as indices or `DisplayId` values, which are not stable across sessions or hotplug.

On hotplug, if the configured pult output disappears, the shell re-resolves via the fallback chain and surfaces a visible notice; scene state is preserved and re-applied if the output returns. Application windows belonging to a scene stay under compositor control and are not forcibly migrated.

## 4. Workspace modes

## 4.1 Developer mode

### Purpose

Optimize code, automation, inspection, testing, preview, project navigation, and agent-assisted work without rebuilding every mature development tool inside ChronOS.

### Default composition

- Left panel: ACP agent, persistent threads, project context
- Center: external editor or native lightweight editor, browser/preview, and project windows
- Right rail: see the flat tab set below
- Dock: projects, editors, terminals, browsers, documentation, and named scenes
- Top bar: project, branch or task, build/test state, permissions, notifications, system state

### Right rail tab set

Every tool is a first-class rail tab. Settings are **not** collapsed behind a single gear entry — the rail has vertical room on the pult monitor, and one click to any tool beats a nested menu.

Existing (`PanelTab` in `side_panel_right/tabs.rs`, 10 variants):

- System, Files, Editor, Terminal
- AcpSettings, McpSettings, LspSettings, ApiProviders, EditorSettings, HyprlandBinds

Added by this design:

- Preview, Inspector, Build, SourceControl

Fourteen tabs total. Ordering groups work tools first, settings second, with a visual separator between the groups; settings tabs keep the same icon language rather than becoming a different control class. The rail scrolls vertically only when the output height cannot fit the set — at the reference resolution it does not.

Gamer mode replaces the work-tool group with its own tools and keeps the settings group intact; §5 rail-position stability applies to the shared tabs.

### Native workbench responsibilities

ChronOS should provide:

- read-oriented project tree with later file operations behind explicit capability work
- embedded PTY/VT terminal
- lightweight editor for configuration, scripts, logs, snippets, and quick fixes
- build, test, task, and run orchestration
- logs and diagnostics
- browser or GPUI preview surface
- UI hierarchy and design-token inspector where technically available
- ACP context sharing across the selected project and resources
- workspace scenes that group external applications and native tools

### External application contract

Zed, VS Code, JetBrains IDEs, terminals, browsers, Blender, and other specialist applications remain normal Wayland clients. ChronOS may:

- launch and focus them
- group them into a scene
- associate them with project metadata
- expose shared commands and status
- place companion panels around them

ChronOS must not claim editor, debug, or document integration that the external application cannot provide.

### Developer density

Developer mode defaults to compact spacing, information-rich lists, strong text hierarchy, monospace technical values, restrained motion, and syntax-aware semantic accents.

## 4.2 Gamer mode

### Purpose

Optimize launching, tuning, playing, capturing, communicating, and returning to desktop without placing permanent shell clutter over the game.

### At-rest game hub

The Gamer workspace may show:

- recent and pinned games
- session history and playtime
- artwork and live game media
- achievements and social presence when real integrations exist
- captures and replay clips
- hardware and controller state
- per-game scene configuration

A game scene can remember monitor, Hyprland workspace, audio output, microphone, performance profile, recording defaults, and companion applications. Resolution and refresh-rate control remain capability-gated follow-ups until a real display service and safe revert flow exist.

### In-game Game Deck

A user shortcut opens a low-latency right-edge overlay over the game. It does not reserve width and does not force a workspace switch.

Game Deck sections:

- FPS, frame time, temperatures, utilization, power, VRAM and network latency
- performance profile and per-game overrides
- screenshot, recording, replay buffer and stream state
- application/game/chat audio mixer
- microphone and voice routing
- controller and input state where available
- friends, invites and notifications from real integrations
- return to desktop and safe game termination

The default overlay prioritizes immediate actions over charts. Historical graphs expand on request.

### Gamer input model

- keyboard and mouse remain fully supported and are the only baseline requirement
- opening the overlay focuses its first actionable control
- closing returns input to the game
- destructive actions require confirmation
- the overlay never closes merely because keyboard focus or Wayland activation changes

Controller navigation — spatial focus ring, predictable grid movement, controller and battery state — is **capability-gated**. No gamepad service exists in the tree today; until one does, the Game Deck presents controller data in the `unavailable` state of §13 and is driven by keyboard and mouse. Building that service is the entry condition of implementation slice 6, and controller navigability is an acceptance criterion of that slice, not of the architecture as a whole (§15).

### Gamer visual character

At rest, Gamer mode may use richer artwork, cinematic transitions, responsive wallpaper, audio visualization, and bolder telemetry. Under active input, animations shorten and chrome becomes quiet.

## 5. Mode switching

### Manual entry points

- top-bar mode control
- launcher
- command palette
- configurable global shortcut

### Smart prompt

ChronOS may detect signals such as a recognized game entering fullscreen or a development project session becoming active. It may show a non-modal prompt:

- `Switch to Gamer mode?`
- `Open Developer scene for ChronOS?`

The prompt must be dismissible, remember a per-application preference, and never steal keyboard focus.

### Transition contract

Mode switching may:

- restore the selected mode's last scene
- change which mode-specific rail tools are present while preserving the positions of shared tools
- change dock contents
- change effect profile
- update shortcuts scoped to the mode

Mode switching must not:

- terminate applications
- discard panel or editor state
- move external windows without a visible scene transition
- change audio, performance, recording, or display configuration without explicit scene settings

Workspace mode and the existing compositor `GamingModeState` are separate layers. Entering Gamer mode changes shell composition only. Applying the compositor gaming profile (animations/blur off, tearing allowed, performance power profile, DND) is an explicit per-scene or user action with observable success/failure and rollback; the UI must not report it active merely because Gamer mode is selected.

## 6. Shared System Deck

The System tab evolves from passive telemetry into a modular control surface available in both modes.

### Immediate modules

- Network and VPN
- Bluetooth
- Audio output, input and volume
- Brightness when supported
- Power profile

### Context modules

- active MPRIS media
- pending permission request
- actionable updates
- battery when present
- recording/streaming state in Gamer mode

### Live system module

- compact CPU, RAM and GPU values first
- expandable history charts
- secondary network throughput
- exception-oriented disk state

### Footer

- Lock acts immediately
- Log out, restart and shutdown use the existing arm/confirm model

Modules may be reordered only in explicit customization mode. Empty or unsupported modules disappear rather than presenting dead controls.

## 7. Navigation and interaction contracts

### Panel behavior

- panels overlay by default
- dock mode is explicit
- rail positions never swap between modes
- header and footer remain fixed while the content middle scrolls
- scroll position, selected resource and expanded module persist while hidden

### Dismissal

On layer-shell a surface does not receive clicks landing on other surfaces; it learns about them only through focus loss or pointer leave. "Click away to close" and "never close on focus loss" are therefore the same signal, and a single dismissal list cannot hold both. Surfaces split into two classes.

**Transient surfaces** — menus, dropdowns, comboboxes, context menus, pickers, tooltips. Implemented as `WindowKind::AnchoredPopup` with a real input grab, so click-away dismissal is delivered natively by the compositor. Dismissed by: Escape, click-away, selecting an item, or the owning control's toggle.

**Persistent surfaces** — left panel, right panel, popups owned by a bar widget, Game Deck. No click-away. Dismissed only by: Escape, rail or bar toggle, explicit close button, or IPC command. They must not close on focus loss, keyboard deactivation, or pointer leave. This matches the behavior already shipped for `updates_popup`.

A transient surface opened from a persistent one closes alone; dismissing it never closes its parent — this is the first step of the escape hierarchy below.

### Escape hierarchy

1. close transient menu or picker
2. leave detail view and return to tool overview
3. close overlay panel
4. return input to the underlying application or game

### Keyboard

- Tab follows visible visual order
- arrow keys navigate grids, rails, menus and controller-oriented layouts
- Enter and Space activate
- shortcuts are discoverable in tooltips and the command palette
- focus never lands on a hidden or collapsed control

### Pointer and touch target sizes

- precision desktop controls: minimum 36 px where density demands it
- primary quick actions and controller-ready controls: 40–44 px minimum
- resize handles require a visible or forgiving hit region

## 8. Visual design system

### Brand character

ChronOS is a playful modular technical instrument. Playfulness comes from responsive composition, tactile objects, spatial continuity, live previews, and controlled semantic color—not random card colors or decorative clutter.

### Typography

- Inter: navigation, labels, prose and controls
- JetBrains Mono: code, paths, identifiers, percentages, timing, status and telemetry

### Shape

- primary radii: 6–10 px
- cards are used for bounded modules, not every text group
- avoid excessive pills
- avoid generic mobile-style shadows

### Color

Implementation uses `Theme::global(cx)` semantic roles. The canonical dark palette remains Mocha-like with `#007acc` principal accent. Light mode uses accepted Light C surfaces and indigo text. Avoid pure black and pure white.

Color must not be the only status channel. Pair status color with text, icon, shape, location, or pattern.

### Effect tiers

1. **Essential** — focus, selection, continuity, visibility
2. **Enhanced** — backdrop blur, springs, dock response, live thumbnails
3. **Cinematic** — shaders, responsive wallpaper, audio visualizers, scene transitions, edge illumination
4. **Reduced** — static surfaces, minimal opacity transition, no blur

The user selects a preferred tier. ChronOS may temporarily degrade expensive effects under measured frame pressure, but must not silently change functional layout.

### Motion

- use opacity and transforms where possible
- use existing easing and spring APIs from the fork
- panel motion preserves spatial origin from the relevant edge
- do not animate layer-shell exclusive zones
- mode transitions show window movement rather than teleporting it; if reliable external-window animation is unavailable, use a short shell-owned scene cross-fade and apply compositor moves atomically rather than faking per-window motion
- anchored positioning itself is proven and accepted from live use (T117); the open fork-level motion defects are the resize ghost-trail (renderer buffer resize lags `set_size` by a tick — `gpui_linux/.../wayland/window.rs:1548-1559`, HANDOFF item #8-bis) and the related dropdown jank. Neither is a dependency of this design: panels and decks must remain usable with those defects present, and no layout here may be built on their fix landing first
- no looping decorative animation without functional meaning
- respect reduced-motion preference

## 9. Accessibility

Minimum target: WCAG AA for applicable desktop UI content.

- normal text contrast: 4.5:1
- large text, focus indicators and essential iconography: 3:1
- visible keyboard focus distinct from hover
- complete keyboard navigation
- controller focus in Gamer mode
- semantic names for icon-only controls
- status never encoded only by color
- text scaling must not clip critical controls
- live telemetry is not continuously announced
- actionable threshold changes may use a polite live region
- reduced motion and reduced effects are first-class settings

## 10. Performance and technical constraints

- target 144 FPS shell animation on the declared reference machine
- input latency takes precedence over visual richness
- no continuous full-panel repaints for unchanged telemetry
- history buffers update only when real samples arrive
- expensive module/device data loads when expanded or needed
- use valid GPUI scrolling with element IDs and structural clipping
- use layer-shell and anchored-popup APIs from the local fork, not assumed upstream APIs
- preserve known Wayland lifecycle and focus-loss constraints
- blur and shader features require graceful non-support paths

## 11. GPUI implementation boundaries

### Preserve

- left and right layer-shell panel architecture
- fixed right-edge activity rail
- overlay/dock and resize model
- `PanelTab` container concept
- `ScrollHandle` middle-content scrolling
- current service subscription model
- persistent ACP threads
- two-step power confirmation
- theme token source split across `crates/ui/src/theme/mod.rs` (`Theme::default` — accent and the base role set, including `#007acc`) and `crates/ui/src/theme/schemes.rs` (base16 schemes and the light-mode overrides, which deliberately do not override the accent)
- pult-monitor resolution via `crates/app/src/monitor.rs`

### Refactor

- move each right-panel tool into its own renderer/entity boundary
- replace temporary T157 measurement widgets with actual Developer tools
- change the System header from static active-window text to real tool/context information
- define explicit panel open/close toggle semantics
- add shared workspace-mode and scene state outside individual panel views
- model optional integrations as capability adapters with explicit availability, stale, error, and recovery state
- add effect-tier state with reduced-motion integration
- collapse the per-module `pick_display()` helpers in `updates_popup`, `desktop_terminal`, `notifications/history_popup` and `bar` onto the single `monitor.rs::pult_display()` resolver (§3.6)

### Do not do

- do not build Developer and Gamer as independent shells
- do not hard-code palette values in runtime components
- do not auto-switch modes
- do not close panels on focus loss
- do not fabricate integration status
- do not turn `side_panel_right/view.rs` into a larger monolith

## 12. Component inventory

### Shared primitives

- ActivityRail
- RailButton
- OverlayPanel
- DockToggle
- ToolHeader
- ToolFooter
- Module
- ModuleGrid
- StatusBadge
- InlineMeter
- SpectrumChart
- Toggle
- Slider
- DevicePicker
- CommandItem
- ConfirmationAction
- EmptyState
- ErrorState
- LoadingState
- StaleDataState
- FocusRing

### Developer-specific composites

- ProjectTree
- TerminalView
- LightweightEditor
- BuildPipeline
- TestResults
- LogsView
- PreviewSurface
- InspectorTree
- TokenInspector
- SceneManager

### Gamer-specific composites

- GameLibrary
- GameHero
- SessionSummary
- GameDeck
- PerformanceGrid
- FrameTimeChart
- CaptureControls
- AudioMixer
- VoiceRouting
- SocialPanel
- ControllerFocusGrid
- GameSceneEditor

## 13. States and failure behavior

Every interactive component must define:

- default
- hover
- focus
- pressed
- selected
- disabled
- loading
- unavailable
- stale
- warning
- error
- permission required
- confirmation armed

Errors must explain the affected capability and offer a recovery action where one exists. A failed optional integration must not collapse the entire mode.

## 14. Initial implementation slices

This design intentionally separates product architecture from implementation planning. The recommended sequence for the subsequent plan is:

1. workspace-mode state, manual switcher and smart-prompt contract
2. shared scene model and stable shell-mode composition
3. right-panel tool modularization and real Developer tool replacement for T157 scaffolding
4. Developer hybrid-workbench minimum: Files, Terminal, Build/Logs, Preview, System
5. Gamer at-rest hub shell and per-game scene model
6. low-latency Game Deck with real telemetry and capture/audio capability adapters, plus the gamepad input service that controller navigation depends on
7. effect tiers, frame-pressure degradation and reduced-motion behavior
8. keyboard, controller, accessibility and end-to-end visual QA

Each slice requires its own approved implementation plan and observed runtime verification.

## 15. Acceptance criteria

The architecture is successful when:

- users can identify and switch Developer/Gamer mode without learning a new navigation model
- no mode switch occurs without explicit user action
- panels overlay without unexpectedly changing tiled-window geometry
- Developer mode can coordinate native tools and external applications in one scene
- Gamer mode keeps the active game dominant and exposes the Game Deck without workspace disruption
- System control remains available and consistent in both modes
- all critical paths are keyboard accessible (controller navigability is an acceptance criterion of slice 6, gated on a real gamepad service — §4.2)
- all shell chrome resolves its output through `pult_display()`, and a pult-output hotplug loses no scene state (§3.6)
- dark and Light C themes preserve hierarchy and WCAG AA contrast targets
- Reduced effects mode preserves every function
- live shell interaction remains smooth at the 144 FPS target on the reference machine
- missing integrations are represented honestly and fail locally

## 16. Out of scope for this specification

- additional profession modes
- full replacement for mature IDEs or creative applications
- a complete game-store backend
- fabricated universal social or achievement integrations
- implementation task breakdown and code changes
- redesign of individual popup visuals not required by the shared contracts
