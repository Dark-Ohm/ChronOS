# Notifications History Popup — Anchored Redesign Implementation Plan

> **For agentic workers:** REQUIRED workflow: **superpowers:subagent-driven-development**
> (recommended: one fresh implementer per task + task review) **or**
> **superpowers:executing-plans** (single session, sequential tasks).
> Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **ChronOS-specific:** Architect does **not** spawn Claude/Grok subagents.
> "Subagent" / "implementer" = a **local minion** assigned via
> `orchestration/tasks/active/T120-*.md` (agent-agnostic T-brief — **no
> personal names in the brief**). Orchestration rules: self-contained task
> file, report → `orchestration/tasks/report/T120-*-report.md`, named
> `git add` only, no AI commit trailers. Architect accepts after greps /
> diffs / build / live smoke.

**Goal:** Bring `notifications/history_popup` (bar bell inbox) to the same
popup discipline as post-T117 `updates_popup`: anchored to the bell,
real scroll, pixel-faithful chrome from `design/Notifications Popup.dc.html`,
plus history mutations the mockup assumes (per-item dismiss, Clear all).

**Architecture:** Reuse the proven updates pattern — `canvas` bounds on the
bell → `on_mouse_down` → `history_popup::toggle(anchor_rect, parent, …)` →
`WindowKind::AnchoredPopup` with LayerShell fallback. View drops the old
header/✕ chrome; list is urgency-striped cards + optional actions + footer
Clear all. Service gains history-only commands (ephemeral `Close`/`DismissAll`
must not be overloaded).

**Tech Stack:** GPUI fork (`../Source/gpui`), existing
`crates/app/src/notifications/history_popup/`,
`crates/app/src/bar/widgets/notification_bell.rs`,
`crates/services/src/notification/`, `chronos_ui::Theme`.

**Design canon (literal):** `design/Notifications Popup.dc.html` (dark only).
**Popup discipline canon:** `docs/superpowers/plans/2026-07-24-updates-popup-anchored-redesign.md`
+ live code in `crates/app/src/updates_popup/` and `bar/widgets/updates.rs`.

---

## Global Constraints

- **Scope = history popup only.** Do **not** redesign the ephemeral toast
  stack (`notifications/view.rs` / `notifications/mod.rs` layer-shell for
  live notifications) except shared card helpers if strictly needed for
  history visuals. If shared card changes break the toast stack layout,
  prefer a history-local card renderer over a risky shared rewrite.
- **Do not touch:** `volume_popup`, `system_popup`, `tray_menu`,
  `updates_popup` (except reading as reference), side panels, T115.
- **Anchor pairing (proven):** `PopupAnchor::BottomRight` +
  `PopupGravity::BottomLeft`, `constraint_adjustment: SLIDE_X | FLIP_X`,
  `offset: point(px(0.), px(4.))`, `grab: true` — copy from
  `updates_popup/mod.rs` `window_options`, not invent new geometry.
- **Bell wrapper must be `.relative()`** around canvas + hit target
  (T117 lesson: missing relative → wrong bounds → ghost/no-open).
- **Trigger = `on_mouse_down(Left)`, not `on_click`** (grab-popup rule,
  skill `anchored-popups`).
- **Dismiss:** bar re-toggle, per-item ✕, Clear all. Mockup has **no**
  panel header ✕ — do not re-add it. Explicit dismiss only (no focus-loss
  close) — same as updates/history today.
- **Open still calls `MarkAllRead`** (bell unread clears on open).
- **Colors:** prefer `Theme` tokens where they map; for mockup-only hexes
  (`#007acc` Clear all border, urgency strip exacts if tokens differ)
  literals are OK in history view for this pass. Dark mockup is the
  acceptance target; light is nice-if-tokens-work, not a separate theme
  project.
- **Width 360px** (mockup). List max height: keep a hard budget (~380–500px
  or `MAX_LIST_H` style constant) + **real** `overflow_y_scroll` +
  `ScrollHandle` / `.id("notif-history-list")` — never only
  `overflow_hidden` clip without scroll.
- **Footer Clear all:** mockup shows when `list.len() > 1`. Keep that rule
  (or `>= 1` only if Clear all with one item is clearly better — document
  choice; default **mockup: `> 1`**).
- **`unsafe_code = deny`.** No new `.unwrap()`/`.expect()` in production
  paths. No `let _ = fallible` — use `?`, `.log_err()`, or explicit match.
- **Commits:** short `area : what`, no AI trailers, named `git add` only.
  Prefer one commit per Task (1–4); Task 5 is verification-only unless
  tiny errata.
