# Task 4: Service Registration — Report

## Status: DONE

## Files modified

| File | Action |
|---|---|
| `crates/services/src/lib.rs` | `pub mod hermes_acp;` added (line 13) |
| `crates/app/Cargo.toml` | `chronos-services = { path = "../services" }` already present (line 26) |

## Notes

Both changes were already present in the worktree before this task started:

- `pub mod hermes_acp;` was added in commit `e33ff4f` (feat(hermes_acp): transport layer with stdio spawn).
- `chronos-services` dependency in `app/Cargo.toml` predates the hermes_acp work entirely.

No new code was written. Verified compilation succeeds with `cargo check -p chronos` (only pre-existing warnings, no errors).

## Test results

```
cargo check -p chronos-services  — OK (1 pre-existing mpris warning)
cargo check -p chronos           — OK (30 pre-existing warnings, 0 errors)
```

## Commit

No new commit needed — changes were already committed as part of earlier hermes_acp work.
