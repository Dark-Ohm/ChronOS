# T108 — Multi-agent switcher: Task 1 Report (REVISED)

**Date:** 2026-07-23
**Status:** PARTIAL — code compiles, 2/2 tests pass, live verification pending
**Revision note:** Previous report (task1) contained fabricated test results.
  Claimed "4/4 pass: state_starts_as_peek, state_default_width,
  state_starts_with_hermes_active, known_agents_includes_hermes" — the
  last two tests **never existed in code**. The "4 passed" was from
  unrelated tests in root `state.rs`. This revision reports real output.

---

## What was done

### 1. Agent Registry (`crates/services/src/hermes_acp/registry.rs`)

New file. Defines:
- `AgentDescriptor` — `id`, `display_name`, `config: HermesConfig`
- `known_agents()` — returns verified ACP-compatible backends
- `find_agent(id)` — lookup by ID

**Verified backends:** Only Hermes (`hermes acp`) — confirmed working via
live ACP handshake on the `feat/left-agent-panel` branch. No other ACP
stdio-compatible backends were found. Cline, OpenCode, Kilo, Grok are
separate CLI tools without ACP stdio protocol support — NOT included.
The dropdown shows only Hermes until new backends are verified.

### 2. Multi-instance client management

- `HermesClient::new(config: HermesConfig)` — now accepts config (was hardcoded Default)
- `SidePanelLeft` holds `clients: HashMap<String, HermesClient>` (lazy spawn)
- `agents: Vec<AgentDescriptor>` from registry, `active_agent_id: String`
- Per-agent sessions (physically impossible to share ACP sessions across processes)

### 3. UI — agent dropdown in header

- Header: clickable cluster (dot + agent name + chevron-down icon)
- Dropdown: absolutely positioned `172px` panel, `top:38px; left:8px`
- Each agent: status dot (color-coded) + display name + checkmark if selected
- Selection: closes dropdown, switches `active_agent_id`, lazy spawns if needed

### 4. Architect-found fixes (committed as part of task1)

1. **Borrow-checker (E0502):** resize handlers built before `render_composer()`/`chat` to avoid double-borrow on `cx`
2. **Ghost-window during resize:** `hold_peek(cx)` called on every `update_resize` tick
3. **Multi-monitor height:** `update_resize` uses `pult_display(cx)` instead of `None`/primary
4. **Transparent background:** `.bg(rgb(0x1e_1e_2e))` on main content area
5. **State simplified:** agent fields moved from `SidePanelLeftState` to `SidePanelLeft` struct

---

## Files touched

| File | Change |
|---|---|
| `crates/services/src/hermes_acp/registry.rs` | **NEW** — AgentDescriptor + known_agents() |
| `crates/services/src/hermes_acp/mod.rs` | Added registry module + re-exports |
| `crates/services/src/hermes_acp/client.rs` | `new(config)` instead of `new()` |
| `crates/services/src/lib.rs` | Added `known_agents`, `find_agent` re-exports |
| `crates/app/src/side_panel_left/state.rs` | AgentStatus Debug derive (fields moved to mod.rs) |
| `crates/app/src/side_panel_left/mod.rs` | Multi-client HashMap, switch_agent(), toggle_agent_menu() |
| `crates/app/src/side_panel_left/panel.rs` | Pure builder API (no rsx!), clickable header, dropdown |
| `crates/app/src/side_panel_left/composer.rs` | Client access via HashMap |

---

## Verification

- `cargo check -p chronos` — green (0 errors)
- `cargo test -p chronos side_panel_left` — **2/2 pass:**
  - `state_starts_as_peek` ✓
  - `state_default_width` ✓
- (Previous report claimed 4 tests including `state_starts_with_hermes_active`
  and `known_agents_includes_hermes` — these **do not exist** in the code)

---

## Live verification — NOT DONE

1. Build release binary — `cargo build --release -p chronos`
2. Run on Hyprland — open left panel, verify:
   - Header shows "Hermes" with green dot + chevron
   - Click opens dropdown, renders on top of content
   - Dropdown shows "Hermes" with checkmark
   - Click outside / select agent closes dropdown
   - Status updates correctly (Connected/Thinking/Disconnected)
3. Grim screenshot of dropdown open state
4. `hyprctl layers` — confirm side_panel_left layer exists

---

## What's NOT in this task

- Only Hermes in the registry (honest — no other ACP stdio backends verified)
- Model/mode lists in composer are hardcoded stubs (item #6 in task file)
- Model dropdown jank (~20fps) unresolved (item #7 in task file)
