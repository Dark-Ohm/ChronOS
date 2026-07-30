# ChronOS ← gpui-shell: REWRITE-BY-PATTERN Cost Audit (9 other services + Tray)

**Scope:** Read-only audit. No donor/ChronOS source was modified. Estimates are for a
*clean-room rewrite* (our code, donor architecture pattern) — the only legal path, because
gpui-shell has **no LICENSE / all-rights-reserved** (0 hits, confirmed earlier).
**Target contract:** `crates/services/src/lib.rs:55` `trait Service { type Data; subscribe(); get(); status(); }`
over `futures_signals::Mutable`, commands as concrete methods (not in the trait) — plus ChronOS's
async template: synchronous `new()` that captures `Handle::current()` and `tokio::spawn`s a
connect+retry loop (`network/mod.rs:61-75`, `upower/mod.rs:62-76`).

**Cost scale (rewrite effort):**
- **S** ≈ ½–1 day — thin/synchronous, no new deps, mechanical `Service` wrap.
- **M** ≈ 1–2 days — real backend, 1 new dep, moderate async port.
- **L** ≈ 2–4 days — complex async orchestration and/or C-FFI deps, threading-model rewrite.
- **XL** ≈ 4–7 days — multi-part (service + UI) plus missing UI infrastructure in ChronOS.

**zbus note:** ChronOS workspace pins `zbus = "5"` (`ChronOS/Cargo.toml:14`); donor uses `5.5.0`.
This is a within-5.x minor bump (effectively already in effect in ChronOS). The `proxy`/`interface`
macros, `Connection::system()/session()`, and `object_server` API are stable across 5.x → **low risk,
negligible churn**. Not a cost driver.

---
## Master table

