# Server-side zbus 5.17 in ChronOS `crates/services`

Verified against installed source `~/.cargo/registry/src/index.crates.io-*/zbus-5.17.0/src`
during the 2026-07-17 `notification` daemon build (first server-side zbus in this repo).

## The one trap that wastes the most time

zbus 5.17's `object_server` dispatches `#[interface]` methods on **its own
executor thread**, NOT on the tokio runtime — even with the `tokio` feature
enabled (under `#[cfg(feature="tokio")]` the `Executor` is a phantom that
delegates to `tokio::spawn`, but the *dispatch* still happens off-runtime).

Consequence: any `tokio::spawn(...)` or `tokio::sync::Mutex::lock().await`
called from inside a handler panics with:
```
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```
and the method never sends its reply -> caller sees `DBus.Error.NoReply`.

**Fix pattern (verified working):**
- In the sync `new()` (called inside `rt.block_on` per our `Service` convention),
  capture `let handle = Handle::current();` and store it on the subscriber.
- For in-handler shared state (e.g. a monotonic id allocator), use `std::sync::Mutex`
  and `.lock().unwrap()` — NO `.await`, NO reactor needed.
- To arm a delayed action (e.g. notification expiry), spawn onto the real runtime:
  `self.runtime.spawn(async move { tokio::time::sleep(...).await; ... });`
  This is the spec-compliant "expire via tokio timer, not std::thread, not nested
  runtime" — the `Handle` is the bridge off the zbus thread.
- `arm_expiry` becomes a **sync** fn (no `.await` at call site).

## API facts confirmed from source (don't guess)

- `Connection::session().await?` — connect to session bus.
- `conn.object_server().at("/path", iface_impl).await?` — register interface.
  Returns `Result<(), zbus::Error>`.
- `conn.request_name_with_flags(name: &str, flags: BitFlags<RequestNameFlags>)`
  — flags from `enumflags2` bitflags enum. Build:
  `BitFlags::from(RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement)`.
  Do NOT pass a bare `RequestNameFlags` value.
- `Connection::closed()` — **does NOT exist in 5.17.** Keep-alive strategy:
  hold `conn` in a `Mutable<Option<Connection>>` so it's never dropped, and
  park the serve task on `std::future::pending::<()>().await`. The object
  server is driven by the connection's internal executor.
- Signals: in the `#[interface]` impl,
  `#[zbus(signal)] async fn notification_closed(emitter: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;`
  Emit: `let emitter = SignalEmitter::new(&conn, OBJECT_PATH)?; Self::notification_closed(&emitter, id, reason).await;`
- FDO method return types are ordinary (`u32`, `Vec<String>`,
  `(String,String,String,String)`). `Notify` returns `u32` (the assigned id).

## Ordering

1. `object_server().at(...)` FIRST (so methods have a registered interface),
2. THEN `request_name_with_flags(...)`.
Otherwise zbus logs:
`Requesting name '...' before setting up the object server. Method calls
arriving before interfaces are registered may be lost.`

## Live smoke recipe (when `app` won't build)

If `crates/app` is unbuildable (e.g. `Source/gpui` fork regression), the
daemon still needs a host. Add `crates/services/examples/<name>-smoke.rs`:
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let services = chronos_services::init_all();   // calls NotificationSubscriber::new()
    let notif = services.notification.clone();
    let mut stream = notif.subscribe().to_stream(); // needs futures_signals::signal::SignalExt
    tokio::spawn(async move { while let Some(s) = stream.next().await { /* log */ } });
    // idle: std::future::pending::<()>().await;
}
```
Probe from another shell:
- `busctl --user list | grep org.freedesktop.Notifications` -> must show YOUR pid.
- `gdbus introspect --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications` -> returns the interface (proves the server is live + responsive).
- `dbus-send --session --print-reply --dest=org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications.GetServerInformation` -> returns our `('ChronOS Notifications','ChronOS','0.1.0','1.2')`.
- `notify-send "t" "b"` -> if it errors "name is not activatable" but `busctl` shows ownership, that's a libnotify activation quirk, NOT a daemon bug.

## Example round-trip that worked
- `dbus-send ... Notify ...` -> `method return ... uint32 2` (assigned id).
- state log showed `id=1` then `id=2`; first notification expired after its
  timeout (active=2 -> active=1) -> proves the `Handle::spawn` timer path.
