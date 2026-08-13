# T267 Unified Edge Separator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the right panel and bar use the same `border.subtle` separator token as the left panel.

**Architecture:** Apply a direct token swap at the two existing border paint sites. Preserve bar edge selection and edit-mode accent behavior; leave both transparent hover strips unchanged.

**Tech Stack:** Rust 2024, GPUI, ChronOS theme tokens, Hyprland live verification with `grim`.

## Global Constraints

- Modify only `crates/app/src/bar/mod.rs` and `crates/app/src/side_panel_right/view.rs` in production code.
- Do not add a helper or a theme token.
- Keep edit-mode border color as `theme.accent.primary`.
- Hover strips are verified transparent and require no changes.
- Do not drag files from Chronos-FM during live verification until T270 passes live.
- Unit tests are not required by T267; verification is compile plus dark/light live frames.

---

### Task 1: Unify separator tokens

**Files:**
- Modify: `crates/app/src/bar/mod.rs:121-125`
- Modify: `crates/app/src/side_panel_right/view.rs:683-687`

**Interfaces:**
- Consumes: `Theme::global(cx).border.subtle`
- Produces: consistent one-pixel bar and right-panel separator colors

- [ ] **Step 1: Change the non-editing bar border token**

```rust
root = root.border_color(if editing {
    theme.accent.primary
} else {
    theme.border.subtle
});
```

- [ ] **Step 2: Change the right-panel border token**

```rust
b.bg(surfaces::chrome(&theme))
    .border_l_1()
    .border_color(theme.border.subtle)
```

- [ ] **Step 3: Verify formatting and compilation**

Run:

```bash
cargo fmt --check
cargo check -p chronos
```

Expected: both commands exit 0; pre-existing warnings may remain.

### Task 2: Verify both themes live and report

**Files:**
- Create: `docs/orchestration/tasks/report/T267-unified-edge-separator-report.md`

**Interfaces:**
- Consumes: release `chronos` binary and dark/light theme selection
- Produces: four `grim` captures and an evidence-based palette verdict

- [ ] **Step 1: Build and run release ChronOS**

```bash
cargo build --release -p chronos
```

Expected: exit 0. Restart only with the project CLI or `pkill -x chronos`; never use `pkill -f`.

- [ ] **Step 2: Capture dark-theme evidence**

Capture one frame containing left panel plus bar and one containing right panel plus bar. Do not perform Chronos-FM drag-out.

- [ ] **Step 3: Capture light-theme evidence**

Switch to the light theme and capture the same two compositions. Do not tune colors from memory or by eye in code.

- [ ] **Step 4: Write the report**

Record exact image paths, build result, and whether `border.subtle` remains legible against the bar in both themes. State that hover strips were checked and required no changes. If light-theme contrast fails, stop with the frames and request an architect decision on a third theme token.

- [ ] **Step 5: Commit accepted implementation and report**

```bash
git add crates/app/src/bar/mod.rs crates/app/src/side_panel_right/view.rs docs/orchestration/tasks/report/T267-unified-edge-separator-report.md
git diff --cached --check
git commit -m "ui: unify panel and bar edge separators (T267)"
```
