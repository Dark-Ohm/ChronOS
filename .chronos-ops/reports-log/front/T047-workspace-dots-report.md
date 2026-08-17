<!-- T047 — migrated 2026-07-22 from docs/orchestration/report-log/cline-report-11.md — see docs/orchestration/tasks/MIGRATION.md -->

# Cline №11 Report — workspace-индикатор: точки вместо номеров

**Status:** Implementation complete, build verified. UX smoke pending (requires user).

## Changes Made

### `crates/app/src/bar/widgets/workspaces.rs`

Переписал render() чтобы вместо badges с номерами/именами показывались точки:
- Размер: 7px (`DOT_SIZE` const)
- Форма: `rounded_full()` — круг
- Цвет активной: `theme.accent.primary` (#007acc)
- Цвет неактивной: `theme.text.disabled` 
- Gap: 5px (согласно мокапу)
- Click остался — `CompositorCommand::FocusWorkspace(id)`

## Verification

### Build
```
cargo check -p chronos → OK (warnings only в чужих зонах)
```

### Live D-Bus (Hyprland)
```
$ hyprctl activeworkspace
workspace ID 1 (...)
```

## Pending UX smoke (requires user per HANDOFF и брифа)

```bash
# 1. Start shell
RUST_LOG=info ./target/release/chronos

# 2. Grab screenshot with grim
grim screenshot.png

# 3. Verify dots visible in bar, active dot highlighted in #007acc

# 4. Click other dot → workspace switches
hyprctl activeworkspace  # before
# click other dot
hyprctl activeworkspace  # after — should show different workspace ID

# 5. Check logs: no error/panic messages
```