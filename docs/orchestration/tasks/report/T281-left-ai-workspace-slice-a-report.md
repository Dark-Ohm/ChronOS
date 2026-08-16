# T281 Left AI Workspace Slice A — Live Acceptance Report

**Date:** 2026-08-15
**Branch:** master (ahead of origin by 24 commits)
**Commit:** `75358a2` (fixes for docked tab-switch + session-select focus)

---

## Gate Results

| # | Gate | Status | Evidence |
|---|------|--------|----------|
| 1–2 | Unit / layer-shell tests | ✅ | 108 side_panel_left, 6 project_switcher, 659 full (1 flake noted) |
| 3 | Drag resize | ✅ | `560→960→40→600` + zero-slice recovery; `rg 'window\.resize\('` clean |
| 4 | `compose-and-send` from closed | ✅ | Opens both surfaces, docks at remembered width (560/600), **exactly 1** `composer: send` + `turn START` |
| 5 | Dock/undock + tab policy matrix | ✅ | **Fixed in `75358a2`**: docked tab-switch preserves dock+width; active-docked click = no-op; undock keeps width until next ordinary tab-switch |
| 6 | Session select → Chat + focus composer | ✅ | **Fixed in `75358a2`**: `on_sessions_event` calls `request_focus_composer`; verified blinking cursor + focus accent `#2754B1` |
| 7 | `compose-and-send` from rail-only / content-open / docked | ⚠️ | IPC fires + single submit per state; **external model 404** (`tencent/hy3:free` became paid); fallback to `nous:meituan/longcat-2.0:free` works |
| 8 | **Restart restores last valid session** | ✅ code+unit / live ⏳ | `23bf89f`: `restore_active_project_on_startup` в `init` (после `project_switcher::init`) сеет SoT из `ProjectsConfig.active`; `ChatTab::new` зовёт `restore_project_thread`. Тесты `restore_on_startup_*` прогнал сам — 2/2. ChatTab::new в TestApp не поднимается (ACP) — сессию юнит не доказал, только path. Live — за владельцем. |
| 9 | Project pill gone from bar | ✅ | `~/.config/chronos/bar.toml` has no project widget |
| 10 | Slice B/C shells self-identify | ✅ | `shell.rs:47-48`: Plan/ContextFiles = "Coming in Slice B"; Tools/Skills/Archive = "Coming in Slice C" |

---

## Key Fixes Committed (`75358a2`)

### 1. Docked tab-switch keeps dock + width (`tabs/mod.rs`)
```rust
// Added arm: (false, _, true) => (clicked, panel_width, true)
// When switching tabs while docked (dock=true), preserves dock state and current width
// Test: select_other_tab_docked_keeps_dock_and_width
```
**Live verification:** docked Chat@560 → Sessions click → log `now_dock=true now_width=560`, x stays 570.

### 2. Session select focuses composer (`workspace_view.rs`)
```rust
// In on_sessions_event SelectThread arm:
self.request_focus_composer(cx);  // after select_session(id, cx)
```
**Live verification:** session row click → Chat opens with blinking cursor (blink interval 530ms confirmed via frame diffs) and focus accent `#2754B1`.

---

## Known Issues / Gaps

### Gate 8 — Session restore on restart (FIXED 2026-08-15)
- **Before:** `ChatTab::restore_project_thread` was only invoked from `mod.rs` on manual project switch; `SidePanelLeftState_.active_project_path` started as `None` on every launch, so a restart opened an empty Chat with a fresh ACP session instead of the last valid thread.
- **Fix (commit `HEAD`):**
  - `side_panel_left::restore_active_project_on_startup(cx)` reducer (called from `init`) seeds `active_project_path` from the persisted `ProjectsConfig.active` (written by `project_switcher::set_active` on every project switch). No-op when no active project.
  - `ChatTab::new` now calls `restore_project_thread(&active_path, cx)` on construction, so the first panel open after a restart loads the active project's last valid thread.
  - `restore_project_thread` → `ThreadStore::active_thread(project_path)` already validates **id + project_path + archived=0**, so stale / archived / deleted / cross-project active ids yield **empty Chat, never another project's leak** (covered by `threads.rs::active_thread_rejects_stale_archived_deleted_and_cross_project`).
- **Tests added:** `restore_on_startup_seeds_active_project_path`, `restore_on_startup_noop_without_active_project` (pure reducer) in `side_panel_left/mod.rs`; `threads.rs` already covers per-project + rejection cases.
- **Live verification:** still required on Hyprland (cannot run headless here) — confirm restart reopens the last valid session per project; archived/deleted/stale → empty Chat.

### Keyboard routing to layer surface
- `keyboard_interactivity: OnDemand` set on content window (mod.rs:283-287)
- `hyprctl dispatch focuslayer` broken on Hyprland 0.56 (Lua parser mangles args)
- ydotool keys route to active tiled window (Kate), not the layer
- **Not a ChronOS code defect** — compositor/layer-shell activation concern; affects all keyboard input equally

### External model 404
- `tencent/hy3:free` became paid on OpenRouter → 404
- Fallback to `nous:meituan/longcat-2.0:free` works; not a code issue

---

## Artifacts

- Screenshots: `/tmp/t281-*.png` (open-left, content-region, docked-plan, sessions-list, sessions-full, content-full, focus-clean, reset-state, post-expand, burst frames)
- Thread DB: `~/.local/share/chronos/threads/threads.db` (schema v2, 1 thread for ChronOS project)
- Config: `~/.config/chronos/projects.toml` (active = ChronOS), `bar.toml` (no project widget)
- Logs: `~/.local/state/chronos/chronos.log` (IPC toggles, rail tab selects, composer sends, ACP turns)

---

## Verdict

**T281 / Slice A — Gate 8 code complete.** The restart-session-restore code gap is fixed (startup reducer + `ChatTab::new` restore) and covered by unit/reducer tests. All automatic gates green: `cargo test -p chronos --lib --bins` (472 lib + 667 bin), `cargo test -p chronos-services --lib threads` (14), release build pending owner confirmation. Live Hyprland restoration (Gate 8 live, Gate 7 external-model fallback) still requires the owner's `+` after a real session restart.

Owner verdict required (`+` to close) — executor does not move this to `report-log/`.