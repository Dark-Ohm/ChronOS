---
name: chronos-shell-ipc
description: "Use when touching crates/app/src/ipc/ (Unix socket command bus), sending commands to a running ChronOS shell, adding a new IPC command, or live-smoke-testing the shell without a mouse."
version: 1.0.0
author: Hermes Agent
license: MIT
metadata:
  hermes:
    tags: [chronos, ipc, unix-socket, shell-automation, smoke-test]
    related_skills: [chronos-shell, verification-before-completion, gpui-layer-shell]
---

# ChronOS Shell IPC

## Overview

The ChronOS shell (`package: chronos`) exposes a **single-instance Unix socket
command bus** at `$XDG_RUNTIME_DIR/chronos.sock`. External clients (Hyprland/niri
keybind daemons, scripts, the `chronos-ipc` helper, automated smoke harnesses)
write newline-free ASCII payloads; the primary instance classifies each payload
and dispatches it onto the GPUI app context (`&mut App`). There is **no
request/response** — it is fire-and-forget over a datagram-ish stream read.

Three files implement it (relative to `crates/app/src/ipc/`):

| File | Role |
|---|---|
| `messages.rs` | Payload constants + `is_*`/`parse_*`/`encode_*` classifier API. Pure, fully unit-tested. |
| `service.rs` | `IpcSubscriber` — binds the socket, `acquire_at` single-instance logic, `accept_loop` that classifies raw bytes into per-command `mpsc` channels. |
| `mod.rs` | `IpcSubscriber::start` — `tokio::select!` loop draining the channels with per-command debounce, calling target functions via `cx.update`. |

This skill is the *operational* guide: how to send commands, the full command
list, the exact recipe for adding a new one, and how to live-smoke without a
mouse. For "where the code lives / why", the `chronos-shell` skill and
`docs/ARCHITECTURE.md` are the source of truth.

## When to Use

- Sending a command to a running shell (keybind wiring, script, automation).
- Implementing a **new** IPC command end-to-end (messages → service → mod).
- Debugging "my IPC command does nothing" (debounce, classifier ordering,
  receiver wiring, focus-after-IPC).
- Live-smoke of panel/tab/preview behavior from a sandbox without synthetic clicks.
- Reading the socket path or testing single-instance behavior.

**Don't use for:** the Lua plugin hot-reload path (that's `crates/luau`,
`chronos-shell`), or D-Bus/FDO services (those are `crates/services`, separate
transport). IPC here means *the shell's own socket*, not `niri-ipc` or
`greetd_ipc`.

## The command surface (as of code read 2026-08)

All payloads are matched with `payload.trim()` — surrounding whitespace/newlines
are stripped, so `b"toggle-launcher\n"` works. Matching is a flat `if/else`
chain in `accept_loop` (`service.rs`), evaluated **top to bottom**; first match
wins.

