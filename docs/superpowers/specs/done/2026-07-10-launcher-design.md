# Launcher Module — Design Spec

**Date:** 2026-07-10
**Status:** APPROVED (brainstorming complete)
**Scope:** `crates/app/src/launcher/` — new module in existing app crate

---

## 1. Goal

A keybind-triggered overlay (analogous to rofi/wofi) that provides fuzzy search
over installed applications via XDG `.desktop` files, launches them detached
from the shell process, and closes on selection or Escape.

**Success criteria:**
- Open via keybind or IPC toggle → instantly visible (no parse-on-open latency)
- Type to fuzzy-search → results update per keystroke
- Select via Enter or click → application launches detached, launcher closes
- Escape → launcher closes, nothing launches
- Killed chronos → launched application survives

---

## 2. Architecture

### 2.1 GPUI entity — new layer-shell surface

Per `ARCHITECTURE.md §4`, overlay layer is reserved for launcher/notifications/osd.
The launcher opens as a **centered overlay window** on the focused display:

```rust
WindowOptions {
    kind: WindowKind::LayerShell(LayerShellOptions {
        namespace: "launcher".to_string(),
        layer: Layer::Overlay,
        anchor: Anchor::empty(),  // centered, not anchored to edges
        exclusive_zone: Some(px(0.)),  // does NOT reserve screen space
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        ..Default::default()
    }),
    window_background: WindowBackgroundAppearance::Transparent,
    // ... bounds calculated to center a ~600×400 px rectangle on display
}
```

`keyboard_interactivity: Exclusive` means all keyboard input goes to the
launcher while it is open — same model as rofi. This is distinct from the bar
(`KeyboardInteractivity::None`).

### 2.2 Module structure

```
crates/app/src/launcher/
├── mod.rs          — init(), open/close toggle, window_options()
├── entry.rs        — DesktopEntry struct + parser
├── cache.rs        — startup parse + inotify watcher (reuses watcher.rs pattern)
├── search.rs       — nucleo wrapper (init, update pattern, collect results)
├── view.rs         — Render impl: search input + result list
└── launch.rs       — process spawn with setsid + detached stdio
```

All in `crates/app` (not a new crate) — same pattern as `crates/app/src/bar/`.

### 2.3 AppState integration

Desktop entry cache lives in a new global, `DesktopEntryCache`, registered
during `init()` alongside `BarWidgetRegistry`. This follows the existing
pattern: `cx.set_global(...)` at init time, `cx.global::<T>()` in render.

```rust
#[derive(Clone)]
pub struct DesktopEntryCache {
    entries: Vec<DesktopEntry>,
}
impl Global for DesktopEntryCache {}
```

Ownership model: `DesktopEntryCache` is a `Global` (not wrapped in `Mutable`)
because it is mutated **only** from the GPUI foreground thread — either at
startup (in `init()`) or from the inotify debounce task (which calls
`cx.update_global::<DesktopEntryCache, _>(|cache, _cx| { *cache = new_cache; })`).
No signal subscription needed; the view reads `cx.global::<DesktopEntryCache>()`
during its `Render::render()` call. This is the same ownership shape as
`BarWidgetRegistry` — a global mutated from the foreground thread, read during
render.

---

## 3. Components

### 3.1 DesktopEntry (`entry.rs`)

```rust
pub struct DesktopEntry {
    pub id: String,          // filename without .desktop extension
    pub name: String,        // resolved Name (see locale fallback)
    pub exec: String,        // raw Exec= value (field codes still present)
    pub icon: Option<String>,
    pub terminal: bool,      // Terminal=true
    pub no_display: bool,    // NoDisplay=true → hide from launcher
}
```

**Parsing rules** (XDG desktop entry spec):
- Required fields: `Type=Application`, `Name=`, `Exec=`
- Skip entries where `Type != Application` or `NoDisplay=true`
- Strip field codes (`%f`, `%F`, `%u`, `%U`, `%d`, `%D`, `%n`, `%N`, `%i`,
  `%c`, `%k`, `%v`, `%m`) from `Exec=` before launch — the launcher never
  passes file arguments
