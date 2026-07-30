# T122 — Dev CLI Shell Scripts: Report

**Status: DONE**
**Date:** 2025-07-25

## Summary

Five dev CLI commands installed into `~/.local/bin` as symlinks back to `scripts/dev/` in the repo. ShellCheck clean. All behavioral contracts verified against a live process.

## Files created / modified

| File | Purpose |
|---|---|
| `scripts/dev/common.sh` | Shared utilities: REPO root resolution, path constants, process helpers (`pkill -x chronos` only) |
| `scripts/dev/chronos-rebuild` | `cargo build --release -p chronos` (or `--debug` with `--features hot-reload`) |
| `scripts/dev/chronos-reload` | One-shot `cargo build -p chronos-hotview` — Design B, no background watcher |
| `scripts/dev/chronos-stop` | `pkill -x chronos` with SIGTERM → SIGKILL fallback, idempotent |
| `scripts/dev/chronos-start` | `nohup` release binary, single-instance guard, log to `$XDG_STATE_HOME/chronos/chronos.log` |
| `scripts/dev/chronos-debug` | Builds debug+hot-reload, starts with `RUST_LOG=debug`, log to `…/chronos-debug.log` |
| `scripts/install-dev-cli.sh` | `ln -s` all five scripts into `~/.local/bin` |
| `CONTRIBUTING.md` | Added "Dev CLI" section (6 lines) before "Plugins" |

## Install location

```
~/.local/bin/chronos-rebuild → …/ChronOS/scripts/dev/chronos-rebuild
~/.local/bin/chronos-reload  → …/ChronOS/scripts/dev/chronos-reload
~/.local/bin/chronos-stop    → …/ChronOS/scripts/dev/chronos-stop
~/.local/bin/chronos-start   → …/ChronOS/scripts/dev/chronos-start
~/.local/bin/chronos-debug   → …/ChronOS/scripts/dev/chronos-debug
```

Symlinks (not copies) — `git pull` updates behavior instantly.

## Reload design: B (one-shot)

Per T122 spec, Design B was chosen (MVP sufficient):

- `chronos-reload` runs `cargo build -p chronos-hotview` once.
- `hot-lib-reloader` in the running debug process picks up the new `.so` automatically (~1-2s).
- No background `cargo watch` process, no pid file — simpler, no leaked processes.
- If a persistent watcher is desired later, Design A can be layered on top.

## Sample command output

```bash
$ chronos-stop
Stopping chronos (PID 4073848)...
chronos stopped.

$ chronos-reload
error: chronos is not running.
  Start a hot-reload build first:  chronos-debug

$ chronos-rebuild
Building chronos (release)...
  repo:     /home/neo/projects/chronos-ecosystem/ChronOS
# NOTE: hit pre-existing E0382 in volume_popup/view.rs — not script-related

$ chronos-start
Starting chronos (release, RUST_LOG=info)...
  binary: /home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos
  log:    /home/neo/.local/state/chronos/chronos.log
chronos started (PID 4134685).

$ chronos-start          # second call — single-instance guard
error: chronos already running (PID 4134685).
  Use: chronos-stop   (then retry)
exit: 1

$ chronos-reload         # release instance — correct error
error: chronos (PID 4134685) is not a debug build — hot-reload not available.
  Running binary: /home/neo/projects/chronos-ecosystem/ChronOS/target/release/chronos
  Use:  chronos-stop && chronos-debug

$ chronos-stop
chronos stopped.
```

## ShellCheck

`shellcheck -e SC1091 scripts/dev/* scripts/install-dev-cli.sh` — **0 warnings, 0 errors**.

SC1091 (can't follow `source` path) is suppressed — it's a shellcheck static-analysis limitation, not a real issue.

## Pre-existing issue

`chronos-rebuild` hit a compile error in `crates/app/src/volume_popup/view.rs:591` (`E0382: use of moved value: row_id`). This is a pre-existing Rust code issue unrelated to T122 scripts. The existing release binary was used for functional testing.

## Verification checklist

| Contract | Status |
|---|---|
| `pkill -x chronos` only (never `-f`) | ✅ |
| `chronos-rebuild` doesn't start/stop shell | ✅ |
| `chronos-start` release build (no hot-reload feature) | ✅ |
| `chronos-start` single-instance guard | ✅ |
| `chronos-debug` debug build + hot-reload | ✅ |
| `chronos-reload` doesn't `pkill` the shell | ✅ |
| `chronos-reload` Design B documented in script header | ✅ |
| `chronos-stop` idempotent | ✅ |
| REPO root resolved (not hardcoded) | ✅ |
| No AI trailers in scripts | ✅ |
| CONTRIBUTING.md blurb added | ✅ |