| Payload | Channel → handler | Debounce | Effect |
|---|---|---|---|
| `ping` | `ping` → log | — | Wakes/confirms the primary instance (used for single-instance detection). |
| `toggle-launcher` | `toggle` → `launcher::toggle(cx)` | 200 ms | Open/close OSD launcher (SUPER+SPACE). |
| `toggle-start-menu` | `start_menu_toggle` → `start_menu::toggle(cx)` | 200 ms | Open/close Start menu (tap Super). |
| `toggle-side-panel-left` | `side_panel_toggle` → `side_panel_left::toggle(cx)` | 200 ms | Toggle left agent panel (pinned, no hover-peek). |
| `toggle-side-panel-right` | `side_panel_right_toggle` → `side_panel_right::toggle(cx)` | 200 ms | Toggle right panel. |
| `toggle-theme` | `theme_toggle` → `theme_config::toggle(cx)` | 200 ms | Switch dark/light scheme. |
| `toggle-edit-mode` | `edit_mode_toggle` → `edit_mode::toggle(cx)` | 200 ms | Toggle edit mode. |
| `toggle-workspace-mode` | `workspace_mode` → `workspace_mode::toggle(cx)` | 200 ms | Toggle Developer/Gamer global. |
| `set-workspace-mode:<mode>` | `workspace_mode` → `workspace_mode::set(cx, mode)` | 200 ms | Set mode explicitly; `<mode>` = `developer`/`gamer` (case-insensitive via `WorkspaceMode::parse`). Unknown → ignored. |
| `select-tab:<id>` | `select_tab` → `side_panel_right::select_tab(tab, cx)` | 100 ms | Switch right panel to tab. `<id>` normalized by `PanelTab::parse_id` (`system` ≡ `System`). Unknown → ignored. |
| `preview-target:<abs-path>` | `preview_target` → `side_panel_right::preview_target(path, cx)` | none | Open Preview/Editor tab at file, `PreviewIntent::Edit`. **Must be an absolute path** or it's rejected. |
| `expand-left` | `expand_left` → `side_panel_left::expand_with_composer(cx)` | 200 ms | Open left panel docked + focus composer. |
| `compose-and-send:<text>` | `compose_and_send` → `side_panel_left::compose_and_send(text, cx)` | none | Send arbitrary text to the left composer+agent. Empty/whitespace text → ignored. Bypasses seat focus (for automation). |
| `wallpaper-next` | `wallpaper` → `wallpaper_ctl::next(cx)` | none | Round-robin next wallpaper. |
| `wallpaper-set:<abs-path>` | `wallpaper` → `wallpaper_ctl::set(cx, &path)` | none | Set wallpaper to absolute image path. Relative → rejected. |
| `wallpaper-gallery` | `wallpaper` → `wallpaper_ctl::open_waytrogen_gallery()` | none | Open waytrogen gallery; 3 s delayed resync (see gotchas). |
| `wallpaper-refresh` | `wallpaper` → `wallpaper_ctl::refresh_after_gallery(cx)` | none | Resync wallpaper state after external set. |

Notes from the real code:
- `encode_*` functions exist **only for out-of-tree clients / tests**; they are
  `#[allow(dead_code)]` because nothing in-crate calls them. Don't delete them
  — the IPC contract is external.
- `classify_*` / `parse_*` (e.g. `classify_select_tab`, `parse_preview_target`)
  return `Option<_>`; `None` ⇒ the payload is **silently dropped** (no error,
  no log). If your command "does nothing", confirm the classifier actually
  returns `Some`.
- `preview-target` and `wallpaper-set` both require **absolute paths**; a
  relative or empty path yields `None` → dropped.

## Sending a command (clients)

### Via the `chronos-ipc` helper
Bash script at `packaging/hyprland/chronos-ipc` writes one payload to the
socket. Usage shown in its header; it is the supported CLI for humans.

### Raw socket from a script (the pattern Hyprland keybinds use)
```python
import socket, os
sock = os.path.join(os.environ["XDG_RUNTIME_DIR"], "chronos.sock")
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect(sock)
s.sendall(b"toggle-launcher")   # no newline needed; trim happens server-side
s.close()
```
Equivalent one-liner (used in `hl.bind` for `SUPER+L` etc.):
```bash
python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.path.join(os.environ['XDG_RUNTIME_DIR'],'chronos.sock'));s.sendall(b'toggle-launcher')"
```

### `socat` (if installed)
```bash
echo -n 'expand-left' | socat - UNIX-CONNECT:"$XDG_RUNTIME_DIR/chronos.sock"
```
The live-smoke harness used a python socket because `socat` was absent on the
build host.

### Single-instance contract
`acquire_at` (`service.rs`): if the socket already exists and connects, the new
process writes the `ping` payload and exits as **Secondary** (it does NOT start
the listener). The primary's `ping` channel just logs. So: **a second `chronos`
binary is NOT a restart** — to truly restart, `pkill -x chronos` first
(`pkill -f` is forbidden — it kills the parent shell).

## Adding a NEW IPC command (exact recipe)

Mirror an existing command exactly. Three edits, each with a unit test. The
canonical template is `toggle-launcher` (no payload arg) and `select-tab:<id>`
(prefix + parse).

### 1. `messages.rs` — protocol surface
For a no-arg command:
```rust
pub const MY_CMD_PAYLOAD: &str = "my-cmd";
#[allow(dead_code)] // external clients only; never called in-crate
pub fn encode_my_cmd() -> String { MY_CMD_PAYLOAD.to_string() }
pub fn is_my_cmd(payload: &str) -> bool { payload.trim() == MY_CMD_PAYLOAD }
```
For a `prefix:<arg>` command, add a `classify_my_cmd` returning `Option<Arg>`
(strip the prefix, validate, return `None` on bad/empty input — never panic).
Match the `preview-target` / `select-tab` shape: trim, `strip_prefix`, reject
empty, reject malformed.