- `Name` resolution: try `Name[LANG]=` (using `$LANG` without encoding suffix,
  e.g. `ru` from `ru_UF.UTF-8`), fall back to `Name=`, fall back to filename
  sans extension

### 3.2 Cache + watcher (`cache.rs`)

**Startup parse:** scan two directories in order:
1. `/usr/share/applications/` (system)
2. `~/.local/share/applications/` (user override)

**Deduplication:** if a user-dir entry has the same filename as a system-dir
entry, the user version wins (XDG spec: user overrides system when filename
matches). Implemented as a single `HashMap<String, DesktopEntry>` keyed by
filename, populated system-first then user-overwrites.

**inotify watcher:** reuses the pattern from `crates/luau/src/watcher.rs`:
- OS thread does blocking `read_events_blocking`
- Sends affected-path batches through `mpsc::unbounded_channel`
- GPUI foreground task runs trailing debounce (300ms, same constant)
- On debounce flush: re-scan affected dirs, replace cache global

Watching directories: `/usr/share/applications/` and
`~/.local/share/applications/` (if it exists). Flat — no recursive watch
needed (`.desktop` files are not in subdirs by XDG convention).

### 3.3 Fuzzy search (`search.rs`)

Dependency: `nucleo 0.5.0` (MPL-2.0, helix-editor org, crates.io verified
2026-07-10).

```rust
pub struct FuzzySearch {
    nucleo: nucleo::Nucleo<u32>,
    items: Vec<DesktopEntry>,
}
```

- `nucleo::Nucleo::new(Config::DEFAULT)` — default config is fine, no
  custom scoring needed for launcher use case
- On each keystroke: call `nucleo.pattern.reparse(0, pattern, CaseMatching::Smart)`
- Collect top 20 matches via `nucleo.matches()`, return `Vec<&DesktopEntry>`
- Pattern update is non-blocking; nucleo runs a background rayon threadpool

### 3.4 View (`view.rs`)

**Layout (top to bottom):**
```
┌──────────────────────────────┐
│  🔍 [search input field]     │  ← focus here on open
├──────────────────────────────┤
│  > Firefox                   │  ← selected (highlight)
│    Thunderbird               │
│    Files                     │
│    Terminal                  │
│  ...                         │
└──────────────────────────────┘
```

**Keyboard handling:**
- Every printable character → append to search pattern, re-filter
- Backspace → remove last char, re-filter
- `ArrowDown` / `Tab` → move selection down
- `ArrowUp` → move selection up
- `Enter` → launch selected, close launcher
- `Escape` → close launcher, nothing launches

**Rendering:** bare `div()`-based flex column. 5–20 results max (filtered),
no virtual scrolling needed. Each result row is a `div()` with text + optional
icon.

### 3.5 Launch mechanism (`launch.rs`)

**Critical requirement:** launched applications must survive chronos restart/crash.

```rust
use std::process::{Command, Stdio};

pub fn launch(exec: &str) -> anyhow::Result<()> {
    // Strip field codes (already done at parse time, but defensive)
    let clean = strip_field_codes(exec);

    Command::new("setsid")
        .arg("sh")
        .arg("-c")
        .arg(&clean)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .context("failed to launch application")?;

    Ok(())
}
```

**Why `setsid`:** creates a new session (SID) for the child, detaching it from
chronos's controlling terminal. Without this, if chronos is killed, the child
process may receive SIGHUP on the controlling terminal (if any) or inherit
chronos's stdio file descriptors — leading to orphaned processes writing to
closed pipes, or processes that die with the shell.

**Why `Stdio::null()` on all three fds:** launched GUI applications have no
use for stdin/stdout/stderr when spawned from a launcher. Redirecting to null
prevents:
- stdout/stderr leaking into chronos's log output
- Broken pipe errors if chronos dies while child still holds the fd

