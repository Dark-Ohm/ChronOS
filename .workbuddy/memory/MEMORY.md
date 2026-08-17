# ChronOS — durable project notes

## Module visibility: the "twin module" pattern
The `chronos` app crate splits into a lib (`crates/app/src/lib.rs`) and a bin
(`crates/app/src/main.rs`) that are SEPARATE crates. A module reachable as
`crate::X` must be declared in whichever crate uses it. Modules like `dock`,
`popup_click_catcher`, `desktop_terminal`, `project_switcher` are declared in
BOTH `lib.rs` (as `pub(crate) mod`) and `main.rs` (as `mod`) so `crate::X`
resolves in either crate. If a lib-side module (e.g. `launcher`) references a
`main.rs`-only module, add the twin declaration to `lib.rs` rather than moving
code.

## gpui-component `Input` / `InputState` gotchas
- In single-line mode `Input` does NOT bind `up`/`down`/`tab`; `enter`/`escape`
  call `cx.propagate()` and bubble to the window root's `on_key_down`. So a
  launcher can keep its up/down/tab/enter/escape handling on the root and
  delegate text editing to the component safely.
- Any window hosting component widgets (Input, PopupMenu) must be rooted in
  `gpui_component::Root::new(view, window, cx)` or widgets panic.
- `input.update(cx, |state, cx| ...)` closure is 2-arg. `InputState` methods that
  need `&mut Window` (e.g. `focus`, `set_placeholder`) must capture `window`
  from the enclosing scope.
- `Input` builder: `Input::new(&entity)`, `.appearance(false)`, `.cleanable(true)`,
  `.text_color()`/`.text_size()` come via the `Styled` trait. `InputState::text()`
  returns `&Rope` → `.to_string()`.

## nucleo relevance score
`FuzzySearch` wraps `nucleo::Nucleo`. `snapshot.matched_items()` returns
`Item { data, matcher_columns }` — NO score field (score is on the private
`Match`). Items come back in score-descending order, so position in the list is
the relevance rank. Encode relevance as `(max_pos - pos) as f32` when you need a
sortable primary key (see `crates/app/src/launcher/search.rs`).

## Checkpoint #18 (2026-08-18)

T299 done — kitchen archive fully role-sorted. `git mv` 170 files from
`docs/orchestration/tasks/{done,report-log,rejected}/` to
`.chronos-ops/{done,reports-log,reject}/<role>/`. Source dirs empty.
Distribution (170): front=121, back=32, qa=12, recon=4, design=1.
Total archive 506 (336 pre-existing + 170 here). README кухни
`.chronos-ops/README.md` claims «334 уже», off by 2 (336 actual). Not
edited — architect's call.

## Checkpoint #17 (2026-08-16)

HEAD `dc6dc38`. T285 OPEN — cold `load_session` must bind ActiveSession. Live fail was our take() on None, not wrong binary.

## Checkpoint #16 (2026-08-16)

T292 accepted `92786c5`. Shell Gamer on right rail above dock. Next T285.

## Checkpoint #15 (2026-08-16)

Executed T288/T289/T291+E/T290-E/T296. HEAD `b2320fa`. Left=AI, right=OS. Display on right rail bottom. Next: T285 or T292.

## Checkpoint #14 (2026-08-15)

Spec-only wave in `docs/orchestration/tasks/active/T288`–`T295` plus T265-A…G. Execute next session. ACP cwd must be project path (process was in `packaging/`). Updates apply via pacman only.

## Chat restore vs ACP (T281/T285)
`restore_project_thread` paints SQLite transcript. `ChatTab::new` still
`create_session`. Hermes starts a new ACP session on every shell restart.
Fix is `load_session` (cwd required) only when the client is in the HashMap —
do not double-paint the transcript. Ticket T285.

## T287-C chrome
Strip inner Sessions rail, entire thread-header, close X. Move Follow to
composer pickers row (`icons/rail-preview.svg`). Do not delete Follow.

## Frecency (T275, committed 89dfd25)
`crates/services/src/applications/frecency.rs`. Global `OnceLock<Mutex<Store>>`,
TOML at `~/.config/chronos/frecency.toml`, 7-day recency half-life
`score = count * 2^(-age_days/7)`. Empty query → frecency primary; typed query →
nucleo relevance primary, frecency secondary. `record_launch(id)`, `flush()` on
close, `cached()`/`now()`/`rank()` exported (`pub mod frecency`).