| Service | Working/Stub | Backend (from code) | Deps to ADD in ChronOS | LOC (donor) | Rewrite cost | Notes / integration point |
|---|---|---|---|---|---|---|
| **applications** | **WORKING** | XDG `.desktop` parse (`parse_desktop_file`, `mod.rs:189-248`) + fork `sh -c` launch (`mod.rs:33-77`) + icon-theme lookup (`icons.rs`). **No D-Bus, no Mutable** (holds plain `Vec<Application>`). | none (std only) | 332 (mod 248 + icons ~84) | **S** | Synchronous scanner, not reactive in donor. Wrap in `Mutable<Vec<…>>` for `Service`. UI already exists in ChronOS (`crates/app/src/launcher/`) — wire `AppState::applications` + rescan-on-change (donor uses no inotify here). |
| **audio** | **WORKING** | **Split:** monitor via **libpulse-binding** (PA subscribe mainloop, `mod.rs:207-392`); commands via **`wpctl` subprocess** (`mod.rs:112,126,139,148,159,176`). "Both" but read=PA / write=wpctl. | **libpulse-binding 2.30.1** (+ optionally pipewire) — C FFI, build-env (pkg-config, libpulse-dev) | 392 | **L** | Uses `Rc<RefCell<…>>` (`mod.rs:8,245-246`) + `thread::spawn` + dedicated PA mainloop — **NOT `Send`**, incompatible with ChronOS `tokio::spawn` + `Service: Send+Sync`. Must re-architect to Arc/Mutex or thread+channel bridge. C-binding build cost. |
| **bluetooth** | **WORKING** | **BlueZ via zbus** (ObjectManager `dbus.rs:181-205`, Adapter `207-234`, Device `236-259`, Battery `261-267`) + **rfkill via inotify** (`mod.rs:146-154`) + discovery auto-stop (`mod.rs:105-118`). | **inotify** (donor `crates/services/Cargo.toml:17`) — NOT in ChronOS services Cargo.toml | 671 (mod 312 + dbus 267 + types 92) | **L** | `thread::spawn` + embedded current-thread tokio `mod.rs:181-209,234-284` must become ChronOS `tokio::spawn` async. Mostly mechanical zbus port; rfkill inotify needs a channel bridge. |
| **brightness** | **WORKING** | **udev** discovery+monitor (`mod.rs:211-220,234-295`) + **systemd-logind D-Bus** `SetBrightness` for writes (`mod.rs:200-208`); sysfs only for reads (`mod.rs:223-231`). | **udev 0.9** — NOT in ChronOS | 295 | **M** | Already uses `tokio::task::spawn_blocking` + `AsyncFd` (`mod.rs:234-295`) → fairly async-friendly; port is moderate. Add udev dep. |
| **mpris** | **WORKING** | **MPRIS2 via zbus** (session bus; `MprisPlayer` proxy `dbus.rs:5-24`; `DBusProxy::name_owner_changed`; per-player property streams + 200ms debounce batch `mod.rs:307-510`). | none (zbus only) | 534 (mod 510 + dbus 24) | **L** | All zbus async; thread+rt at `mod.rs:285-305` → `tokio::spawn`. Complex: dynamic proxy set, `select_all` fan-out, metadata mapping. No new deps. |
| **notification** | **WORKING** | **org.freedesktop.Notifications D-Bus *server*** (`#[interface]` `mod.rs:243-445`) + proxy for close/action. Full daemon: Notify/CloseNotification/GetCapabilities/GetServerInformation, NotificationClosed + ActionInvoked signals, timeout threads, action emission. | none (zbus + chrono, both present) | 576 (single file) | **L** | It's a **server**, not just a client — needs `object_server` live in the runtime (ChronOS services are currently client-only proxies). Action/timeout orchestration is involved. |
| **privacy** | **WORKING** | **PipeWire 0.8** media-node registry (`mod.rs:161-205`) + **inotify** `/dev/video0` (`mod.rs:218-259`) + `/proc` fd scan (`mod.rs:262-289`). | **pipewire 0.8** + **inotify** — both NOT in ChronOS; pipewire is C FFI | 289 | **L** | PipeWire `MainLoop` is blocking, own-thread (`mod.rs:152-205`) → bridging to Mutable/async is non-trivial. C-binding build cost. |
| **sysinfo** | **WORKING** | **sysinfo 0.35.1** crate (`mod.rs:10,224-338`): CPU/mem/swap/temp/disk/network, 2s poll. | **sysinfo 0.35.1** — NOT in ChronOS | 338 | **M** | Trivial port: `thread::spawn`+`sleep` (`mod.rs:187-221`) → `tokio::spawn` + `Interval`. Only new dep is sysinfo. |
| **wallpaper** | **WORKING** | **External `swww`/`swww-daemon` subprocess** (`mod.rs:88-108` start daemon, `111-130` `swww img`). | none (std::process) | 130 | **S** | Spawn process + optimistic `Mutable` update; only command `SetWallpaper`. Trivial. |
| **tray (service)** | **WORKING** | **StatusNotifierItem/Watcher + DBusMenu via zbus** (see Tray detail). | **image** crate (UI side; zbus present) | 884 (mod 555 + dbus 329) | **L** (part of XL) | Watcher server + client fan-out. `thread::spawn`+rt at `mod.rs:367-391` → tokio. See detail. |
| **tray (UI)** | **WORKING** | GPUI icon strip + recursive DBusMenu panel (see Tray detail). | **image** crate (present in donor app) | 654 (mod 638 + config 16) | **L** (part of XL) | Depends on ChronOS **panel system** (MISSING) + `BarWidget` (ChronOS sources it from `chronos_luau`, not donor `ui`). See detail. |

---
## Tray — special focus

### A. Watcher SERVER completeness (`tray/dbus.rs`)
`StatusNotifierWatcher::start_server()` (`dbus.rs:30-108`) owns `org.kde.StatusNotifierWatcher`,
sets up `name_owner_changed` re-emit logic (`dbus.rs:49-100`) and a 30s discovery poll
(`discover_items`, `dbus.rs:110-153`) for pre-existing `StatusNotifierItem` bus names.
The `#[interface]` (`dbus.rs:176-234`) implements:

- **`RegisterStatusNotifierItem`** — **COMPLETE** (`dbus.rs:185-198` → `register_status_notifier_item_manual` `155-173`): reads `header.sender()`, dedupes, emits `status_notifier_item_registered`.
- **`IsStatusNotifierHostRegistered`** (property) — **COMPLETE**, returns `true` (`dbus.rs:207-210`).
- **`RegisteredStatusNotifierItems`** (property) — **COMPLETE** (`dbus.rs:202-205`).
- **Host registration events** — `StatusNotifierHostRegistered` / `StatusNotifierHostUnregistered`
  signals are **defined** (`dbus.rs:229-233`) and the watcher treats *itself* as the host.
