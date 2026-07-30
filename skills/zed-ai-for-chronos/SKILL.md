---
name: zed-ai-for-chronos
description: >
  Use when designing or implementing ChronOS left agent panel / Hermes ACP
  integration and deciding what to steal from Zed AI architecture — GPL
  boundary, layer split, phased scope, and explicit non-goals versus
  Chronos-IDE chronos-agent.
---

# Zed AI → ChronOS (steal map)

**Entry:** `zed-ai`  
**Product target:** left `side_panel_left` agent chat (brainstorm 2026-07-22)  
**Right panel:** system sidebar — different module; reuse **windowing only**.

## License

| Allowed | Forbidden |
|---|---|
| ACP public protocol behavior | Copying GPL `agent*` / `acp_thread` / `agent_ui` source |
| Ideas: state machines, UI IA, stdio lifecycle | Vendoring those crates into Apache tree |
| Clean-room reimplementation | "I'll adapt this file slightly" |

## Architecture to mirror (Approach A)

```text
crates/app/src/side_panel_left/     # window: peek/pin/resize (← side_panel_right)
crates/services/…/hermes_acp/      # stdio + sessions (← agent_servers pattern)
UI entities: SessionList | Thread | Composer | Status
schema: agent-client-protocol OR hand JSON-RPC
composer: homemade textarea (gpui-component BLOCKER — see §gpui-component BLOCKER)
```

**One Hermes process per ChronOS shell**; multi-session on that connection.

## Steal / skip

| Zed pattern | ChronOS | Skill |
|---|---|---|
| Panel dock + min width 300 | layer-shell + drag resize | `zed-agent-panel` |
| retained_threads + draft + idle cap | yes | `zed-agent-panel`, `zed-thread-metadata` |
| ServerState Loading/Error/Connected | yes | `zed-conversation-view` |
| entries User/Assistant/ToolCall | yes v1 | `zed-acp-thread` |
| stdio initialize + new_session | yes | `zed-acp-stdio` |
| MessageEditor=Editor | **no** — TextInput | `zed-message-editor` |
| @mentions / slash | later | `zed-message-editor` |
| Diff + terminal tool bodies | v1.1 | `zed-agent-tools` |
| subagent sessions | v2 | `zed-acp-thread` |
| terminal-agent tabs in panel | no | `zed-agent-panel` |
| native edit_file tool host | no (Hermes) | `zed-agent-tools` |
| Workspace/Project/Buffer graph | no | — |

## Not Chronos-IDE

`Chronos-IDE/chronos-agent` = custom OpenAI-style LLM + local tools registry.
**Not ACP.** Do not wire left panel to that crate by default; Hermes ACP is
the shell contract.

## Phased ship (status 2026-07-23)

| Phase | Status | Notes |
|---|---|---|
| **Skills** — this family, agents route here first | ✅ DONE | |
| **Mockup** `docs/design/Agent Thread.dc.html` | ✅ DONE | |
| **v0 — Thread canvas** (T109): thread header + message flow + composer | ✅ DONE (C-2 blocker, см. ниже) | Визуальные блоки A, B, C по мокапу;
  три утверждённых отклонения;
  C-2 gpui-component BLOCKER — homemade textarea fallback |
| **v1:** window + ACP + sessions + stream + collapsed tools + composer + model + status | 🔄 WIP (canvas готов, остальное в очереди) | |
| **v1.1:** terminal expand + diffs | 📅 later | |
| **v1.2:** @context chips | 📅 later | |
| **v2:** subagents | 📅 later | |

### v0 — Thread canvas (T109) result

**Files:** `crates/app/src/side_panel_left/` — `panel.rs`, `chat_view.rs`, `composer.rs`, `mod.rs`, `state.rs`

**Thread header (block A):** 38px sub-header inside the panel, sparkle `#007acc` icon + agent name + three header buttons (+ new session / history / ⋯).

**Message flow (block B):**
- User: card on `bg #1e1e2e`, `border 1px #232336`, rounded 7, padding 8 10 — no "You" label
- Agent: flat text `#cdd6f4`, no background — no "Agent" label
- Tool cards preserved below message
- Empty state: "No messages yet" centered

