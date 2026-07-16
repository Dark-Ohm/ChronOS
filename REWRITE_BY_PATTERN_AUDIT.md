# REWRITE-BY-PATTERN Audit: gpui-shell APP-LAYER + UI-CRATE → ChronOS

**Mode:** READ-ONLY. No donor or ChronOS code was modified.
**Donor:** `/home/neo/projects/chronos-ecosystem/ChronOS/reference/gpui-shell-main` ("gpui-shell", andre-brandao)
- gpui dep: `Cargo.toml:6` → `gpui = { git = "https://github.com/zed-industries/zed", branch = "main", features = ["wayland"] }` — **UNPINNED upstream zed `main`**.
- **LICENSE: ABSENT** (no LICENSE/COPYING/MIT/Apache anywhere in tree; `search_files LICENSE*` → 0 hits; `ls` confirms none). → "all rights reserved" → only legal path is REWRITE-BY-PATTERN (our code, their architecture).
- README.md **DOES** exist (3846 B) — see AGENTS.md flag §6.

**Target (ChronOS):** `/home/neo/projects/chronos-ecosystem/ChronOS`
- gpui dep: `Cargo.toml:6` → `gpui = { path = "../Source/gpui" }` — our **pinned gpui-ce fork**. API differs from upstream `main`.
- Current app modules: `bar/`, `launcher/`, `ipc/`, `plugin_bridge/`, `state.rs`. **No `config/`, no `theme`, no `control_center/`, no `keybinds.rs`.**
- Services crate (`crates/services/src/`): only **3** subscribers — `compositor`, `network`, `upower`. Donor has **13** (audio, bluetooth, brightness, privacy, tray, mpris, wallpaper, notification, applications, + those 3).

---

## 1. control_center/ — quick-settings popup panel

**Donor files + LOC (total 2785):**
| file | LOC | role |
|---|---|---|
| `control_center/mod.rs` | 804 | panel root; `Render`, `Focusable`, service subscriptions, dynamic measure+resize |
| `wifi.rs` | 533 | `render_wifi_section`: scan, connect/disconnect, password prompt |
| `bluetooth.rs` | 505 | `render_bluetooth_section`: paired-device list, connect/disconnect (zbus) |
| `quick_toggles.rs` | 389 | wifi/bt/mic/cam toggle row |
| `sliders.rs` | 234 | volume + brightness slider renderers |
| `power.rs` | 135 | battery status + power-profile cycle |
| `icons.rs` | 115 | Nerd-Font glyph constants |
| `config.rs` | 37 | `ControlCenterConfig`/`PowerActionsConfig` (systemctl strings) |
| `tooltip.rs` | 33 | tooltip helper |

**What it IS:** a Wayland layer-shell *popup panel* for quick system controls (AGENTS.md: "Popup panels for system controls"). Root `ControlCenter` struct holds two `Entity<Slider>`, a `FocusHandle`, `WifiPasswordState`. On `new()` it subscribes sliders to `SliderEvent` and dispatches service commands; `subscribe_to_services()` wires 6 services (audio, bluetooth, brightness, network, privacy, upower) via donor's `watch(cx, AppState::X(cx).subscribe(), …)` + `cx.notify()`.