- **`RegisterStatusNotifierHost`** (method) — **NO-OP** (`dbus.rs:200`: `fn register_status_notifier_host(&mut self, _service: &str) {}`).
  This is spec-correct KDE behavior (the watcher *is* the host; the shell calls this to announce
  itself and the watcher ignores it). Flag: it's a deliberate stub, not incomplete logic.
- `ProtocolVersion` property present (`dbus.rs:212-215`).

**VERDICT: Watcher server is FUNCTIONAL and essentially complete.** No `TODO`/`unimplemented!`.
The only "stub" is the intentionally-no-op host-registration method. Minor gap: `discover_items`
registers every bus name containing `"StatusNotifierItem"` (broad; could catch unrelated names) — cosmetic.

### B. Client side (`tray/mod.rs`)
- `fetch_tray_data`/`create_tray_item` (`mod.rs:280-364`): enumerates registered items, builds
  `TrayItem` with ARGB→RGBA pixmap or icon-name, title, id, and full `MenuLayout` via `DBusMenu::get_layout`.
- Listener (`mod.rs:394-442`): `status_notifier_item_registered`/`unregistered` streams.
- Per-item fan-out (`mod.rs:444-549`): `icon_pixmap_changed` + `layout_updated` streams merged via `select_all`.
- `dispatch` (`mod.rs:200-277`): `MenuItemClicked` (DBusMenu.event + refresh), `Activate`/`ContextMenu`/`SecondaryActivate`
  (SNI proxy calls), `AboutToShow` (DBusMenu.about_to_show + refresh). **Complete command surface.**

### C. UI-side rendering (`crates/app/src/bar/modules/tray/`)
**VERDICT: NOT just a list — full rendering of icons AND context menus.**
- Icon strip: `render_tray_item` (`mod.rs:93-179`) renders pixmap via `image::RgbaImage`→`RenderImage`
  (`mod.rs:105-113`) with icon-font fallback; left/right/middle click wired (`mod.rs:149-176`).
- **Recursive collapsible menu panel**: `TrayMenuPanel` (`mod.rs:240-274`) + `render_menu_items`
  (`mod.rs:316-389`) recurses over `MenuLayout`, handling separators, checkbox/radio toggle
  indicators (`mod.rs:421-440`), labels, expandable submenus with `AboutToShow` lazy-population
  (`mod.rs:293-309`), and live refresh via `watch` (`mod.rs:259-266`).
- `config.rs` is just `icon_size`.

**BUT** the UI depends on infrastructure ChronOS does **NOT** have:
- `crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel}` (`tray/mod.rs:6`) → **no `panel.rs` in ChronOS** (`crates/app/src` has only `bar/`, `ipc/`, `launcher/`, `lib.rs`, `main.rs`, `plugin_bridge.rs`, `state.rs`).
- ChronOS bar widgets come from `chronos_luau::bar::BarWidget` (`ChronOS/crates/app/src/bar/mod.rs:2`),
  a **Luau-sourced** trait — not the donor's Rust `BarWidget` (`tray/mod.rs:206`). The donor tray
  widget will not drop into ChronOS's bar without reconciling the two `BarWidget` APIs.

**VERDICT: Tray UI is feature-complete in the donor but requires (a) a panel system and (b) a
BarWidget bridge in ChronOS before it can land.**

### D. Tray rewrite cost
**XL.** Service side (~884 LOC) is an L on its own (zbus 5.x compatible, but thread→tokio port +
object_server must live in the runtime + per-item stream fan-out). UI side (~654 LOC) is another L,
but it is **blocked on missing ChronOS UI infra** (panel system + Luau BarWidget bridge) that must
be built first. Combined = XL. Integration: `crates/services/src/tray/` (new) + `AppState::tray`
accessor (donor already has it at `state.rs:242`; ChronOS has none) + new bar widget + new panel system.

---
## Integration points in ChronOS (per service)

