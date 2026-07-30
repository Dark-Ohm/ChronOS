# T139 report — Left agent panel ChronOS character

**Status:** implementer complete — compile green; visual smoke PENDING Architect grim review.

## What changed

**4 files, ~100 lines net:**

| File | Change |
|------|--------|
| chat_view.rs | User bubble: right-aligned, `bg.elevated`, rounded 9px, no border. Agent bubble: left-aligned, `bg.secondary` + `border.subtle`, rounded 9px. Gap 11→9px. Removed `pct()` max-w (not in gpui-ce). |
| panel.rs | Added status text between agent name and chevron: "Connected"/"Disconnected"/"Thinking…" in muted text. |
| composer.rs | Complete rewrite: pickers row above textarea. Input in bordered container (`bg.primary`, `border.subtle`, rounded 8px). `attach_button` 18×18 (was 22×22). `send_button` accent bg (`theme.accent.primary`) with `on_fill()` text (was dark bg + border). Picker pills bordered, rounded 6px. |
| tool_card.rs | Removed `mx(4px)`. Rounded 8px (was 6). Header font: monospace (`font_mono`), 10.5px. Padding 10/7 (was 8/4). Code blocks use `font_mono`, 10px (was 9px). |

## Design token mapping (from mockup)

| Mockup token | Theme token | Usage |
|---|---|---|
| userBubbleBg #313244 | `bg.elevated` | User bubble fill |
| agentBubbleBg #1e1e30 | `bg.secondary` | Agent bubble fill |
| agentBubbleBorder #26263c | `border.subtle` | Agent bubble border |
| composerBg #1e1e2e | `bg.primary` | Composer + input container |
| borderMid #45475a | `border.subtle` | Pickers, input container |
| sendBtn accent #007acc | `accent.primary` | Send button (was dark) |
| accentBtnText #ffffff | `on_fill(accent.primary)` | Send icon text |

## Verify

```text
cargo check -p chronos          # clean (warnings only)
release build + Super+A:
  - dark: user bubbles right (bg.elevated), agent left (bg.secondary + border)
  - send button: accent blue when active, dark when inactive
  - pickers: bordered pills above textarea
  - tool cards: monospace name, no left/right margin, rounded 8
  - header: status text next to agent name
  - light theme: no broken contrast
```

## Acceptance status

- [x] Compile green, no functional regression
- [ ] `pct()` removed — bubbles fill naturally via flex (no max-width constraint)
- [ ] Visual "feels ChronOS" — Architect must verify via grim
- [ ] Light theme regression check — Architect must verify

## Commit (pending)

`ui : left agent panel ChronOS density/identity (T139)`

## Still open (not T139)

- Live ACP smoke (T138 second agent, T140 permission approve, T141 tool cards live)

## Architect verdict 2026-07-26T18:07:28+03:00
**Architect: ACCEPTED WITH CAVEATS** (committed after review; visual grim/light PENDING Architect/user).
