# T288 — ACP session cwd is the active project — Report

**Date:** 2026-08-15
**Role:** BACKEND (ACP). Zone: `crates/services/hermes_acp/**` + 3 call sites in `crates/app/.../tabs/chat.rs`.

## Status

**Done (code + unit tests green).** Live release verification deferred — see
"Не сделано / Live".

## Симптом

`chronos-start` does `nohup "$RELEASE_BIN"` with no `cd`, so the shell inherits
the caller's cwd (`…/ChronOS/packaging`). The ACP session + the persisted
`ThreadRecord.cwd` both came from `std::env::current_dir()` → `…/ChronOS/packaging`,
while `project_path` (the active project) pointed at `…/ChronOS`. The two had
diverged, so a New session was created in the wrong directory.

## Contract (implemented)

If an active project is selected → ACP `cwd` and `ThreadRecord.cwd` equal the
project path. Otherwise → `std::env::current_dir()` (pre-T288 behaviour).

## Done

### 1. `crates/services/src/hermes_acp/client.rs`

- `Command::CreateSession { cwd: PathBuf, reply }` — the variant now **carries**
  the resolved cwd instead of re-reading the process cwd on the service side.
  The handler forwards it via `ensure_fresh_session(cx, session, intercepted_models, &cwd)`.
- `start_new_session(cx, cwd: &Path)` — calls `cx.build_session(cwd)`
  (ACP 0.11 SDK: explicit path) instead of `cx.build_session_cwd()` (which is
  literally `std::env::current_dir()`).
- `ensure_fresh_session` now takes `cwd: &Path` and threads it to `start_new_session`.
- The two lazy on-demand paths in `send_prompt_streaming` / `send_prompt_on_active`
  (no session yet when a prompt arrives) fall back to the process cwd — the
  "no active project" branch; services has no access to the shell's project
  scope, so this preserves prior behaviour for that edge path.
- `HermesClient::create_session(&self, cwd: &Path)` — caller resolves the path
  once and passes it in.
- Unit tests (`#[cfg(test)] mod tests`): `create_session_command_carries_cwd`
  (variant carries cwd — compile-time proof of the shape change),
  `start_new_session_uses_build_session_with_cwd` (source-scan: `.build_session(cwd)`
  present, the cwd convenience gone — same split-token gate pattern as
  `chat.rs::chat_tab_source_has_no_window_lifecycle`).

### 2. `crates/app/src/side_panel_left/tabs/chat.rs`

- New pure helper `session_cwd(active_project: Option<&Path>, process_cwd: &Path) -> PathBuf`
  (exactly the brief's spec): non-empty active project wins, else process cwd.
  Single source of truth — `project_path`, `ChatTab::new`, `switch_agent`, and
  `create_new_session` all flow through it instead of each calling
  `current_dir()` independently.
- `project_path(cx)` refactored to delegate to `session_cwd` (was the two-source
  split: global `active_project_path` + inline `current_dir()`).
- **`ChatTab::new`** — reads `SidePanelLeftState_.active_project_path` from the
  global **before** `cx.spawn` (the scope is seeded by
  `restore_active_project_on_startup` during `init`,
  `side_panel_left/mod.rs:870`, so it's already set), resolves `session_cwd`,
  and passes `&session_cwd` to `create_session`. No longer lazily reads
  `current_dir()` inside the async body.
- **`create_new_session`** — `let cwd = self.project_path(cx);` (single source;
  `project = cwd.clone()`). The same `cwd` feeds `insert_for_project`,
  `ThreadRecord.cwd`, and `create_session(Path::new(&cwd))` — they can't drift.
- **`switch_agent`** — captures `PathBuf::from(self.project_path(cx))` before the
  spawn and passes `&session_cwd` to `create_session`.
- Unit tests: `session_cwd_project_some_returns_project`,
  `session_cwd_none_returns_process`, `session_cwd_empty_project_returns_process`
  (project Some → project; None → process; empty → process).

### 3. `crates/services/src/hermes_acp/client_smoke.rs`

- Updated the live (ignored) smoke test: `client.create_session(Path::new("."))`.

## Evidence (commands run)

```
cargo test -p chronos --lib session_cwd
cargo test -p chronos --lib side_panel_left
cargo test -p chronos-services --lib hermes_acp
```

| Command | Result |
|---|---|
| `cargo test -p chronos --lib session_cwd` | 3 passed (`session_cwd_project_some_returns_project`, `session_cwd_none_returns_process`, `session_cwd_empty_project_returns_process`) |
| `cargo test -p chronos --lib side_panel_left` | 113 passed; 0 failed. Existing suite (incl. `chat_tab_source_has_no_window_lifecycle`, `restore_on_startup_*`, `switch_project_sets_path_and_clears_session`) still green. |
| `cargo test -p chronos-services --lib hermes_acp` | 2 passed; 1 ignored (`smoke_hermes_session_reuse_two_prompts` — live, needs `CHRONOS_SMOKE_HERMES_ACP=1`); 0 failed. |

### Isolation proof

Per the commit rules (commit must build by itself, not just "tree builds"), the
would-be commit = `HEAD` + exactly the three staged files was verified in an
isolated `git worktree` (external `Source/gpui` symlinked). `cargo
check -p chronos -p chronos-services` in that worktree passes — the change is
additive and touches no other in-flight files.

`git diff --stat` of the staged commit (only these three files):

```
 crates/app/src/side_panel_left/tabs/chat.rs       |  N ++++
 crates/services/src/hermes_acp/client.rs          |  N ++++
 crates/services/src/hermes_acp/client_smoke.rs    |  2 +-
```

## Что НЕ сделано (выполняет Архитектор / дальше)

1. **Live + release verification (T288 §Верификация).** The unit tests prove the
   wiring and the resolution logic, but per the project rules "компилируется и
   тесты зелёные" для окон/UX ничего не значит — нужен релизный бинарь и живой
   кадр. Не проверял, за архитектором. Required live:
   - стартовать шелл специально из `packaging/`;
   - выбрать ChronOS → в логе `session/new` / Hermes cwd = `…/ChronOS`, не
     `…/ChronOS/packaging`;
   - New session пишет в store тот же path;
   - без active project — как раньше (process cwd).
2. **Point 6 (опционально).** `chronos-start` делает `cd "$HOME"` (или `cd "$REPO"`)
   перед `nohup` — не выполнено. Это опциональный fallback "на всякий случай";
   контракт T288 (п. 1–5) уже выполняет cwd из active project, так что fallback
   не нужен для исполнения контракта. Осталось как defensive hardening, если
   захотим.
3. **T285 (`load_session` on restore).** `load_session` vs `create_session` на
   restore — следующий тикет, после T288. Новый `create_session` пишет правильный
   cwd; старые ряды в SQLite с `cwd=…/packaging` не мигрировать оптом
   (явное "New session" чистит их). T285 load пишет в записанный `record.cwd`;
   после T288 новые сессии чистые.
4. **T286 (composer) / T287-C (chat strip chrome).** Не в зоне.

## Коммит

```
fix(left-panel): ACP session cwd is the active project (T288)
```

(Three files only — see isolation table above.)
