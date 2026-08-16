# T124 — ПРИНЯТ WITH CAVEATS (2026-07-25)

**Статус: ACCEPTED WITH CAVEATS.** Ephemeral toast stack mockup.
Commit: `813b3aa`. Live grim user. Anim debt. Review: report-log/T124-*-review.md.

---

<!-- T124 — Global ephemeral toast stack (not history_popup).
     Мокап: docs/design/Toast-Notifications.dc.html. Агент не в имени брифа. -->

# T124 — Ephemeral toast notifications (global stack)

**Статус: OPEN, не назначен.**  
**Мокап (канон):** `docs/design/Toast-Notifications.dc.html`  
**Код сегодня:** `crates/app/src/notifications/{mod,view}.rs`  
**Не путать с:** `notifications/history_popup/` (T120, bell inbox) — **out of scope**.

## Цель

Глобальный **эфемерный** стек тостов (FDO live `NotificationState::notifications`):
визуал и поведение по мокапу — top-right stack, toast-карточки, progress
auto-dismiss, urgency styling, enter motion (если форк даёт), actions + ✕.

Сейчас: MVP-карточки (`render_notification_card`), left border urgency,
фиксированное окно 360×360, `overflow_hidden` без toast-chrome, **нет**
progress-бара, **нет** enter/exit, без soft shadow / icon tile.

## Мокап → requirements

| | Mockup | Сейчас |
|---|---|---|
| Позиция | top-right, margin ~top 42 / right 16 (под bar 30) | TOP\|RIGHT LayerShell margin 12 |
| Ширина стека | **340px** | 360 |
| Gap | 8px | 8px (ok) |
| Карточка | bg `#1e1e2e`, border `#313244`, radius **8**, shadow heavy | flat card + left border accent |
| Layout | icon 28×28 \| app + ✕ / summary / body / actions | app name row + summary + body |
| Progress | 2px bar bottom, color by kind, shrinks over TTL | нет |
| Critical | border tint `#f38ba833`, pulse shadow, summary/app accent red | status.error left border only |
| Actions | outlined small chips | secondary bg buttons |
| Anim | enter: opacity+translateX+scale; exit symmetric | нет |
| Sticky | error «manual dismiss» (long/no progress?) | expire via daemon only |

### Mapping FDO urgency → toast kind (зафиксировать в коде + отчёте)

FDO даёт только `Low | Normal | Critical` (`Urgency`). Мокап рисует 4
семантики (info / warning / error / success). **MVP mapping:**

| Urgency | Kind | Progress color | Notes |
|---|---|---|---|
| `Low` | info | `#89b4fa` | default short TTL |
| `Normal` | warning **or** info | `#f9e2af` if you want warn-for-normal; **prefer info `#89b4fa`** for Normal, use warning only if you add a later hint | document choice |
| `Critical` | error | `#f38ba8` | pulse; longer hold; sticky if `expire_at` is `None` |

**Success green** (`#a6e3a1`): **не выдумывать** category parser в T124,
если в `Notification` нет поля. Optional: heuristic later. If no success
signal → skip success skin; mockup shows it as aspirational.

Default TTL if daemon didn't set: use existing expire path; progress bar
duration = `expire_at - now` when known, else hide progress or use fixed
5s visual only **without** lying about dismiss (prefer hide if unknown).

## Архитектура (не ломать)

- `NotificationPopupState` + `sync_window`: empty → close window; non-empty → open.
- LayerShell `Overlay`, TOP|RIGHT, **no exclusive zone**, keyboard None.
- Daemon: `NotificationCommand::Close` / `InvokeAction` — уже есть; **`.detach()`**
  on `background_spawn` (T120 lesson).
- `expire_at: Option<u64>` on `Notification` — for progress fraction.

**Не** AnchoredPopup к bell (это history). Ephemeral остаётся corner stack.

### Window sizing

Keep **fixed cap** philosophy (module docs: no pixel-estimate bleed). Options:

1. Width **340**, height cap ~`max(stack, 1 card)…` still hard max (e.g. 480)
   with clip/overflow; **or**
