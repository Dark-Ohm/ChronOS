# Known Documentation Discrepancies (verified 2026-07-10)

Audit of ARCHITECTURE.md / MEMORY.md / DECISIONS.log against actual code in
`crates/`. Severity ratings from Philip audit workflow.

## 🔴 Critical

### ARCHITECTURE.md §8: "LuaU is NEVER in the render path" — WRONG

Doc claims:
> LuaU is NEVER in the render path. Widgets render in Rust; LuaU only on events

Actual code (`crates/luau/src/dsl.rs:236-241`):
`LuaWidgetAdapter::render()` calls `self.lua.globals().get::<Function>(render_fn_name).call(())`
on EVERY render invocation. Lua IS in the render path for every LuaU-backed widget.

Impact: the "< 4 ms synchronous LuaU call budget" (same section) has no enforcement
and the architectural guarantee it relies on is false. For 144 FPS this matters.

Status: **needs decision** — either fix the doc to reflect reality, or restructure
the widget render path to cache Lua output in Rust and only update on events.

## 🟡 High

### ARCHITECTURE.md §3 + MEMORY.md: "inotify hot-reload watcher — NOT YET IMPLEMENTED"

Both docs (last updated 2026-07-09) claim the watcher is not implemented.
Actual code: `crates/luau/src/watcher.rs` (192 lines) is fully implemented and
wired via `PluginManager::start_watcher()` called in `main.rs:53`.

Status: **stale docs** — watcher was implemented and merged but §3/§9 were not
updated to reflect completion.

### ARCHITECTURE.md §7: Service trait signature ≠ code

Doc says: `trait Service { type Data; fn subscribe(); fn status(); fn dispatch(); }`

Actual (`crates/services/src/lib.rs:55-62`):
- `type Error: Send + Sync + 'static` — present in code, absent from doc
- `fn get(&self) -> Self::Data` — present in code, absent from doc
- `fn dispatch()` — NOT in trait (only on CompositorSubscriber as concrete method)
- `Send + Sync + 'static` bounds on trait — not mentioned in doc

## 🟠 Medium

### ARCHITECTURE.md §8: "< 4 ms" budget — aspiration, not enforced

No timing measurement, assertions, or budget tracking exists in code.
The claim is architectural intent, not a verified property.

### crates/ui — documented as PLANNED, zero skeleton exists

Not in workspace members, no Cargo.toml, no src/. The doc is honest ("PLANNED")
but someone reading §3 might assume at least a stub exists.

## 🟢 Low

### manager.rs debug output

`eprintln!("scan_dir: ...")` calls at lines 50, 60, 68 — debug noise in production path.

### Capability-gated API modules are stubs

`mod_fs.rs`, `mod_process.rs`, `mod_net.rs`, `mod_ipc.rs` — all return empty tables.
Doc §5 describes them as functional modules; they compile but do nothing.
