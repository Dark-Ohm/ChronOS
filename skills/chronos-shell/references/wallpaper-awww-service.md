# Wallpaper service — awww CLI contract & daemon facts

Captured 2026-07-17 while building `crates/services/src/wallpaper/` (HERMES №8).
Backend is **awww** (a maintained swww fork), NOT swww/hyprpaper.
Corrected 2026-07-17 after acceptance: the `awww query` line format has a
LEADING `": "` that an early invented fixture omitted — the fix and the
real captured output are below.

## Binaries (present on this host)
- `/usr/bin/awww` — client (`img`, `query`, `clear`, `kill`).
- `/usr/bin/awww-daemon` — the wallpaper daemon (v0.12.1).

## Daemon bootstrap (idempotent)
1. `pidof awww-daemon` → if alive, skip.
2. Else `Command::new("awww-daemon").stdin/stdout/stderr(Stdio::null()).spawn()`.
3. Wait for the socket by RETRYING `awww query` (bounded loop, NOT a blind
   `sleep`). Socket path: `/run/user/1000/wayland-1-awww-daemon.sock`
   (matches `$WAYLAND_DISPLAY`, e.g. `wayland-1`).
4. If it never comes up → set `ServiceStatus::Degraded`.

**CRITICAL — daemon needs a live Wayland compositor.** In a headless session
(no display server) `awww-daemon` starts then immediately exits:
```
[WARN]  We failed to find wayland buffer with id: 11. This should be impossible.
[INFO]  Removed socket at /run/user/1000/wayland-1-awww-daemon.sock
[INFO]  Goodbye!
```
and `awww query` then reports `Error: "Socket file '...' not found."`.
So the LIVE apply-smoke (actually changing wallpaper) is impossible without a
real graphical session — same class of limit the HANDOFF notes for GUI/display
smokes ("UX-смоки ТОЛЬКО release; gpui-оконный код — только живой прогон").
The smoke example must `exit 1` cleanly on zero-result; it cannot verify the
pixel change here.

## `awww img` (set wallpaper)
```
awww img --resize <crop|no|fit|stretch>
         [--fill-color <#rrggbb>]
         [--outputs <MONITOR>]
         [--filter <Nearest|Billinear|...>]
         [--transition-type <none|simple|fade|...>]
         [--transition-step <ms>] [--transition-duration <ms>]
         [--transition-fps <n>] [--transition-bezier <..>]
         <path>
```
MVP maps `WallpaperCommand::Set { path, monitor, transition }` to:
`["img", "--resize", "crop", (if monitor) "--outputs", mon, (if transition) "--transition-type", t, path]`.
Donor `reference/waytrogen-main/src/changers/awww.rs` has the full enum→string
map if more flags are wanted later.

## `awww query` (read current) — output format (REAL awww 0.12.1, captured live)
One line per output. CRITICAL: every line has a LEADING `": "` before the
output name — do NOT omit it in fixtures. An invented `eDP-1: ...` fixture
passes unit tests but breaks the live smoke, because `split_once(':')` then
yields an EMPTY output name → the parse silently drops the monitor and the
smoke fails verification even though the wallpaper was actually set.
```
: HDMI-A-1: 1920x1200, scale: 1, currently displaying: color: 000000
: DP-1: 2560x1440, scale: 1, currently displaying: image: /tmp/chronos-wallpaper-smoke.png
```
Parse rule (pure fn `parse_query`, unit-tested on the EXACT live strings above):
1. `line.trim_start_matches([':', ' '])` FIRST — strips the leading `: ` so the
   output name parses correctly.
2. Split on the first `:` → output name (before) / rest (after).
3. Skip lines without `currently displaying`.
4. Locate `currently displaying: image: ` via `rest.find(...)` (it sits
   MID-LINE, so `strip_prefix` is WRONG — it only matches at the start).
   The substring after it is the per-monitor path (`.trim()`).
5. `color: RRGGBB` monitors → NO image; do NOT treat the hex as a path
   (skip them; `per_monitor` stays empty for that output, no panic).
`current` = first output's image path (if any). `per_monitor` = map of
output-name → image path (only image monitors land here).

## Donor: `reference/waytrogen-main` (LEGAL distinction!)
- waytrogen is **Unlicense (public domain)** — code MAY be copied line-by-line.
  This is DIFFERENT from `reference/gpui-shell-main` which is all-rights-reserved
  (clean-room only, no copy). Still, only the `reference/` checkout is in
  `.gitignore` — do NOT commit the donor checkout. Add a `Source/NOTICE`
  attribution line as good practice (done for the wallpaper service).
- Take the CLI contract + daemon-bootstrap shape; do NOT pull the iced GUI or
  sqlite history. NOTE: `Source/NOTICE` lives in the SIBLING `Source/` repo, NOT
  `Chronos/Source/` — creating a `Chronos/Source/` directory is wrong; the
  Architect placed the attribution in the real `../Source` repo (separate
  commit). Don't re-create a local `Source/` under ChronOS.

## Live smoke recipe (`examples/wallpaper-smoke.rs`)
- `tracing_subscriber::fmt().init()` is MANDATORY (a blind smoke is worthless).
- Generate the test image with `magick -size 64x64 xc:Navy /tmp/chronos-wallpaper-smoke.png`
  — NEVER write into `~/Pictures`.
- Capture `awww query` BEFORE (user's current wallpaper) → set on ONE monitor
  (first `per_monitor` key, else `"DP-1"`) → confirm via `awww query` →
  restore the captured wallpaper (or `awww clear` if none).
- Must `exit 1` on any failed assertion (zero-result failure criterion).
- ALWAYS: spawn daemon with null stdio + run `pkill -f awww-daemon` in cleanup
  + an internal `tokio::time::timeout` so the example can never hang (an orphan
  daemon child wedges the terminal session — see SKILL.md Verification pitfall).