2. Adaptive height via estimate of N cards (risky — history of bugs #9–#12).

**Recommended:** width 340; `LIST_MAX_H` raise if needed; clip older toasts
at bottom (they expire). Transparent outer chrome (no panel border around
whole stack — mockup is **separate** cards, not one framed list).

Remove the outer stack `border_1` if still present (`view.rs` currently
borders the whole column — mockup cards each have their own border).

## UI tasks

### 1. Toast card renderer (ephemeral-only)

New helper e.g. `render_toast_card` in `view.rs` (or `toast_card.rs`).  
**Do not** force history to use the same skin; history already has
`render_history_card`. Shared code only if trivial (monogram).

Card structure (mockup):

```
┌─────────────────────────────────────────┐
│ [icon 28]  app name              [✕]    │
│            summary (semibold)           │
│            body (muted, multi-line)     │
│            [Action] [Action]            │
├─────────────────────────────────────────┤  ← 2px progress track
│ ████████░░░░  (kind color, shrinking)   │
└─────────────────────────────────────────┘
```

- Icon: monogram / status glyph by urgency (SVG assets if exist; else
  colored tile + letter like history). App icon path string may be empty.
- ✕ → `Close(id)` + detach.
- Actions → `InvokeAction(id, key)` + detach.
- Progress: width fraction from remaining TTL; tick repaint:
  - either `cx.spawn` / interval on `NotificationsView` while non-empty
  - or depend on existing daemon expiry + re-render on state watch
    (progress may jump; better smooth tick ~100–250ms while open).

Critical: soft red border, optional `gpui_animation` pulse if cheap
(skill `chronos-gpui-popup` / `vendored-gpui-animation`); else static
border is accept-min for pulse.

### 2. Enter animation (best-effort)

Mockup CSS enter 280ms. Fork: `with_transition` / opacity if boot
`gpui_animation::init` already from bar (T121+). If fighty — ship without
enter, document debt. **Do not** block accept on perfect CSS parity.

### 3. mod.rs geometry

- `POPUP_WIDTH = 340.`
- Margins: top ≥ bar clearance (~42 from screen top in mockup; bar ~30 →
  margin_top ~12–16 ok if bar exclusive; verify live under ChronOS bar).
- Transparent window bg; stack gap 8.

### 4. Live smoke

```bash
chronos-rebuild && chronos-stop && chronos-start
notify-send -u low "ChronOS" "Screenshot saved"
notify-send -u normal "System" "Battery at 15%"
notify-send -u critical "Package Manager" "Update failed"
# grim stack; click ✕; action if any; wait auto-dismiss + progress
```

## Зона файлов

**Писать:**
- `crates/app/src/notifications/view.rs` (primary)
- `crates/app/src/notifications/mod.rs` (width/margins/height constants)
- optional assets under `crates/app/assets/icons/` if new glyphs needed

**Не трогать:**
- `history_popup/**` (except if shared helper extraction is zero-behavior)
- volume/updates/system popups
- notification **service** protocol unless expire_at missing (it exists)
- T123 audio

## Skills

- `chronos-gpui-popup` — layer popup discipline, window bounds trap
- `gpui-rsx` — static chrome optional; dynamic list → builder
- `vendored-gpui-animation` / `easing-and-springs` — enter/pulse optional
- `anchored-popups` — **N/A** (not bar-anchored)

## Верификация

```bash
cargo build --release -p chronos
cargo test -p chronos --lib  # any new pure helpers
# live notify-send + grim
```

Отчёт:  
`docs/orchestration/tasks/report/T124-ephemeral-toast-notifications-report.md`  
— urgency mapping table, progress approach, anim yes/no, grim paths.

## Accept / Reject

**Accept:**
- Stack of independent toast cards (no single outer list border)
- Width ~340, top-right, ✕ + actions work (detach)
- Progress bar when TTL known (or honest skip)
- Critical visually distinct
- Live notify-send evidence
- History popup unchanged in behavior

**Reject:**
- Only restyle shared history card and break inbox
- `pkill -f` / dual shell nonsense
- Fabricated “success” category without data
- Unit-only without live toasts

## Out of scope

- Replace FDO daemon
- Do-not-disturb / per-app mute UI
- Grouping / replacement-id polish beyond existing service
- Light theme variant of toast mockup (dark only in file)
