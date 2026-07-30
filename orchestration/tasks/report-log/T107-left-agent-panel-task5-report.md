## Task 5: Panel Header + Status Indicator — Done

### What was built
- `panel.rs` — real header with rsx!, matching right panel's pattern
- `state.rs` — added `AgentStatus` enum (Connected/Disconnected/Thinking)

### Header structure (rsx!)
```
┌──────────────────────────────┐
│ ● Agent                   ✕  │  ← status dot + label + close
├──────────────────────────────┤
│ Chat goes here               │  ← body placeholder
└──────────────────────────────┘
```

### Colors (Catppuccin hex literals, same as right panel)
- Status dot: `a6e3a1` (green), `f38ba8` (red), `f9e2af` (yellow)
- Border: `232336`
- Label text: `a6adc8`
- Close button idle: `6c7086`, hover: `232336` bg + `cdf6f4` text

### Close button
Wired to `crate::side_panel_left::close_this(window, cx)` — same ghost-guard pattern as right panel.

### Tests
Both existing `side_panel_left` tests pass. `PanelState` gained `Debug` derive for `assert_eq!`.

### Commit
`78841b6` on `feat/left-agent-panel`
