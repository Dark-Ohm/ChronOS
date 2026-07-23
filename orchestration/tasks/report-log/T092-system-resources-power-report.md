<!-- T092 — migrated 2026-07-22 from orchestration/report-log/grok-report-19.md — see orchestration/tasks/MIGRATION.md -->

# Session: Tasks 3+4 system_resources (CPU/RAM/GPU) — 2026-07-21

## Сделано (факт, не намерение)

- `crates/services/src/system_resources/types.rs` (новый): `SystemResourcesState { cpu_percent, ram_percent, gpu_percent: Option<f32> }` — `PartialEq`, без `Eq` (float).
- `crates/services/src/system_resources/mod.rs` (новый): `sample_cpu_ram`, `sample_gpu`, `SystemResourcesSubscriber` + `Service` impl, poll-луп 1с, NVML init один раз вне лупа.
- `Cargo.toml` (workspace.dependencies): `sysinfo = "0.39.3"`, `nvml-wrapper = "0.12.1"`.
- `crates/services/Cargo.toml`: `sysinfo.workspace = true`, `nvml-wrapper.workspace = true`.
- `crates/services/src/lib.rs`: `pub mod system_resources;`, re-export, поле `Services.system_resources`, строка в `init_all`.
- `crates/app/src/state.rs`: accessor `AppState::system_resources(cx)`.
- **Не коммитил** (по брифу).

## Shared-файлы — ЯВНЫЙ список добавленных строк (только мои)

Параллельно в дереве уже лежит **незакоммиченный GLM Task 5** (`crates/services/src/power/` + проводка `power` в `lib.rs`/`state.rs`). Я **не** создавал power; ниже — только то, что добавил я. Архитектор при приёмке может вытащить только эти строки, если power ещё не в master.

### `Cargo.toml` (корень)
```toml
sysinfo = "0.39.3"
nvml-wrapper = "0.12.1"
```
(после `serde_json = "1"`)

### `crates/services/Cargo.toml`
```toml
sysinfo.workspace = true
nvml-wrapper.workspace = true
```
(после `serde_json.workspace = true`)

### `crates/services/src/lib.rs`
- `pub mod system_resources;` (рядом с `power` / `tray`)
- `pub use system_resources::{SystemResourcesState, SystemResourcesSubscriber};`
- в `Services`: `pub system_resources: SystemResourcesSubscriber,`
- в `init_all()`: `system_resources: SystemResourcesSubscriber::new(),`

### `crates/app/src/state.rs`
```rust
    #[inline(always)]
    pub fn system_resources(cx: &App) -> &chronos_services::SystemResourcesSubscriber {
        &Self::global(cx).services.system_resources
    }
```
(после accessor `power`, который уже был от GLM)

### Полностью мои файлы
- `crates/services/src/system_resources/mod.rs`
- `crates/services/src/system_resources/types.rs`

## Расхождения со спекой/планом

- План Task 3 Step 11 / Task 4 не предусматривал совместный коммит — бриф №19: **не коммитить**. Сделано.
- Юнит-тест `sample_cpu_ram_reads_real_host_values_in_range`: добавил warm-up sleep 200мс + второй sample (первый после `System::new_all()` часто 0.0 по доке sysinfo). Инвариант [0,100] тот же; план-тест без sleep тоже прошёл бы на 0.0.
- `sysinfo` в lock разрешился в **0.39.6** (semver от `0.39.3`) — API `refresh_cpu_usage` / `global_cpu_usage` совпал с планом, эскалация не нужна.
- Worktree-изоляция: не делал — дерево уже держало GLM power; добавил свои строки рядом, не затирая power.

## Не реализовано из acceptance criteria

- UI спектр-баров (Task 10) — не моя зона, backend only.
- Watch в `bar/mod.rs` на `system_resources` — не просили, UI ещё нет.
- Коммит в git — запрещён брифом.

## Проверено фактом, не на словах

```
$ cargo test -p chronos-services --lib system_resources -- --nocapture
test system_resources::tests::gpu_sample_none_when_nvml_unavailable_does_not_panic ... ok
test system_resources::tests::sample_cpu_ram_reads_real_host_values_in_range ... ok
# 2 passed

$ cargo test -p chronos-services --lib
# 148 passed; 0 failed  (было 146 + 2 новых)

$ cargo build --workspace
# Finished `dev` profile, EXIT 0 (pre-existing warnings only, not mine)

$ nvidia-smi --query-gpu=utilization.gpu,memory.used --format=csv,noheader
6 %, 2896 MiB

# живой one-shot (тот же API: sysinfo + nvml device 0 utilization):
cpu=34.4% ram=54.8% gpu=Some(6.0)
# GPU % совпал с nvidia-smi (6)
```

## Новые риски / известные баги

- **Shared-файлы смешаны с GLM power WIP** (severity): `lib.rs`/`state.rs` в diff vs HEAD содержат и power, и system_resources. Severity: medium для приёмки — Архитектор должен стейджить/применять построчно, не `git add` целиком shared без глаз.
- **Первый tick CPU может быть 0.0** — документированное поведение sysinfo, не баг; панель (Task 10) увидит нормальные значения со 2-го тика (~1с).
- **NVML device 0 only** — multi-GPU не в скоупе; на этой машине (RTX 3070) ок.
- `Cargo.lock` изменился (sysinfo/nvml-wrapper + transitive) — нужен в коммите проводки.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлял. Backend-сервис по существующему `Service`-паттерну; новые архитектурные решения не вводил. Синхрон доков — на Архитекторе при приёмке/коммите капстоуна.

## Исход (одной строкой)

**Tasks 3+4 сданы: `system_resources` CPU/RAM/GPU, 148 services tests green, live GPU Some(6.0) == nvidia-smi; коммит не делал.**
