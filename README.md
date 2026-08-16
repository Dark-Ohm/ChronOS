<div align="center">

# ChronOS

**A modular, GPU-accelerated Wayland desktop shell for Hyprland — written in Rust on GPUI, scriptable with Luau.**

[![status](https://img.shields.io/badge/status-work%20in%20progress-orange)](#status)
[![platform](https://img.shields.io/badge/platform-Wayland%20%2F%20Hyprland-blue)](#requirements)
[![rust](https://img.shields.io/badge/rust-edition%202024-b7410e)](#building)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](#license)
[![site](https://img.shields.io/badge/site-dark--ohm.github.io%2FChronOS-0ea5e9)](https://dark-ohm.github.io/ChronOS/)

**[Landing page →](https://dark-ohm.github.io/ChronOS/)**

</div>

---

ChronOS is a self-contained desktop shell: bar, dock, launcher, notifications and
on-screen-display, all rendered by [GPUI](https://www.gpui.rs/) (the retained-mode
GPU UI toolkit behind Zed) and driven by a sandboxed [Luau](https://luau.org/)
plugin runtime. It targets [Hyprland](https://hyprland.org/) 0.55.4+ and
[Niri](https://github.com/YaLTeR/niri), aiming for 144 FPS and hot-reload of
config and plugins with **no process restart**.

> **Scope note.** Hyprland's *config layer* moved to Lua in 0.55.0 (hyprlang is
> deprecated); the compositor core stays C++. "Lua-Hyprland" throughout the docs
> refers to that config layer, not a rewrite of the compositor.

## Highlights

- **Native Wayland layer-shell** surfaces (bar / dock / osd / notifications) with
  correct per-monitor placement, exclusive zones and input regions — via a
  project fork of GPUI, [`Chronos-GPUI`](https://github.com/Dark-Ohm/Chronos-GPUI).
- **Service-oriented core** — audio (WirePlumber), network, power (UPower),
  MPRIS, system tray (StatusNotifierItem / DBusMenu), applications index and
  wallpaper control, each behind a uniform `Service` trait with reactive
  subscribers.
- **Luau plugin runtime** (`mlua`, LuauJIT) — hot-reloadable, type-checked at
  load, sandboxed at the boundary. Plugins add modules without recompiling the
  core.
- **Zero-allocation render paths** on hot widgets (the bar repaints every
  second — no per-frame allocation or I/O without a cache).

## Architecture

```
crates/
├── app        chronos — the binary: wires modules, windows, IPC, main loop
│   └── src/{bar,dock,launcher,notifications,osd,tray_menu,ipc,...}
├── services   chronos-services — audio, network, upower, mpris, tray,
│              applications, wallpaper; the Service trait + subscribers
├── luau       chronos-luau — the mlua/LuauJIT plugin runtime & hot-reload
├── ui         chronos-ui — shared GPUI widgets, theming, primitives
└── plugins    Luau plugins (data, not a Rust crate) — e.g. clock/
```

Canonical design lives in [`.chronos-ops/checkpoint/ARCHITECTURE.md`](.chronos-ops/checkpoint/ARCHITECTURE.md); rejected
alternatives and their rationale in [`.chronos-ops/checkpoint/REJECTED.md`](.chronos-ops/checkpoint/REJECTED.md). The
original approved spec is kept as a historical record under
[`.chronos-ops/superpowers/specs/`](.chronos-ops/superpowers/specs/).

## Requirements

- **Compositor:** Hyprland 0.55.4+ (Lua config layer) or Niri.
- **Rust:** a recent toolchain (edition 2024). Dependency policy is
  bleeding-edge — newest versions, no inherited upstream pins.
- **System libraries:** a Wayland session plus the usual GPUI build deps
  (Vulkan loader/ICD, `libwayland`, `libxkbcommon`, `fontconfig`/`freetype`).
- Runtime integrations expect the corresponding D-Bus services to be present
  (WirePlumber, UPower, a notification/tray host, etc.).
- **Build tooling — `mold` linker + `sccache` (both required).**
  `.cargo/config.toml` pins the linker to `mold` (`-fuse-ld=mold`) and
  wraps `rustc` with `sccache` — without `mold` on `PATH`, linking fails
  outright; it is not an optional speed-up here. Install both before your
  first build:
  ```sh
  # Arch/CachyOS
  sudo pacman -S mold sccache
  ```
  (Other distros: install the equivalent `mold`/`sccache` packages, or
  build from source.) `sccache` needs no manual startup — `cargo` invokes
  it as the `rustc` wrapper automatically per `.cargo/config.toml`.

## Building

```sh
# debug
cargo build -p chronos

# release (what UX smokes are run against)
cargo build --release -p chronos
./target/release/chronos
```

**Dev CLI** (recommended day-to-day): install once, then rebuild/start/stop
without remembering paths — see **[`docs/dev-cli.md`](docs/dev-cli.md)**.

```sh
./scripts/install-dev-cli.sh
chronos-rebuild && chronos-stop && chronos-start
```

Run with logging for live debugging on Wayland:

```sh
RUST_LOG=info ./target/release/chronos
# or: chronos-start   → log at ~/.local/state/chronos/chronos.log
```

Tests:

```sh
cargo test --workspace --lib --bins
```

> **Note on verification.** For window/UX code, "compiles + green unit tests" is
> not sufficient — changes are validated with a release binary against a live
> Wayland session (`RUST_LOG=info`, `grim` screenshots). See
> [`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md) for the smoke recipes.

## Status

**Work in progress — everything is subject to revision.** The shell already runs
with a working bar (clock, workspaces, network, volume, MPRIS, tray), dock,
launcher, notifications, OSD, wallpaper control and an SNI/DBusMenu tray. See
[`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md) for the current state and open work.

## Development

Maintained by **[Dark-Ohm](https://github.com/Dark-Ohm)**. Contribution
guidelines live in [`CONTRIBUTING.md`](CONTRIBUTING.md) — read that before
opening a PR.

Changes are developed against numbered task briefs under [`docs/orchestration/tasks/`](docs/orchestration/tasks/) —
every accepted change is reviewed against the running shell before it
lands, not merged on green tests alone. If you're an agent working in this
repo, [`AGENTS.md`](AGENTS.md) is the entry point.

Canonical project knowledge, read in this order when in doubt:
[`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md) (current state) →
[`.chronos-ops/checkpoint/ARCHITECTURE.md`](.chronos-ops/checkpoint/ARCHITECTURE.md) (accepted design) →
[`.chronos-ops/checkpoint/REJECTED.md`](.chronos-ops/checkpoint/REJECTED.md) (what was tried and rejected,
and why).

## License

Licensed under the **Apache License, Version 2.0** — see [`LICENSE`](LICENSE).
Ported and derived code retains its upstream terms and is attributed in
[`NOTICE`](NOTICE) (the GPUI fork, Kael, waytrogen, Alloy). The unlicensed
`reference/` study material is never committed.

## Related projects

- [`Chronos-GPUI`](https://github.com/Dark-Ohm/Chronos-GPUI) — the GPUI fork
  ("gpui-ce chronos edition") this shell builds on.
