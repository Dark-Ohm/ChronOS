# Contributing

Thanks for the interest. ChronOS is maintained by
**[Dark-Ohm](https://github.com/Dark-Ohm)** — bug reports and PRs are
welcome; here's how to make one that lands cleanly.

If you're an AI coding agent working in this repo, read
[`AGENTS.md`](AGENTS.md) instead — it covers the task-ticket workflow and
zone discipline this project uses for agent-assisted development.

## Before you start

- **Read the docs in priority order:** [`.chronos-ops/checkpoint/HANDOFF.md`](.chronos-ops/checkpoint/HANDOFF.md)
  (current state) → [`.chronos-ops/checkpoint/ARCHITECTURE.md`](.chronos-ops/checkpoint/ARCHITECTURE.md) (why
  things are built the way they are) → [`.chronos-ops/checkpoint/REJECTED.md`](.chronos-ops/checkpoint/REJECTED.md)
  (what was tried and rejected, and why). A design question you're about to
  raise may already have a documented answer there.
- **Check [`.chronos-ops/`](.chronos-ops/)** for
  open work before starting something from scratch — someone may already
  be on it, or it may be intentionally deferred.
- **Read those files, don't edit them.** Everything under
  `.chronos-ops/checkpoint/` (plus `.rules` and `CLAUDE.md`) is the
  maintainer's working instrument — a PR touching them is rejected on sight,
  however correct the change. If something there is wrong or out of date,
  say so in the PR description or an issue and let the maintainer fix it.

## Building

See [`README.md#building`](README.md#building) for build commands and
requirements (`mold` + `sccache` are required, not optional).

## Definition of done

For anything that touches a window, a popup, layout, or user input:
**a green `cargo test` is not sufficient.** This project has shipped
visually broken UI behind passing tests more than once. Verify with a
release binary against a live Wayland session before calling it done:

```sh
cargo build --release -p chronos
pkill -x chronos          # not `-f` — matches only this binary by name
RUST_LOG=info ./target/release/chronos
```

Or with the dev CLI (`./scripts/install-dev-cli.sh` once, then):

```sh
chronos-rebuild && chronos-stop && chronos-start
```

See [`docs/guides/dev-cli.md`](docs/guides/dev-cli.md) for the full CLI reference.

## Code style

- **Never swallow a fallible call silently.** `let _ = might_fail()` has
  caused real, hard-to-diagnose bugs here — propagate the error with `?`,
  log it explicitly with `.log_err()` if it's genuinely fine to ignore, or
  handle it with a real `match`.
- **New crates** must opt into the workspace lints: add `[lints] workspace
  = true` to the crate's `Cargo.toml`, or the workspace-level lints (`deny
  unsafe_code`, `warn unwrap_used`/`expect_used`, etc.) silently don't apply.
- **Comments explain *why*, not *what*.** If a comment just restates the
  line below it, delete it.
- **Dependencies are bleeding-edge.** Take the newest version; don't
  inherit a pin from another project just because it's convenient.

## Git

- Commit messages: `area : what changed`, present tense, no fluff.
- No AI-authorship trailers (`Co-Authored-By`, `Assisted-by`, etc.) in
  commit messages, regardless of how the change was produced.
- Stage files by name and read `git diff --staged` before committing —
  especially for files shared across concurrent work. Sweeping an unrelated
  file into your commit with `git add -A` has broken things here before.
- `reference/` (unlicensed third-party study material — see
  [`NOTICE`](NOTICE)) is never committed. Look at it for inspiration, never
  copy from it.

## Skills / documentation proofs

Skill files no longer live in this repository. As of 2026-08-18 they are in
a single ecosystem-wide vault outside git
(`/home/neo/projects/chronos-ecosystem/skills/`), shared by every project,
and the `skill-proofs` CI job and its pre-commit hook were removed with
them. Nothing to run here.

The rule the gate enforced still stands for docs in this repo: a claim
about the code carries a `file:line` reference, or it is a guess.

## Plugins

Luau plugins live under `crates/plugins/<name>/{manifest.toml,init.luau}` —
the plugin id is the directory name.

## Opening a PR

Skim [`.chronos-ops/checkpoint/REJECTED.md`](.chronos-ops/checkpoint/REJECTED.md) first — if your approach
was already considered and rejected, a short PR description explaining why
it's different beats 400 lines that get closed for going against an
existing decision. When in doubt, open an issue with the question before
writing the code.

All contributions are licensed under **Apache-2.0** — see
[`LICENSE`](LICENSE) / [`NOTICE`](NOTICE).