For **every** service:
1. New module `ChronOS/crates/services/src/<name>/` implementing `trait Service` (`lib.rs:55`).
2. Register in `lib.rs`: `pub mod <name>;` + add field to `Services` (`lib.rs:20-24`) + `init_all()` (`lib.rs:30-36`).
3. Add `AppState::<name>(cx)` accessor in `crates/app/src/state.rs` (pattern `state.rs:29-41`).

UI modules (gaps in ChronOS — must be built):
- **applications** → `crates/app/src/launcher/` (exists) — wire launcher to `AppState::applications`.
- **audio / brightness / mpris / sysinfo / privacy** → control-center / bar widgets. ChronOS has **no** control center, OSD, or panel system yet.
- **bluetooth** → control center.
- **notification** → notifications module (ChronOS AGENTS.md wants it; no code present).
- **tray** → bar tray widget (`BarWidget` via `chronos_luau`) + **panel system (missing)**.

---
## AGENTS.md claim verification

- **Donor AGENTS.md:** *"compositor auto-detects Hyprland or Niri at runtime."* → **CONFIRMED.**
  `compositor/mod.rs:101-109` `detect_backend()` checks `hyprland::is_available()` then `niri::is_available()`; `CompositorSubscriber::new` (`mod.rs:39-42`) errors if neither.
- **Donor AGENTS.md:** *"Services are global singletons accessed via AppState."* → **CONFIRMED.**
  Donor `crates/app/src/state.rs:168-252`: `AppState` holds `Services` (`impl Global`), with accessors for **all 12** services incl. the 9 gap ones (`audio` `:197`, `bluetooth` `:202`, `brightness` `:207`, `mpris` `:217`, `notification` `:227`, `privacy` `:232`, `sysinfo` `:237`, `tray` `:242`, `wallpaper` `:252`, `applications` `:192`).
- **ChronOS AppState singleton pattern** ("services are singletons via AppState") → **PATTERN EXISTS.**
  `ChronOS/crates/app/src/state.rs:11-41` defines `AppState { services: Services }`, `impl Global`, `init()`, and 3 accessors. The 9 gap accessors do **not** exist yet, but the pattern is present and must be extended — not a false claim, just incomplete wiring.

**No AGENTS.md claim was found to be false.** The only "stub" discovered anywhere is the
intentionally-no-op `register_status_notifier_host` (`tray/dbus.rs:200`), which is spec-correct.

---
## Cross-cutting rewrite risks (apply to most services)

1. **Threading-model mismatch (highest cost driver).** 7 of 9 donor services use `thread::spawn`
   with either a dedicated current-thread tokio runtime (`bluetooth`, `mpris`, `tray`) or a raw
   blocking loop (`audio` PA mainloop, `privacy` PW mainloop, `sysinfo` poll, `wallpaper` spawn).
   ChronOS's contract is `tokio::spawn` inside one runtime + `Service: Send+Sync`
   (`lib.rs:55`). Where donor uses `Rc/RefCell` (`audio/mod.rs:8,245-246`) the struct is **not `Send`**
   and must be reworked (Arc/Mutex or thread+channel).
2. **Missing deps in ChronOS services crate:** `inotify` (bluetooth, privacy), `udev 0.9`
   (brightness), `pipewire 0.8` (privacy), `libpulse-binding` (audio), `sysinfo 0.35.1` (sysinfo),
   `image` (tray UI). `pipewire`/`libpulse` are C FFI → build-env tooling (pkg-config, -dev packages).
3. **D-Bus server role (notification, tray)** needs `object_server` living in the runtime; ChronOS's
   existing services are client-only proxies — new pattern to establish.
4. **UI infra gap.** 8 of 9 services have no ChronOS UI home yet (no control center / panel / OSD /
   notifications module). Tray UI additionally needs a panel system + Luau `BarWidget` bridge.
5. **zbus 5.5→5(.17): low risk** — within 5.x, macro/runtime API stable. Not a cost driver.

---
## One-line summary of costs
S: applications, wallpaper. · M: brightness, sysinfo. · L: audio, bluetooth, mpris, notification, privacy, tray-service, tray-UI. · **XL: tray (service+UI combined, blocked on missing panel/BarWidget infra).**