Add `#[cfg(test)]` cases: round-trip encode/is, reject-other, and for
classified commands — `Some` on valid, `None` on empty/unknown/relative.

### 2. `service.rs` — channel + classifier
- Declare a receiver type alias (`pub type IpcMyCmdReceiver = mpsc::UnboundedReceiver<Arg>;`).
- In `start_listener`, create `(my_sender, my_receiver) = mpsc::unbounded_channel()`.
- Thread `my_sender` into the `accept_loop` parameter list **and** the
  `start_listener` return tuple (currently 12 channels — keep the tuple ordered).
- In `accept_loop`, add an `else if` branch **in the match order you want**
  (first match wins) calling `my_sender.send(arg)` on a successful classify.

### 3. `mod.rs` — select arm + handler
- Destructure the new receiver in `start`'s `let (..., mut my_receiver, ...) = self.start_listener();`.
- Add a `tokio::select!` arm:
  ```rust
  my_cmd = my_receiver.recv() => {
      if let Some(arg) = my_cmd {
          // toggle-style: debounce 100–200 ms to coalesce repeat binds
          let now = std::time::Instant::now();
          if now.duration_since(last_my_cmd_at) >= std::time::Duration::from_millis(200) {
              last_my_cmd_at = now;
              let _ = cx.update(|cx| crate::my_module::my_handler(arg, cx));
          }
      } else { break; } // channel closed → end loop
  }
  ```
- **Handler must take `&mut App` (or `&App`) only — NOT `Window`.** There is no
  window in the IPC context. This is why `launcher::toggle(cx)` / `side_panel_*::toggle(cx)`
  all take `cx`, never `window`. If your target needs a window, it cannot be
  IPC-driven directly — route through a global/state change the view observes.
- For `compose-and-send` (explicit send, not a toggle) there is **no debounce** —
  follow that shape when repeated sends must all land.

**Completion criteria for a new command:**
- `cargo test -p chronos ipc::` is green (your new `messages.rs` tests + the
  existing suite).
- `cargo build --release -p chronos` succeeds (the 12-tuple in `start_listener`
  and the `accept_loop` signature must stay in sync — a mismatch is a hard
  compile error, not a silent drop).
- Sending the payload live produces the expected effect (verify via the smoke
  recipe below — claims without a live send are unverified).

## Live-smoke IPC without a mouse (sandbox-tested 2026-08)

Agent sessions kill background processes when a command returns (~30 s), so a
bare `./target/release/chronos &` dies. Run the shell as a **transient
systemd unit** instead:
```bash
systemd-run --user --unit=chronos-ipc --collect \
  -E RUST_LOG='info,chronos::ipc=trace,chronos::side_panel_right=trace' \
  -E WAYLAND_DISPLAY=wayland-1 -E DISPLAY=:0 -E XDG_RUNTIME_DIR=/run/user/1000 \
  ./target/release/chronos
journalctl --user -u chronos-ipc --no-pager -f   # logs go to journald, not a file
```
`RUST_LOG` needs the **crate prefix**: `chronos::ipc=trace`, not `ipc=trace`,
or the target `chronos::ipc::...` won't match and traces stay silent.

Send commands into it:
```bash
python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.path.join(os.environ['XDG_RUNTIME_DIR'],'chronos.sock'));s.sendall(b'expand-left')"
```
Watch `journalctl` for `IPC expand-left received` → handler log. Capture the
result with `grim` / `wf-recorder`.

### Focus-after-IPC gotcha (critical for input smoke)
GPUI **layer-shell** windows do not receive keyboard focus from synthetic
clicks (`ydotool`/`wtype`). For tabs that accept text input you must drive
focus through the IPC path:
- `select-tab` / `preview-target` **defer focus 50 ms** via
  `cx.spawn` + `background_executor().timer()` (NOT `tokio::time::timeout`,
  which is inert inside the GPUI executor — it does not advance the GPUI clock).
- Without that deferral, `wtype` after `preview-target` lands nowhere because
  the seat never focuses the layer-shell surface.
- `compose-and-send:<text>` exists precisely to bypass this: it injects text
  into the composer without needing seat focus at all.