**Services it binds (command vocabulary we'd need):** `NetworkCommand::{Connect,Disconnect}`, `BluetoothCommand`, `AudioCommand::SetSinkVolume`, `BrightnessCommand::SetPercent`, `UPowerCommand::CyclePowerProfile`, `PrivacyService::webcam_access()`. All go through `AppState::X(cx).dispatch(...)`.

**Reusable architecture vs throwaway:**
- *Reusable pattern:* the **panel = subscribe-to-services + dispatch-commands + cx.notify()** shape is exactly our `state::watch()` + `Service::subscribe()` model (`crates/app/src/state.rs:48`, `crates/services/src/lib.rs:55`). That is portable-by-rewrite.
- *Throwaway implementation:* the whole render body is bound to **upstream `main` gpui API** (`cx.theme()`, `div!` builder, `cx.new(|_| Slider::new()…)`, `layout_as_root` for measure at `mod.rs:781-790`, `EntityId`/`canvas` in slider). Plus it depends on **10 service backends we don't have** (audio, bluetooth, brightness, privacy, tray, mpris, wallpaper, notification, applications, +). Our services crate ships only compositor/network/upower.

**Scaffold-worthiness verdict:** **MEDIUM as architecture, NOT as code.** Steal the *panel subscription + command-dispatch skeleton*; do **not** copy the 2785 LOC. A faithful port is **blocked** until we build the missing service backends. As an immediate scaffold it is **premature** — the binding surface (services) doesn't exist yet.

**Our analog:** ABSENT (no `control_center/` in `crates/app/src`; `search_files control_center*` → 0 hits).
**Rewrite cost:** **XL** (gated on ~10 new service backends; the panel UI itself is M-L once services exist).
**Integration point:** `crates/app/src/control_center/` (future) + `crates/services/src/` (must add audio/bluetooth/brightness/privacy first).

---

## 2. config/ + hot-reload (watcher.rs, themes.rs, persistence, theme/*)

**Donor files + LOC:**
- `crates/app/src/config/mod.rs` — **198** — `Config` GPUI Global, `#[serde(default)]`, `start_hot_reload()` spawns `FileWatcher::watch` per `config.toml` + `theme.toml`; on `rx.recv()` → `Config::reload()` + `cx.refresh_windows()` (lines 133-185).
- `crates/app/src/config/persistence.rs` — **59** — `toml::from_str`/`to_string_pretty`, XDG config path, auto-create default on miss.
- `crates/app/src/config/theme/{mod,config,persistence}.rs` — ~**460** — `Theme` Global (`ActiveTheme` trait), base16 + `colorize`, `theme.toml` load/save.
- `crates/services/src/watcher.rs` — **90** — inotify: `WatchMask::MODIFY|CLOSE_WRITE|CREATE|DELETE|MOVED_TO|MOVE_SELF|DELETE_SELF`; watches **parent dir + matches filename**; **200 ms trailing debounce** via `Instant` (`DEBOUNCE_MS=200`, line 11); `mpsc::unbounded` to gpui thread (lines 42-73).
- `crates/services/src/themes.rs` — **356** — Base16 scheme **git clone** (`tinted-theming/schemes`, `--depth 1`), `load_cached()`, and **Stylix** `/etc/stylix/palette.json` loader.

**How their TOML+inotify hot-reload works:** `Config::init()` loads TOML → sets Globals → `start_hot_reload()` launches one inotify thread per file; the thread blocks on `read_events_blocking`, debounces 200 ms, sends `()` over a channel; a `cx.spawn` loop `recv()`s and calls `reload()` + `refresh_windows()`. Theme path is identical but reloads `Theme`.

**OUR hot-reload (per MEMORY + code):**
- `feat-inotify-hot-reload` merged to `master` (`d7ab5a7`, 2026-07-09): inotify watcher on `crates/luau` `PluginManager` + launcher `cache.rs` reuses the pattern. `Cargo.lock` confirms `inotify`/`inotify-sys` present.
- **BUT canonical `ARCHITECTURE.md:81` says "inotify hot-reload watcher — NOT YET" and `:218` "inotify hot-reload watcher still missing (§9)"** for the app-config path. Our app has **no `config/` module, no `theme` Global** (`search_files config|Config|theme` in `crates/app/src` → only `plugin_bridge` hot-reload mention). So: luau + launcher-entry hot-reload EXIST; **app config.toml/theme.toml hot-reload does NOT yet exist** in ChronOS.
- ⚠️ **Discrepancy flag:** MEMORY says the branch is merged; ARCHITECTURE.md (declared canonical in AGENTS.md) says app-level hot-reload is "NOT YET". The merged work covered *plugins + desktop-entry cache*, not *app config/theme*. Treat donor's config/theme hot-reload as **net-new** to us.

**What they do BETTER we should steal (rewrite-by-pattern):**
1. **Separate `watch_config` / `watch_theme` booleans** in `Config` (`config/mod.rs:27-29, 133-136`) — disable per-file reload without restart. Cheap, useful. *(Better than our single global watcher toggle.)*
2. **Theme Global + `theme.toml` TOML persistence + base16 + Stylix** (`theme/*`, `themes.rs`) — we have **zero** theme system at app layer. Wholesale-steal candidate (rewrite against gpui-ce).
3. **`Config` as serde Global with per-module config structs** (`BarConfig`, `LauncherConfig`, `ControlCenterConfig`, …) — we have no app config struct at all.
4. inotify impl itself: **comparable, not better** — ours is `WatchDescriptor`-based (more correct for moved files); donor is path/filename-based. No clear win; donor's 200 ms debounce matches ours.

**Our analog:** PARTIAL — inotify infra exists (`crates/luau/src/watcher.rs`, `launcher/cache.rs`); app `config/`+`theme` Global ABSENT.
**Rewrite cost:** **M–L** overall. TOML load/save + wiring to existing inotify = **S–M**; the Theme system (base16/colorize/schemes/Stylix) = **L** (net-new).
**Integration point:** `crates/app/src/config/` (new) + reuse `inotify` dep already in workspace.

---

## 3. keybinds.rs — global keybinds?

**Donor file:** `crates/app/src/keybinds.rs` — **84 LOC**.

**Concrete technique (read the whole file):**
- Defines actions via `actions!(keybinds, [Cancel, Confirm, CursorUp/Down, PageUp/Down, Backspace, DeleteWordBack, CursorLeft/Right, WordLeft/Right, Select*…])` (lines 4-28).
- `register(cx)` calls **`cx.bind_keys([ KeyBinding::new("escape", Cancel, Some("Launcher")), … ])`** (lines 30-83) — i.e. **pure in-app GPUI keybinding scoped by `key_context`** (`Some("Launcher")`, `Some("ControlCenter")`).
- Consumed in views via `.key_context("Launcher")` + `.on_action(cx.listener(|this, _: &Cancel, …| …))`.

**It is NOT:** a global shortcut daemon, evdev, winit, layerr, or compositor IPC. It is purely **GPUI's `bind_keys` action dispatch within an already-focused view**. No OS-level global hotkey.

**Does it address our launcher keyboard-focus Critical bug?** **NO.**
- Our Critical bug (MEMORY 'Launcher keyboard focus (Critical)') is **compositor-level**: `KeyboardInteractivity::OnDemand` + explicit focus doesn't auto-acquire on Hyprland/Niri; `activate_window()` (xdg_activation) is *rejected for layer-shell* (SESSION_REPORT.md:111-112, DECISIONS.log:194-199). keybinds.rs operates **after** a view is focused — it maps keys→actions, it cannot make the compositor deliver keys to the surface. The fix lives in the **launcher window/focus code, not keybinds.rs** (see §5).
- keybinds.rs is still **worth stealing** as a cleaner input model than our raw `on_key_down` string matching (see §5).

**Our analog:** ABSENT — our `launcher/view.rs` uses `on_key_down` with raw `key`/`key_char` string matching, no `actions!`/`bind_keys`/`key_context`.
**Rewrite cost:** **S** (84 LOC, pure pattern; gpui-ce has `actions!`/`KeyBinding`/`bind_keys`).
**Integration point:** `crates/app/src/keybinds.rs` (new) + `cx.bind_keys` at app init.

---

## 4. crates/ui/ — reusable primitives vs tied-to-upstream-gpui

**Donor files + LOC (total 2299):**
components/: `slider.rs` 225, `switch.rs` 254, `label.rs` 105, `stack.rs` 11, `list/{mod 7, list_container 95, list_item 142, list_separator 17}`, `input/{mod 5, buffer 349, line 111}`; `components/mod.rs` 16; `components/button.rs` **0 (EMPTY FILE)**.
theme/: `mod.rs` 370, `base16.rs` 203, `colorize.rs` 256, `schemes.rs` 51.
traits/: `mod.rs` 1, `styled_ext.rs` 20. `ui.rs` 61 (re-exports).

**Reusable vs tied-to-upstream-gpui verdict: MOSTLY TIED.**
- Everything rendering uses **upstream `main` gpui API**: `cx.theme()` (`ActiveTheme`), `Hsla`, `div!()` builder chains, `.on_drag/.on_drag_move`, `canvas()` for bounds (`slider.rs:147-163, 213-221`), `EntityId`, `Render` derive. None of this compiles against gpui-ce as-is (the brief is correct: upstream main ≠ gpui-ce fork).
- **Portable-by-rewrite (logic, not API):**
  - `input/buffer.rs` (349) — `InputBuffer` is a **near-pure text-cursor model** (`move_left/right(_select)`, `move_word_*`, `select_all`, `insert_str`, `backspace`, `delete_word_back`, `plain_render_parts`/`masked_render_parts` returning theme-agnostic structs). Only `use gpui::*;` at top; the cursor math is gpui-agnostic → highest-value portable chunk.
  - `theme/{base16.rs, colorize.rs}` — **pure color conversion** (hex↔`Hsla`, base16 palette math). Uses `Hsla` (which gpui-ce also exposes) but no runtime gpui calls → portable.
  - `theme/schemes.rs` (51) + `traits/styled_ext.rs` (20) — tiny, portable.
- **NOT portable (div-builder tied):** `slider.rs`, `switch.rs`, `label.rs`, `list/*`, `input/line.rs`.
- **Button primitive ABSENT** — `components/button.rs` is a 0-byte file, and `ui.rs` does not re-export `Button`. The brief's "button" primitive does not exist in donor either.

**Drift risk:** donor pins **unpinned `git = "…/zed", branch = "main"`** (`Cargo.toml:6`). Any upstream signature that drifted vs our gpui-ce fork will not compile. The primitives are therefore **rewrite-by-pattern against gpui-ce**, not vendored.

**Our analog:** ABSENT — our app has no `ui` crate / shared primitives / theme Global.
**Rewrite cost:** **M–L** overall. `InputBuffer` + theme color math = **M**; full div-builder re-expression of slider/switch/list/label against gpui-ce = **L**.
**Integration point:** new `crates/ui` (or `crates/app/src/ui/`) — pairs with §2's Theme Global.

---

## 5. Launcher comparison — focus handling (the Critical bug)

**OUR launcher (ChronOS):**
- `crates/app/src/launcher/mod.rs:35-65` `window_options()`: `keyboard_interactivity: KeyboardInteractivity::OnDemand` (line 60), `exclusive_zone: None`, no `focus: true` field, comment (31-34) explaining `Exclusive` "wedges the input stack" → crash.
- `crates/app/src/launcher/view.rs`: holds `focus: gpui::FocusHandle` (25, 40); `impl Focusable` (170-172). **But `render()` does NOT call `focus_handle.focus()`, does NOT `.track_focus(...)`, and handles input only via `.on_key_down(...)` (126).** No per-frame focus re-assertion.

**DONOR launcher (`crates/app/src/launcher/mod.rs`):**
- Open: `keyboard_interactivity: KeyboardInteractivity::Exclusive` (**line 642**) + `focus: true` (645) + anchor TOP. *(Note: donor chose the OPPOSITE of us; our MEMORY has empirical proof `Exclusive` crashes Hyprland/Niri — see §6 flag.)*
- `render()` **lines 336-338:** `if !self.focus_handle.is_focused(window) { self.focus_handle.focus(window, cx); }` — **re-acquires focus every single frame.**
- line 376: `.track_focus(&self.focus_handle)` on root div.
- line 377: `.key_context("Launcher")` + `.on_action(...)` for all nav/edit actions.

**Explicit focus-handling differences (donor vs ours):**
| # | Aspect | Donor | Ours | Missing in ours? |
|---|---|---|---|---|
| 1 | **Per-frame focus re-assert** | `render()` polls `is_focused` → `focus()` every frame (336-338) | none — focus set once at construction | **YES — the big one** |
| 2 | `track_focus` | `.track_focus(&focus_handle)` (376) | absent in `view.rs` | **YES** |
| 3 | `WindowOptions.focus: true` | present (645) | absent (no such field used) | likely |
| 4 | `keyboard_interactivity` | `Exclusive` (642) | `OnDemand` (60) — deliberate anti-crash | opposite (ours safer) |
| 5 | Input model | `key_context` + `on_action(actions!)` | raw `on_key_down` string match | different (ours cruder) |

**Can the donor's pattern fix our Critical bug?** **PARTIAL — and NOT the real fix.**
- Adding #1+#2 (per-frame `focus()` + `track_focus`) is a **cheap, legitimate hygiene improvement** (cost **S**) and *may* mitigate focus loss on some compositors by re-asserting GPUI focus each frame. **But** the root cause (per MEMORY/SESSION_REPORT/DECISIONS.log) is compositor-level: layer-shell `OnDemand` needs the compositor to deliver keyboard focus, and `activate_window()`→`xdg_activation_v1` is *rejected for layer-shell* (gpui itself comments this). If the compositor never routes keys to the surface, GPUI `focus()` is a **no-op** (GPUI believes focused, but no key events arrive). So per-frame focus() alone does **not** solve the Critical bug.
- The OMP/DECISIONS.log conclusion is option **(c): migrate launcher to XDG toplevel** (auto-receives focus, `activate_window()` works) — that is the genuine fix; it is **architectural**, not a keybind tweak.
- **Recommendation:** rewrite-by-pattern #1+#2+#5 (per-frame focus re-assert, `track_focus`, `key_context`+`actions!`) into `launcher/view.rs` as a *hygiene* fix (S cost), but treat it as a mitigation; the real resolution remains the toplevel migration tracked in DECISIONS.log (undecided, awaiting Architect).

**Our analog:** launcher EXISTS (cache/search/launch/view/entry/mod) — focus handling is the gap.
**Rewrite cost:** **S** for the focus-hygiene patch; **XL** for the actual toplevel migration.
**Integration point:** `crates/app/src/launcher/view.rs` (+ `mod.rs` window options).

---

## 6. AGENTS.md / claim flags (unconfirmable or wrong)

- ❌ **Brief claim "no README"** is **FALSE**. Donor root has `README.md` (3846 B) + `CONTRIBUTING.md` + `CLAUDE.md`(→AGENTS.md symlink). Only **LICENSE** is absent (confirmed). The *legal conclusion* ("rewrite-by-pattern only") still holds (no license = all rights reserved), but the "no README" sub-claim is wrong.
- ⚠️ **ChronOS `ARCHITECTURE.md:81/218` ("inotify hot-reload — NOT YET") vs MEMORY ("feat-inotify-hot-reload merged d7ab5a7").** Not contradictory on inspection: the merged branch delivered **luau plugin + launcher-entry** hot-reload, NOT app **config/theme** hot-reload. Per AGENTS.md, ARCHITECTURE.md wins → app config/theme hot-reload is genuinely **not yet present**. Flagged as a doc/state discrepancy, not unconfirmable.
- ⚠️ **Donor `launcher/mod.rs:642` uses `KeyboardInteractivity::Exclusive`** — directly contradicts our MEMORY's hard-won finding that `Exclusive` *crashes Hyprland/Niri*. Donor's launcher may itself be broken on those compositors; do **not** copy their interactivity choice. Our `OnDemand` is the safer one.
- ✅ Donor `panel.rs` (claimed in their AGENTS.md) **exists** — confirmed.
- ✅ Donor `control_center/`, `osd/`, `notification/` all exist as described.
- ⚠️ Donor `components/button.rs` is an **empty (0-byte) file** — there is no Button primitive to assess; the brief's "button" item is moot for the donor.
- ⚠️ Donor `traits/mod.rs` is 1 line (`pub mod styled_ext;`) and `styled_ext.rs` is 20 LOC — the "traits/" layer is negligible, not a real abstraction to port.

---

## Per-item summary

| # | Item | Donor LOC | Our analog | Rewrite cost | Integration point |
|---|---|---|---|---|---|
| 1 | control_center/ | 2785 | ABSENT | **XL** (gated on ~10 missing service backends) | `crates/app/src/control_center/` (future) + `crates/services/src` |
| 2 | config/ + hot-reload | ~707 (config) +90 (watcher) +356 (themes) | PARTIAL (inotify infra yes; app config/theme Global no) | **M–L** (TOML wiring S–M; Theme system L) | `crates/app/src/config/` (new) |
| 3 | keybinds.rs | 84 | ABSENT | **S** | `crates/app/src/keybinds.rs` (new) |
| 4 | ui/ crate | 2299 | ABSENT | **M–L** (InputBuffer+theme math M; full primitives L) | new `crates/ui` |
| 5 | launcher focus | 665 | EXISTS, focus gap | **S** (hygiene) / **XL** (toplevel fix) | `crates/app/src/launcher/{view,mod}.rs` |

**control_center scaffold verdict:** reusable *architecture* (service-subscribe + command-dispatch), throwaway *code*; not a drop-in scaffold until service backends exist → **MEDIUM**.
**keybinds verdict:** pure in-app GPUI `bind_keys`+`key_context` (no global daemon); **does NOT fix** the Critical focus bug.
**launcher focus fix verdict:** steal per-frame `focus()` + `track_focus` (#1+#2) as **S-cost hygiene**; real fix is XDG-toplevel migration (XL, undecided in DECISIONS.log).
**ui crate verdict:** **mostly tied to upstream `main` gpui**; only `InputBuffer` logic + theme color math (`base16`/`colorize`) are portable-by-rewrite; **Button primitive absent**.
