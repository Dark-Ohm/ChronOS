# Task 2 Review: Hermes ACP Service — Transport

**Reviewer:** Lead Architect Agent  
**Commit:** `e33ff4f feat(hermes_acp): transport layer with stdio spawn`  
**Date:** 2026-07-23

---

## Verdict: PASS (with minor notes)

---

## Requirement Checklist

| # | Requirement | Status | Notes |
|---|-------------|--------|-------|
| 1 | `hermes_acp/transport.rs` created | ✅ | 90 lines, well-structured |
| 2 | `hermes_acp/mod.rs` created | ✅ | Correct re-exports |
| 3 | `lib.rs` wired up | ✅ | `pub mod hermes_acp;` added |
| 4 | `Cargo.toml` updated | ✅ | Both ACP crates added |
| 5 | `HermesTransport` struct produced | ✅ | Spawn + Drop + shutdown |
| 6 | Compiles clean | ✅ | Zero new warnings (1 pre-existing mpris warning) |
| 7 | Commit present | ✅ | Single clean commit |

---

## Deviations Review

### 1. SDK versions 0.5 → 0.11.1 — **JUSTIFIED**

The plan assumed `agent-client-protocol = "0.5"`. Actual available versions on crates.io: `0.11.1`. Using 0.5 would have failed at `cargo add`. The tokio integration crate (`agent-client-protocol-tokio`) also exists only at 0.11.1. No 0.5 release exists for the tokio crate. **This is not a deviation — the plan was written against stale data.**

### 2. `StdioTransport` → `AcpAgent` — **JUSTIFIED**

The task brief referenced `StdioTransport::spawn()`. In 0.11.x, `StdioTransport` wraps the current process's stdin/stdout. Subprocess spawning is handled by `AcpAgent` (which uses `shell-words` to parse the command string). The implementer adapted correctly to the actual SDK API. The connection flow is equivalent to what the brief intended.

### 3. `Client::builder()` → `Client.builder()` — **JUSTIFIED**

In 0.11.x, `Client` is a unit struct. `Client::builder()` (associated function) and `Client.builder()` (instance method) are equivalent for unit structs, but the compiler required the instance form. Non-issue.

### 4. Missing `inner()` method — **ACCEPTABLE**

The brief specified `pub fn inner(&self) -> &StdioTransport`. Since `StdioTransport` doesn't exist and the connection runs entirely inside the background tokio task, exposing the transport directly isn't useful at this stage. The `_handle` field is correctly kept private. Can be re-introduced when Task 3/4 needs channel access.

### 5. `hermes_acp.rs` module declaration not created — **BRIEF WAS WRONG**

The task brief Step 4 said to create `crates/services/src/hermes_acp.rs` with `pub mod hermes_acp;`. This would be a **module named `hermes_acp` containing a submodule named `hermes_acp`** — infinite recursion. The correct approach (which the implementer used) is adding `pub mod hermes_acp;` to `lib.rs`. The implementer silently fixed a bug in the task brief. Good.

### 6. `client.rs` and `session.rs` deferred — **ACCEPTABLE**

The brief's Step 3 declared `pub mod client; pub mod session;` in mod.rs. These files don't exist yet. The implementer didn't declare phantom modules, which would cause compilation errors. Correctly deferred to Task 3+.

---

## Code Quality

### What's good

- **Clean struct layout.** `HermesTransport` is minimal — just the join handle. No premature abstraction.
- **Drop implementation.** Correctly aborts the background task on drop. Prevents orphaned ACP connections.
- **`HermesConfig` with `Default`.** Good pattern for future configurability (custom hermes binary path, extra args).
- **Logging at key points.** `debug!` on spawn, `info!` on init success, `error!` on termination. Appropriate levels.
- **Error conversion.** The `AcpError::internal_error().data(e.to_string())` mapping inside `connect_with` is correct — the closure must return `Result<_, AcpError>`, not `anyhow::Error`.
- **`std::future::pending` keepalive.** Clean way to keep the connection alive without busy-looping. Dropped when `_handle` is aborted.

### What could be better

1. **`_handle` naming.** The underscore prefix is a Rust convention for "intentionally unused field." The field IS used (in `shutdown()` and `Drop`). Should just be `handle`. Minor style issue.

2. **`command_str` joining is fragile.** `format!("{} {}", config.command, config.args.join(" "))` doesn't handle args with spaces. `AcpAgent::from_str` uses `shell-words` internally, so this actually works for simple cases, but it's worth noting. If args ever need quoting, this will break silently.

3. **`shutdown()` + `Drop` double-abort.** Calling `shutdown()` then dropping the struct calls `.abort()` twice on the same handle. Harmless (aborting an already-aborted task is a no-op), but redundant. Either `shutdown()` should consume self, or Drop should use `JoinHandle::abort()` which already checks.

4. **No retry logic.** The `connect_with` call is fire-and-forget. If the Hermes process isn't installed or crashes immediately, the background task logs an error and exits. The report mentions this is by design (transport layer only, no message forwarding yet). Acceptable for Task 2 scope.

5. **ACP async runtime concern.** The report correctly flags that ACP core uses `async-io` internally while ChronOS uses tokio. This coexistence is fine as long as ACP doesn't block on smol-specific executors inside the `connect_with` closure. Worth monitoring in Task 3 when actual message passing begins.

---

## Cargo.lock Bloat

The diff adds ~1700 lines of Cargo.lock changes — `rmcp`, `serde_with`, `time`, `darling 0.23`, `indexmap 1.x` (indirect), etc. These are transitive dependencies of ACP 0.11.1. The lockfile bloat is unavoidable given the ACP SDK's dependency tree. Not a concern.

---

## Build Verification

```
cargo check -p chronos-services   ✅ (0 new warnings)
```

Pre-existing warning: `unused imports: Array and Str` in `crates/services/src/mpris/mod.rs:19`. Unrelated.

---

## Summary

Solid transport layer. The deviations from the task brief are all justified (SDK version changes, API adaptations, and one brief bug fix). Code quality is clean with minor style notes. No blockers for Task 3.

**Ship it.**
