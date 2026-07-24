---
name: evaluating-hot-reload-solutions
description: Use when evaluating Rust hot-reload solutions (subsecond, hot-lib-reloader, etc.) for non-Dioxus UI frameworks like GPUI, Iced, Bevy — especially when workspace has unsafe_code = "deny" and you cannot pull Dioxus toolchain.
---

# Evaluating Hot-Reload Solutions for Non-Dioxus Projects

## Overview

When your project uses a non-Dioxus UI framework (GPUI, Iced, Bevy, Tauri/wry, egui) and you need dev hot-reload, the landscape is fragmented. This skill captures the evaluation framework used in ChronOS Track B bake-off: criteria, fatal blockers, and decision matrix.

## When to Use

- Choosing hot-reload for a GPUI / Iced / Bevy / Tauri project
- Evaluating `subsecond` / `hot-lib-reloader` / `cargo-watch` alternatives
- Workspace enforces `unsafe_code = "deny"` (or similar safety policies)
- You **cannot** pull full Dioxus CLI/toolchain (wrong framework)
- Need function-level patching without separate dylib crate

## Quick Decision Matrix

| Solution | Framework | Runner | Unsafe in app? | ThinLink/standalone | Verdict for GPUI |
|---|---|---|---|---|---|
| **subsecond + `dx serve`** | Dioxus only | `dx` (requires Dioxus) | ❌ `apply_patch` is `pub unsafe` | ThinLink inside `dx` only | ❌ Wrong framework |
| **subsecond + `cargo-hot`** | Any (theoretically) | `cargo-hot` (broken) | ❌ Same unsafe API | No | ❌ Runner broken |
| **hot-lib-reloader + cdylib** | Any | `cargo watch` | ✅ Isolated in separate crate | N/A (dylib reload) | ✅ **Track A winner** |
| **cargo-watch + full rebuild** | Any | `cargo watch` | ✅ Safe | N/A | ✅ Baseline |

## Core Blockers for subsecond in Non-Dioxus Projects

### 1. Unsafe API Leakage (Fatal)

```rust
// subsecond 0.8.0-alpha.0 public API
pub unsafe fn get_jump_table() -> Option<JumpTable>;
pub unsafe fn apply_patch(mut table: JumpTable) -> Result<(), PatchError>;
```

**If your workspace has `unsafe_code = "deny"` (ChronOS does at Cargo.toml:27):**
- Calling these from your app crate requires `unsafe {}` blocks in **your code**
- This violates workspace policy — not fixable with `#[allow(unsafe_code)]`
- The unsafe is **not encapsulated inside subsecond** — it leaks to callers

### 2. ThinLink Not Standalone

- ThinLink (the linker generating jump-tables) is **bundled inside Dioxus CLI**
- No separate binary, no library API, no Cargo integration
- Without ThinLink, no jump-tables → no patches delivered

### 3. Runner Gap

| Runner | Status | Usable standalone? |
|---|---|---|
| `dx serve --hotpatch` | Works | ❌ Requires Dioxus project structure |
| `cargo-hot` (hecrj) | "Very broken! Will eat your laundry!" | ❌ Alpha, unmaintained |
| Custom runner | You write it | ❌ Requires implementing WebSocket protocol + ThinLink |

### 4. Workspace Limitation

> "Subsecond currently only patches the 'tip' crate — ie the crate in which your `main.rs` is located. Changes to crates outside this crate will be ignored." — docs.rs/subsecond

In a workspace like ChronOS (`crates/app`, `crates/ui`, `crates/services`, `crates/luau`), only `crates/app` would hot-reload. Shared logic in `crates/services` (e.g., `net_stats::update_speed`) **won't patch**.

## Evaluation Protocol (Track B Bake-off)

Use this checklist for any candidate:

```markdown
- [ ] **Framework match:** Does it work with MY UI framework (not Dioxus)?
- [ ] **Safety:** Zero `unsafe` required in MY crate(s)? 
- [ ] **Runner:** Standalone runner exists, maintained, no foreign toolchain?
- [ ] **Workspace:** Patches all workspace crates, not just tip?
- [ ] **State safety:** Handles Entity/globals/subscriber re-instancing?
- [ ] **Build speed:** Incremental < 1s for target scope?
- [ ] **Failure mode:** On compile error — keeps old version running?
```

## Track A Pattern (Working Alternative)

For GPUI/ChronOS-style architecture:

```
crates/hotview/          # Separate cdylib + rlib crate
  - render_network_widget(NetState) -> impl IntoElement  # PURE, no cx.subscribe
  - #[allow(unsafe_code)] inside this crate ONLY
  - Own [lints], NOT workspace = true

crates/app/
  - BarWidget::render() calls hotview::render_network_widget(state)
  - cargo watch -w crates/hotview rebuilds .so only
  - hot-lib-reloader loads .so, calls cx.notify() on widget
```

**Key invariants:**
- Hot function = **pure render**: `(State) -> Element`, no subscriptions
- State (tickers, caches, subscribers) stays in `crates/app`
- Struct layout (`NetState`, `SpeedView`) **must not change** between patches

## Common Mistakes

| Mistake | Why it fails |
|---|---|
| "I'll just add `#[allow(unsafe_code)]` locally" | Violates workspace policy; auditor will reject |
| "cargo-hot will improve" | 0.1.1 since 2023, marked broken by author |
| "I'll write my own ThinLink runner" | 3+ months work; protocol undocumented |
| "subsecond works for Bevy, so GPUI too" | Bevy has official integration + custom runner |
| "Put hot-reload in release for faster dev" | Spec forbids: "не включай hot-reload путь в release-профиль" |

## Real-World Impact (ChronOS T111)

| Track | Result |
|---|---|
| Track B (subsecond) | **Failed** — unsafe API + no standalone runner |
| Track A (hot-lib-reloader) | **Passed** — isolated unsafe, pure render, cargo watch |

**Time spent:** ~4 hours evaluation + integration attempt → documented failure → Track A auto-wins.

## Related Skills

- **REQUIRED BACKGROUND:** `chronos-shell` — understanding GPUI architecture, Entity/globals, why full-app hot-swap is impossible
- **REQUIRED BACKGROUND:** `using-git-worktrees` — isolation for spike evaluation
- **COMPLEMENTARY:** `writing-plans` — for structuring bake-off specs

## Keywords for Discovery

hot-reload, subsecond, dioxus, gpui, hot-lib-reloader, unsafe_code deny, thinlink, function-level patching, hot reload rust, dev tools, cargo watch, cdylib