**Acceptance criteria:**
1. Launch an application via launcher
2. Kill chronos (`kill -9 <pid>`)
3. Verify launched application is still running (`ps aux | grep <app>`)

---

## 4. Data Flow

```
Startup:
  main.rs → launcher::init(cx)
    → parse all .desktop files → DesktopEntryCache { entries }
    → cx.set_global(cache)
    → start inotify watcher (background thread → channel → debounce → replace cache)

Keybind / IPC toggle:
  bar widget or IPC → launcher::toggle(cx)
    → if closed: open layer-shell overlay window
    → if open: close it

User types:
  view.on_keystroke → search.update(pattern) → nucleo reparse
    → collect top 20 → view re-renders result list

User presses Enter:
  view.on_enter → launch(selected.exec)
    → Command::new("setsid")...spawn()
    → close launcher window

User presses Escape:
  view.on_escape → close launcher window
```

---

## 5. Error Handling

| Failure | Behavior |
|---|---|
| `.desktop` parse error (malformed file) | Skip entry, log warning, continue |
| `~/.local/share/applications/` doesn't exist | Skip user dir, system dir still scanned |
| inotify init fails | Log error, cache stays at startup snapshot (no live updates) |
| `nucleo` pattern returns 0 results | Show "No results" message |
| `setsid` not found | `Command::new("setsid")` returns `Err` → show error toast, launcher stays open |
| Launch `spawn()` fails | Log error, show error toast, launcher stays open |
| No displays found | Skip launcher init (same as bar) |

---

## 6. IPC Integration

Launcher responds to existing IPC service (`crates/app/src/ipc/service.rs`):

- New message type `ToggleLauncher` → calls `launcher::toggle(cx)`
- This enables external keybind daemons (Hyprland bind, niri binding) to
  trigger the launcher via IPC, same pattern as bar toggle

---

## 7. Testing

### Unit tests
- `entry.rs`: parse minimal valid `.desktop` content, verify field extraction
- `entry.rs`: parse localized `Name[ru]=` with `$LANG=ru_UF.UTF-8`
- `entry.rs`: field code stripping from `Exec=`
- `entry.rs`: skip `NoDisplay=true` and `Type != Application`
- `cache.rs`: dedup — user override system when same filename
- `search.rs`: nucleo pattern match — "ffx" matches "Firefox"
- `search.rs`: empty pattern returns all entries
- `launch.rs`: strip_field_codes correctness

### Integration tests
- Parse real `.desktop` files from system (if available in CI, otherwise mock)
- Toggle open/close lifecycle (requires GPUI test harness)

### Manual smoke test (acceptance)
- `cargo run -p chronos` → keybind → launcher opens
- Type "fire" → Firefox appears
- Enter → Firefox launches, launcher closes
- `kill -9` chronos → Firefox still alive
- Restart chronos → type "fire" → results appear instantly (cache warm)

---

## 8. Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `nucleo` | `0.5.0` | Fuzzy matching engine |
| `inotify` | workspace | Already in `crates/luau` — add to `crates/app` deps |
| `dirs` | workspace | Already in `crates/app` — `dirs::data_local_dir()` for `~/.local/share` |

No new crates beyond `nucleo`. `inotify` and `dirs` are already workspace
dependencies used by other crates.

---

## 9. Out of Scope (YAGNI)

- **PATH-scan fallback** for binaries without `.desktop` files — add when a
  concrete consumer appears
- **`crates/ui` List component** — 15 filtered results don't need virtual scrolling
- **Icon rendering** — `.desktop` `Icon=` field parsed and stored, but actual
  icon loading/rendering is a follow-up (requires SVG/PNG → GPUI image, non-trivial)
- **`LauncherView` trait integration** — module works standalone; trait
  integration is a follow-up per `ARCHITECTURE.md §6`
- **Multi-monitor** — launcher opens on focused display (same as bar); per-display
  launcher is a follow-up
- **Categories / filtering** — single flat list; category filtering is YAGNI
- **Recent / frecency** — no history tracking in MVP
