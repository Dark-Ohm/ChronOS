# Re-review: Task 3 fixes (83f8925..0748914)

## Verdict: ALL FIXED

### F1: `AcpAgent::from_args` instead of `from_str` with format!
- **Before:** `format!("{} {}", cmd, args.join(" "))` → `AcpAgent::from_str(&command_str)`
- **After:** `vec![command] + extend(args)` → `AcpAgent::from_args(agent_args)`
- **Status:** Fixed. `from_args` is the correct API — no string parsing ambiguity with spaces/quotes. `use std::str::FromStr` removed from imports.

### F2: Consistent error types (anyhow::Result)
- **Before:** Mixed `Result<T, String>` via `.map_err(|e| format!("...{e}"))`
- **After:** `Result<T>` (anyhow) everywhere — `Command` enum, `create_session`, `send_prompt`, `HermesClient` methods. All error sites use `.context("...")`.
- **Status:** Fixed. Consistent `anyhow::Result` with contextual errors throughout.

### F3: `std::future::pending` instead of sleep loop
- **Before:** `loop { tokio::time::sleep(Duration::from_secs(3600)).await; }`
- **After:** `Ok::<(), AcpError>(std::future::pending::<()>().await)`
- **Status:** Fixed. Pending future is the idiomatic way to keep a spawned task alive forever without polling or timer overhead. Type-annotated to satisfy `spawn`'s `Output = Result` bound.