**Composer (block C):** auto-growing textarea (min ~64px / ~3 rows, max 45% panel height), on `bg #181825` (unified with chat canvas), `border_t #232336` divider. Three accepted deviations from the mockup:

1. **Dark send button** (not blue): 24×24, `bg #11111b`, `border 1px #313244`, icon `#cdd6f4`; hover `bg #232336`, `border #45475a`; inactive `#45475a`; blocked when `agent_status == Thinking`
2. **YOLO button** (not sparkle): text pill, font 10px semibold. Detects bypass modes — см. §YOLO bypass pattern
3. **Unified input/output canvas**: composer on same background as chat (`#181825`), only a hairline `border_t #232336` separates them

**No `rsx!` used** in any of these files — every element requires listeners or conditional geometry; pure `div()` builder. rsx-vs-div map in the T109 report.

### gpui-component BLOCKER

`gpui-component` depends on Zed's gpui (`git = "https://github.com/zed-industries/zed"`).
ChronOS uses `gpui-ce` via `path = "../Source/gpui"`. API incompatibility:
```
error[E0432]: unresolved imports `gpui::AssetSource`, `gpui::Result`, `gpui::SharedString`
```
**Resolution:** homemade `handle_composer_key` + auto-grow textarea fallback. To lift the blocker:
port TextInput from gpui-component into the gpui-ce fork, or bring gpui-ce to Zed API parity.

## YOLO bypass pattern

YOLO = quick bypass mode toggle in the composer toolbar.

**Detection:** `detect_yolo_bypass_mode(modes)` — search `available_modes` for an `id`
containing `bypass`|`dont`|`yolo` (case-insensitive). Cache the match in
`composer_yolo_bypass_id: Option<String>`.

**Toggle:** first click → switch current mode to bypass mode, save previous mode in
`composer_previous_mode: String`. Second click → restore previous mode.

**Lifecycle:** `available_modes` is **empty at startup** (arrives after first ACP
request). YOLO button hidden until modes arrive. `detect_yolo_bypass_mode()` called
in the `send_composer` response handler when modes update.

**State fields** (on `SidePanelLeft`): `composer_previous_mode`, `composer_yolo_bypass_id`.  

## Windowing baseline (from right panel)

Reuse blood facts: `PANEL_EDGE_GAP = BAR_HEIGHT`, top gap under bar, height to
bottom, hover strip + peek generation debounce, pin vs peek, no Esc
(`KeyboardInteractivity` policy as right). **New:** left anchor + width drag.

Skills: `gpui-layer-shell`, `side_panel_right` code, `chronos-shell`.

## Thread canvas visual specs (mockup -> code)

| Mockup element | Code |
|---|---|
| bg chat + composer | `#181825` |
| user message card bg | `#1e1e2e` |
| user message card border | `1px #232336` |
| agent message text | `#cdd6f4`, flat, no bg |
| thread header bg | inherit from panel |
| sparkle icon | `#007acc` |
| composer divider | `border_t 1px #232336` |
| send button bg | `#11111b` |
| send button border | `1px #313244` |
| send icon | `#cdd6f4` |
| send hover bg | `#232336` |
| send hover border | `#45475a` |
| send disabled | `#45475a` |
| YOLO active bg | `rgba(0xf3_8b_a81e)` + text `#f38ba8` |
| YOLO inactive | `#6c7086` |
| YOLO disabled | `#45475a` |
| model/mode picker text | `#a6adc8`, hover `bg #232336` |
| placeholder text | `"Message {agent} — @ to include context, / for commands"` |

## Red flags

- "Just copy agent_panel.rs" → GPL + won't compile without half of Zed  
- "Use chronos-agent from IDE" → wrong protocol  
- "v1 = full Zed parity" → multi-month; force phase cut  
- Skipping Loading/Error states → blank flaky panel  
- Second chronos instance / wrong worktree — still HANDOFF field rules  

## Recon location

`/home/neo/scratch/zed-agent-recon` — sparse crates only; refresh with sparse
checkout if missing. Line numbers drift; grep symbol names.
