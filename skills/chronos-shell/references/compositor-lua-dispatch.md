# Compositor dispatch under Lua-Hyprland (hyprland-rs is broken)

Verified 2026-07-17. Condensed knowledge bank for any future edit to
`crates/services/src/compositor/hyprland.rs` `execute_command` (or any
compositor command path).

## The bug (why `hyprland-rs` dispatchers silently do nothing)

- `hyprland = "0.4.0-beta.3"` in this repo. Its `hyprland::dispatch::Dispatch::call`
  writes the **classic** socket form, e.g. `dispatch workspace 4`.
- Lua-Hyprland **0.55.4+** wraps EVERYTHING read from the control socket in Lua.
  The classic string is evaluated as `return hl.dispatch(workspace 4)` and fails
  server-side:
  `error: [string "return hl.dispatch(workspace 4)"]: ')' expected near '4'`.
- **Reading** (events / `Workspaces::get()` / `Monitors::get()` via hyprland-rs)
  still works — only **dispatchers** (write path) silently no-op. The `Result`
  comes back `Ok` (the socket write succeeded) but the action never happens.
- Symptom in practice: a bar widget calls `CompositorSubscriber::dispatch(
  CompositorCommand::FocusWorkspace(id))` and nothing happens — no error, no
  workspace switch.

## The fix (write the Lua form straight to the unix socket)

Do NOT use `hyprland::dispatch`. Build the Lua dispatcher table yourself and
write a `/dispatch <lua>\n` line to the control socket:

```
$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock
```

Sync, **no tokio** — the compositor service runs its listener on a plain
`std::thread` (sync-thread model, spec §5.2); `std::os::unix::net::UnixStream`
is correct and avoids the zbus "no reactor running" panic class.

Reference implementation shape (already in `hyprland.rs`):

```rust
fn command_to_socket_line(cmd: &CompositorCommand) -> String {
    match cmd {
        CompositorCommand::FocusWorkspace(id) =>
            format!("hl.dsp.focus({{ workspace = {id} }})"),
        CompositorCommand::NextWorkspace =>
            "hl.dsp.focus({ workspace = \"+1\" })".to_string(),
        CompositorCommand::PrevWorkspace =>
            "hl.dsp.focus({ workspace = \"-1\" })".to_string(),
        CompositorCommand::MoveToWorkspace(id) =>
            format!("hl.dsp.move({{ workspace = {id} }})"),
    }
}

fn send_dispatch(line: &str) -> Result<()> {
    let path = socket_path()?; // $XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock
    let mut s = std::os::unix::net::UnixStream::connect(&path)?;
    s.write_all(format!("/dispatch {line}\n").as_bytes())?;
    Ok(())
}
```

## Lua dispatcher syntax (verified against wiki.hypr.land §Dispatchers / §Workspace selectors)

- `hl.dsp.focus({ workspace = N })` — N is a **number** (workspace ID).
- Relative navigation uses **Lua strings**: `hl.dsp.focus({ workspace = "+1" })`
  / `({ workspace = "-1" })`. Relative IDs in the workspace-selector grammar are
  `+1` / `-3` etc. (NOT numbers).
- `hl.dsp.move({ workspace = N })` — moves the **active** window to workspace N
  (`follow` defaults false — matches the old `MoveToWorkspace(id, None)`).
- Other selectors also valid as strings: `name:Web`, `previous`,
  `m+1` (relative on monitor), `e+1` (relative open), etc.

## Verification (headless-friendly)

- Keep `command_to_socket_line` **pure** (no I/O). Unit-test the exact string
  for every `CompositorCommand` variant — runs without a running compositor:
  `cargo test -p chronos-services --lib compositor::hyprland`.
- The live socket write can't run headless (no Hyprland session). Verify the
  click path with a real session + `hyprctl` / screenshot, OR trust the
  Architect's live smoke. Document which you did.

## DON'T regress

- Keep `hyprland` crate import for **reading only** (`data`, `event_listener`,
  `prelude`). Remove `use hyprland::dispatch::{...}` once you stop calling
  `Dispatch::call`.
- Don't "fix" the dispatch into an async/tokio task — the service is sync by
  design (see SKILL.md "Service trait" + "Runtime split").
