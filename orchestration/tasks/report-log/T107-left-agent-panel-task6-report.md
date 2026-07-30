## Task 6: Sessions Sidebar — Done

### What was built
- `sessions_list.rs` — `SessionItem` struct + sidebar width constants
- `state.rs` — `sessions_collapsed: bool`, `active_session_id: Option<String>` on `SidePanelLeftState`
- `mod.rs` — `sessions: Vec<SessionItem>` on `SidePanelLeft`, `toggle_collapse()`, `create_new_session()`, `select_session()` methods
- `panel.rs` — inline sessions sidebar in body, builder pattern (rsx for static chrome, div for dynamic session items)

### Sidebar structure
```
COLLAPSED (48px icon strip):        EXPANDED (200px sidebar):
┌────┐                              ┌──────────────────┐
│ >  │  expand button               │ Sessions      <  │  ← header + collapse
│ +  │  new session                 ├──────────────────┤
│ ●  │  active session (green dot)  │ + New session    │  ← new session button
│ ○  │  inactive session            │                  │
│ ○  │                              │ ● Active Session │  ← scrollable list
│ ○  │                              │ ○ Session 2      │
└────┘                              │ ○ Session 3      │
                                    └──────────────────┘
```

### Layout
Panel body is a `flex_row`:
- Left: sessions sidebar (48px or 200px based on collapse state)
- Right: chat area (remaining width = 352 - sidebar_width)

### Session items
- Active: green dot (`a6e3a1`) + dark bg (`313244`) + light text (`cdf6f4`)
- Inactive: gray dot (`585b70`) + hover bg (`232336`) + muted text (`a6adc8`)
- Click handler wired via `cx.listener` for collapse/new session (select_session ready for future wiring)

### Implementation notes
- Sidebar rendering done with builder pattern (div chains) — rsx `{for ...}` syntax wasn't needed since `children()` with iterators works cleanly
- `cx.listener` closures take 4 args: `|this, _ev, _window, cx|` (GPUI contract)
- `ElementId` doesn't support tuple IDs — used `format!("session-dot-{sid}")` strings
- `overflow_hidden()` not available as rsx attribute — used builder `.overflow_hidden()`

### Colors (Catppuccin hex literals)
- Sidebar bg: `181825`, border: `232336`
- Header text: `a6adc8`, weight: SEMIBOLD
- Collapse/expand button: `6c7086` idle, `232336` bg + `cdf6f4` text on hover
- Session active bg: `313244`, border: `45475a`

### Tests
Both existing `side_panel_left` tests pass. `select_session` method unused for now (prepared for chat_view wiring).

### Commit
`902c0bd` on `feat/left-agent-panel`