- **Live UX mandatory** for Task 5. Unit green ≠ done for window/UX.
- **Reject conditions (architect will reject):**
  - Report claims done without live grim / notify-send evidence
  - Fabricated tests that don't exercise real command paths
  - Anchored path without fallback `PopupNotSupportedError`
  - History clear implemented by calling `DismissAll` / `Close` only
  - Personal agent names as the only deliverable path (task is T120)

---

### Task 1: History mutations in notification service

**Files:**
- Modify: `crates/services/src/notification/types.rs`
- Modify: `crates/services/src/notification/mod.rs` (`NotificationCommand`,
  `dispatch`, tests)
- Test: existing `#[cfg(test)]` in `mod.rs` (extend; do not invent fake
  crate names)

**Interfaces produced for later tasks:**
```rust
// NotificationCommand additions (names may match these):
RemoveFromHistory(u32),
ClearHistory,
```
- `NotificationState::remove_from_history(id) -> bool` (or equivalent)
- `NotificationState::clear_history()` — clears `history` vec; decide
  unread: prefer `unread = 0` on clear (inbox empty ⇒ no badge)
- Does **not** emit FDO `NotificationClosed` for history-only removes
  unless a matching live notification still exists — history is an
  in-session log, not the live queue. If `id` is also in
  `notifications` (live), you may leave live as-is OR close live too —
  **document choice; default: history-only, live toast independent**.

- [ ] **Step 1: Failing tests**

Add tests roughly:
- `remove_from_history_drops_one_keeps_others`
- `remove_from_history_missing_id_is_noop` (Ok / false, no panic)
- `clear_history_empties_and_clears_unread`
- `dismiss_all_does_not_clear_history` (regression: ephemeral still separate)

- [ ] **Step 2: Implement pure helpers on `NotificationState`**

- [ ] **Step 3: Wire `NotificationCommand` + `dispatch`**

- [ ] **Step 4: Run**
```bash
cargo test -p chronos-services --lib notification -- --nocapture
```
Expected: green, including new tests.

- [ ] **Step 5: Commit**
```
services/notification : history remove + clear commands
```

---

### Task 2: Bell widget — bounds capture + mouse-down toggle

