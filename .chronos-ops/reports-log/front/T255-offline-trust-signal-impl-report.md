# T255 — offline trust signal: implementation report

**Date:** 2026-08-05
**Commit:** `ui : offline trust signal — ACP settings + About (T255)`

## Summary

Implemented two static muted mono text labels as the "offline trust signal" per the approved T240 design.

## Changes

### 1. ACP settings tab (`crates/app/src/side_panel_right/tab/acp_settings.rs`)

- Added second mono subtitle line under the existing `{n} agent(s) · agents.toml`:
  ```
  local only · no network · no telemetry
  ```
- Updated `HEADER_H_PX` from `62.0` → `78.0` to account for the second `text_xs` line (title + gap + subtitle1 + gap + subtitle2 + borders). Comment documents the calculation.
- Style matches existing subtitle: `theme.text.muted`, `text_xs()`, `font_family(theme.font_mono)`.

### 2. Bar Settings → About (`crates/app/src/side_panel_right/tab/bar_settings.rs`)

- Added fourth `justify_between` row in the existing About card:
  - Left: `offline by design` (muted, `text_xs`)
  - Right: `no network · no telemetry` (muted, `text_xs`, mono)
- Same pattern as the three existing rows (version, license, runtime).

## Verification

| Check | Result |
|-------|--------|
| `cargo build --release -p chronos` | ✅ Clean (only pre-existing warnings) |
| `cargo test --release -p chronos --lib -- side_panel_right` | ✅ 171/171 passed |
| Visual (ACP settings @ 320px) | Header height = 78px, both subtitle lines visible, card stretches to viewport bottom (HEADER_H_PX updated) |
| Visual (Bar Settings → About) | New line renders in-card, same style as siblings, no overflow |

## Notes

- No new components, badges, or icons — purely text as specified.
- English used for consistency with existing UI (`agent(s)`, `agents.toml`, `Open agents.toml`).
- HEADER_H_PX bump ensures T249's card-stretch-to-bottom logic remains correct on short agent lists at 320px panel width.