### Geometry / capture timing
- `hyprctl layers -j` reports `x/y/w/h` (NOT `width/height`).
- **Mid-animation geometry is garbage.** With `preview-target`, wait **≥2.0 s**
  after the open before measuring (EaseOutBack entry animation). A `sleep 1.3`
  once yielded `x=2520 w=560` on a 2560-wide screen — content still sliding in.
- `wf-recorder` silently dies on geometry past the monitor edge and leaves the
  previous run's file (detect via mtime). `wtype -s` is **milliseconds**
  (`-s 100`), not seconds (`-s 0.1` errors).

## Common Pitfalls

1. **Command silently dropped.** `classify_*`/`parse_*` return `None` on
   bad input and `accept_loop` just falls through — no log. Check: absolute
   path for `preview-target`/`wallpaper-set`, valid tab id for `select-tab`,
   non-empty text for `compose-and-send`.
2. **Second `chronos` is not a restart.** It pings the primary and exits.
   Always `pkill -x chronos` (never `-f`) before launching a fresh instance for
   a smoke run.
3. **Debounce eats rapid repeats.** Toggle commands debounce 100–200 ms.
   A script firing `select-tab` twice in <100 ms loses the second. For forced
   sends, use `compose-and-send` (no debounce) or space your sends.
4. **`tokio::time::timeout` is inert in `cx.spawn`.** IPC handlers run on the
   GPUI executor. Use `cx.background_executor().timer(dur)` for the 50 ms focus
   deferral (see `mod.rs` wallpaper-gallery arm and the focus gotcha above).
5. **`start_listener` return tuple vs `accept_loop` params must match.**
   Adding a channel to one but not the other is a compile error — fix both
   together. Count is currently 12; don't reorder existing entries.
6. **Match order in `accept_loop` is significant.** `set-workspace-mode:<m>`
   is checked *after* `toggle-workspace-mode` because the latter is a plain
   equality; keep prefix commands after their bare siblings, or the bare one
   shadows the prefix.
7. **`preview-target`/`wallpaper-set` reject relative paths.** Always pass an
   absolute path (the harness resolved `PathBuf::from(rest)` and checks
   `is_absolute()`).
8. **Single-instance stale socket.** If the primary crashed without `Drop`,
   the socket file lingers; `acquire_at` removes a stale, non-connectable
   socket before rebinding. If you see "Failed to remove stale socket", the
   path is owned by a still-alive process — `pkill -x chronos` first.

## Verification Checklist

- [ ] Command list above matches the current `accept_loop` (`service.rs`) — no
      command added/removed without this skill being updated.
- [ ] New command (if any): `messages.rs` const+classifier+test, `service.rs`
      channel+tuple+`accept_loop` arm, `mod.rs` receiver+`select!` arm+handler.
- [ ] `cargo test -p chronos ipc::` green.
- [ ] `cargo build --release -p chronos` succeeds (tuple/param arity in sync).
- [ ] Live send produces the effect: `journalctl` shows the `IPC <cmd> received`
      trace, and the UI/state change is observable (grim screenshot or state read).
- [ ] For input-bearing commands: focus deferral present; `wtype` lands text
      after the deferred focus (or `compose-and-send` used to bypass it).
- [ ] No synthetic-click focus assumption: layer-shell surfaces get focus only
      via the IPC `FocusHandle`/defer path, never from `ydotool`/`wtype` clicks.

## One-Shot Recipes

**Restart + smoke a tab switch:**
```bash
pkill -x chronos
systemd-run --user --unit=chronos-ipc --collect -E RUST_LOG='info,chronos::ipc=trace' \
  -E WAYLAND_DISPLAY=wayland-1 -E XDG_RUNTIME_DIR=/run/user/1000 ./target/release/chronos
sleep 2
python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.path.join(os.environ['XDG_RUNTIME_DIR'],'chronos.sock'));s.sendall(b'select-tab:terminal')"
journalctl --user -u chronos-ipc --no-pager | grep 'IPC select-tab'
```

**Send text without seat focus (automation):**
```bash
python3 -c "import socket,os;s=socket.socket(socket.AF_UNIX);s.connect(os.path.join(os.environ['XDG_RUNTIME_DIR'],'chronos.sock'));s.sendall(b'compose-and-send:deploy the widget now')"
```