**Files:**
- Modify: `crates/app/src/bar/widgets/notification_bell.rs`
- Modify: `crates/app/src/notifications/history_popup/mod.rs` (signatures
  only: `open`/`toggle` accept `anchor_rect` + `parent`; body can still
  use old LayerShell until Task 3 if split carefully — **prefer Task 2
  only changes bell + signatures that still compile with temporary
  `_anchor` unused only if Task 3 is immediate next; better implement
  signature + pass-through together in Task 3**. **Recommended:** Task 2
  = bell mirror of `updates.rs` **and** history_popup signature change
  with **full** anchored open in Task 3. If splitting is painful, merge
  Task 2+3 into one commit — still mark both checklists done.

**Reference:** `crates/app/src/bar/widgets/updates.rs` (canvas +
`on_mouse_down` + `Rc<Cell<Bounds<Pixels>>>` pattern — copy structure).

- [ ] **Step 1: Widget holds bounds cell** (struct field, not local that dies)

- [ ] **Step 2: Wrapper `.relative()` + `canvas(...).absolute().size_full()`**

- [ ] **Step 3: Replace `on_click` with `on_mouse_down(MouseButton::Left, …)`**
  calling `history_popup::toggle(anchor_rect, parent, window, cx)`.

- [ ] **Step 4: Unit tests for `describe` stay green**
```bash
cargo test -p chronos --lib notification_bell -- --nocapture
```
(or the actual test module path — use real filter from tree)

- [ ] **Step 5: Commit** (alone or with Task 3)
```
bar : anchor capture for notification bell
```

---

### Task 3: `history_popup` window — AnchoredPopup + fallback

**Files:**
- Modify: `crates/app/src/notifications/history_popup/mod.rs`

**Reference:** `crates/app/src/updates_popup/mod.rs` (`window_options`,
`fallback_window_options`, `open` match on `PopupNotSupportedError`,
`close` / `close_this` reentrancy discipline — **do not regress
HANDOFF ghost-window pattern**).

- [ ] **Step 1: Change `open` / `toggle` signatures**
```rust
pub fn open(cx: &mut App, anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle);
pub fn toggle(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle, window: &mut Window, cx: &mut App);
```

- [ ] **Step 2: Anchored `WindowOptions`** — width **360**, height estimate
  from history len (header budget ≈ 0 if no header; list + optional
  footer). Keep constants named and documented.

- [ ] **Step 3: Fallback LayerShell TOP|RIGHT** on `PopupNotSupportedError`
  (log warn, same as updates).

- [ ] **Step 4: Keep `MarkAllRead` on open**

- [ ] **Step 5: Keep watcher / resize / notify paths**; any
  `handle.update` failures → log, never silent `let _ =`.

- [ ] **Step 6: Build**
```bash
cargo build -p chronos 2>&1 | tail -40
```

- [ ] **Step 7: Commit**
```
notifications/history_popup : anchored open + LayerShell fallback
```

---

### Task 4: View — mockup-faithful list + Clear all + row dismiss

**Files:**
- Modify: `crates/app/src/notifications/history_popup/view.rs`
- Optionally: `crates/app/src/notifications/view.rs` **only if** extracting
  a shared card is cleaner; default prefer history-local renderer so
  ephemeral toast is untouched.
- Ensure `ScrollHandle` field on view if required by fork scroll API
  (mirror `UpdatesPopupView`).

**Mockup layout (acceptance):**
```
┌─ panel 360, bg #1e1e2e / theme.bg, border #313244, radius 10 ─┐
│ [urgency 3px] [icon letter] app name          [✕ dismiss]     │
│                 summary (semibold)                             │
│                 body (muted, clamp ~4 lines)                   │
│                 [Action] [Action]  (outlined, optional)        │
│ ─ row border ─ … scroll …                                     │
│ [ Clear all ]  (outlined #007acc; only if len > 1)             │
└────────────────────────────────────────────────────────────────┘
Empty: centered "No notifications"
No panel title row. Newest first (current reverse order OK).
```

- [ ] **Step 1: Remove header "Notifications" + panel close ✕**

- [ ] **Step 2: Scrollable list** with `.id("notif-history-list")` +
  `overflow_y_scroll` + max height budget

- [ ] **Step 3: Card chrome** — urgency strip, app monogram (first letter
  of `app_name`, color from theme accent/status or hash — readable),
  summary/body/actions

- [ ] **Step 4: Row ✕** → `NotificationCommand::RemoveFromHistory(id)`
  via `background_spawn` + proper error log (not `let _ =` without log)

- [ ] **Step 5: Actions** → existing `InvokeAction(id, key)` if actions
  non-empty (same as toast card)

- [ ] **Step 6: Clear all** → `ClearHistory`; footer only when `len > 1`

- [ ] **Step 7: Empty state** string exactly or near mockup:
  `"No notifications"`

- [ ] **Step 8: Build release**
```bash
cargo build --release -p chronos
```

- [ ] **Step 9: Commit**
```
notifications/history_popup : mockup list UI + clear/dismiss
```

---

### Task 5: Live smoke + report (mandatory)

**No code unless errata.**

- [ ] **Step 1: Restart release binary** (kill stale process first —
  stale deleted binary is a known false "UI broken")

- [ ] **Step 2: Seed history**
```bash
notify-send "Zed" "Build finished" -u normal
notify-send "Mail" "Re: design review" -u low
notify-send "System" "Battery critical" -u critical
# optional: actions if your notify-send supports them
```

- [ ] **Step 3: Open bell** — popup under/near bell (not random corner),
  unread badge clears (`MarkAllRead`)

- [ ] **Step 4: Scroll** if enough items; row ✕ removes one; Clear all
  empties when ≥2

- [ ] **Step 5: grim screenshots** — open with items, empty state,
  optional after clear

- [ ] **Step 6: Write report**
  `orchestration/tasks/report/T120-notifications-history-popup-report.md`
  with: commits, test commands+counts, live yes/no, grim paths,
  residual debt. Honest PENDING if something blocked.

- [ ] **Step 7: Do not mark accepted** — Architect accepts.

---

## Execution notes (for the human / Architect running SDD)

| Task | Role | Model tier hint |
|------|------|-----------------|
| 1 | implementer | cheap–standard (service + TDD) |
| 2–3 | implementer | standard (window lifecycle) |
| 4 | implementer | standard–strong (pixel UI) |
| 5 | implementer / operator | any + human eyes |
| after each 1–4 | task review | standard (spec + quality) |
| after all | Architect accept | greps, diff, live re-smoke |

**Progress ledger (optional, if running multi-session):**
`.superpowers/sdd/progress.md` — append `Task N: complete (commits …)`
so compaction does not re-dispatch finished work.

**Order:** 1 → 2 → 3 → 4 → 5. Do not parallelize 2–4 on the same
files. Task 1 can theoretically land first while UI is separate crates;
UI tasks still sequential.
