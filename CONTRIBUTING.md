# Contributing

ChronOS is **[Dark-Ohm](https://github.com/Dark-Ohm)**'s shell — Hyprland/Niri,
Rust, a private GPUI fork, Luau plugins. If you want to change it, you are
talking to a person who ships the product, not to a faceless “AI project.”

Agents (Claude, Codex, whatever) work in this tree under orchestration. They
do **not** own the design. They do not get commit credit. They get tasks,
acceptance, and rejection when they invent green. If you are a human
contributor, the same rules apply — fewer excuses.

Site: [dark-ohm.github.io/ChronOS](https://dark-ohm.github.io/ChronOS/) ·
GPUI fork: [Chronos-GPUI](https://github.com/Dark-Ohm/Chronos-GPUI)

---

## Before you touch code

Read, in order:

1. **`HANDOFF.md`** — what is actually true *today* (queue, blood bugs, field
   rules). Stale chat memory loses to this file.
2. **`ARCHITECTURE.md`** — accepted design.
3. **`DECISIONS.log`** — what we already tried and threw out (so you do not
   re-propose Kael, exclusive keyboard on popups, or the other corpses).
4. **`AGENTS.md` / `MEMORY.md`** — house rules for anyone coding here.

If those four disagree with a blog post, a remembered session, or an agent
brief — the four win.

---

## What “done” means here

`cargo test` green is the floor, not the ceiling.

For bar / dock / launcher / notifications / popups / anything that paints a
layer-shell surface:

- build **release**: `cargo build --release -p chronos`
- kill only the binary: **`pkill -x chronos`** (never `pkill -f` — it eats the
  shell that launched it)
- run live under Hyprland/Niri with `RUST_LOG=info`
- prove it with **grim** (or equivalent), not “looks fine on my machine”

Unit tests do not see ghost windows, grab handshake failures, or DDC queues
that jump brightness for three minutes. Live smoke does.

---

## Code rules that already cost real bugs

- **Do not swallow errors:** no `let _ = fallible_call()`. Use `?`,
  `.log_err()`, or an explicit match. Silent `handle.update` errors once left
  ghost layer-shell windows in the compositor.
- **Workspace lints** (`Cargo.toml`): `unsafe_code = deny`; unwrap/expect are
  warn. New crates: `[lints] workspace = true` or you are outside the fence.
- Comment **why**, not a transcript of what the next line does. Match the file
  you are in.
- `cargo fmt`. Clippy clean on **new** code; do not boil the ocean on old
  unwraps unless that is the task.

Bleeding-edge deps are intentional (CachyOS habit). Do not pin ancient
versions “for safety” when you port foreign code — adapt APIs, document the
exception in `DECISIONS.log` if a bump actually breaks the tree.

---

## Git

- Message: `area : what changed` — short, specific, human.
- **No** `Co-Authored-By`, `Assisted-by`, or any AI trailer. Ever.
- `git add path/to/file` by name. **Never** `git add -A`.
- Read `git diff --staged` before commit. Shared files (`main.rs`,
  `widgets/mod.rs`, `lib.rs`) have been contaminated by parallel WIP more
  than once — if a line is not yours, do not ship it.
- Do not commit `reference/` gpui-shell material (unlicensed). Rewrite by
  pattern, zero copied lines.

---

## Build / run

```sh
cargo build --release -p chronos
cargo test  --workspace --lib --bins
RUST_LOG=info ./target/release/chronos
```

Day-to-day loop — install once, then use the wrappers:

```sh
./scripts/install-dev-cli.sh    # → ~/.local/bin

chronos-rebuild && chronos-stop && chronos-start
chronos-debug                   # debug + hot-reload
chronos-reload                  # hotview dylib only
chronos-stop                    # pkill -x chronos
```

Details: [`docs/dev-cli.md`](docs/dev-cli.md).

---

## Plugins

Luau plugins live under `crates/plugins/` (data, not a Rust crate). Each needs
`manifest.toml` + entry (usually `init.luau`). The `chronos-luau` runtime
hot-reloads them; identity is the **directory path**, not the display name in
the manifest.

---

## Agents in this repo

If you are an automated agent: your brief is a task file under
`orchestration/tasks/`, not a free-form chat fantasy. Report claims get
grepped against the tree. Invented “done” gets rejected. The human maintainer
and the lead architect role decide what lands.

If you are a human and something is unclear — open an issue on
[Dark-Ohm/ChronOS](https://github.com/Dark-Ohm/ChronOS) or ask. Prefer one
honest question over a 400-line PR that redoes architecture we already
killed in `DECISIONS.log`.

---

## License

Apache-2.0. See `LICENSE` and `NOTICE`. By contributing you agree your work
ships under the same terms.
