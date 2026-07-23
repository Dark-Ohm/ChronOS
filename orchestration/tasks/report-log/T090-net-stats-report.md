<!-- T090 — migrated 2026-07-22 from orchestration/report-log/cline-report-1.md — see orchestration/tasks/MIGRATION.md -->

# Session: Task 1 — net_stats extract — 2026-07-21

**Исход: DONE.** Сэмплинг сетевой скорости вынесен в `chronos_services::net_stats`, бар-виджет переключён, покрытие не упало.

## Сделано (факт, не намерение)

- Создан `crates/services/src/net_stats.rs`: `SAMPLE_INTERVAL`, `NetSample`, `NetState` (+`Default`), `NetSpeed { dl, ul }`, `read_interface_bytes`, `update_speed` — логика 1:1 с прежним private-кодом бара (поля переименованы по брифу).
- 5 юнит-тестов в `net_stats::tests` (3 из плана + 2 портированных: repeated same-frame + counter wrap).
- `crates/services/src/lib.rs`: **ровно** `pub mod net_stats;` — **не** в `Services`/`init_all` (чистые функции, не `Service`).
- `crates/app/src/bar/widgets/network.rs`: private sampling удалён; `use chronos_services::net_stats::{NetState as NetworkState, SAMPLE_INTERVAL, read_interface_bytes, update_speed}`; call-site `.dl`/`.ul`; 4 `update_speed_*` теста удалены (маппинг 1:1 на net_stats); `format_speed` / `indicator_color` / `compute_view` + 14 тестов остались.
- `Cargo.toml` app не трогал — `chronos-services` path-dep уже был.

## Расхождения со спекой/планом

1. **Пакет `chronos-app` в плане не существует.** В манифесте package name = `chronos`, lib name = `chronos_app`, а bar/widgets живёт только в **бинарном** `src/main.rs` (`mod bar`), не в `src/lib.rs`. Поэтому:
   - baseline: `cargo test -p chronos --bin chronos bar::widgets::network` (не `-p chronos-app --lib`);
   - `cargo test -p chronos --lib bar::widgets::network` → 0 tests (фильтр не видит bin-модуль).
2. Alias `NetSpeed as SpeedSample` из плана **не** оставил — тип нигде в файле по имени не использовался → unused import. Поля читаются как `.dl`/`.ul` у значения из `update_speed`.
3. Чужой WIP в working tree (audio `AudioStream`, `side_panel_right/`, `main.rs`) **не трогал**. В `lib.rs` чужой re-export `AudioStream` **разведён** — в staged только `pub mod net_stats`.

## Не реализовано из acceptance criteria

- Нет. Живой grim-смок бара на сетевой строке не требовался (чистый рефактор, zero behaviour change).

## Проверено фактом, не на словах

### Baseline (до переезда)

```
cargo test -p chronos --bin chronos bar::widgets::network
test result: ok. 18 passed; 0 failed; … 106 filtered out
```

Ровно 4 `update_speed_*` (grep): first_call / immunity / computes_correct / handles_counter_wrap — таблица плана, пятого нет.

### После переезда

```
cargo test -p chronos-services --lib net_stats
running 5 tests
… first_sample… ok
… sample_within_min_interval… ok
… second_sample_after_interval… ok
… repeated_calls_in_one_frame… ok
… counter_wraparound… ok
test result: ok. 5 passed; 0 failed; … 138 filtered out
```

```
cargo test -p chronos --bin chronos bar::widgets::network
test result: ok. 14 passed; 0 failed; … 106 filtered out
```

14 = baseline 18 − 4 перенесённых. Покрытие sampling: 5 в services (4 порта + 1 новый regression `sample_within_min_interval`).

```
cargo test -p chronos-services --lib
test result: ok. 143 passed; 0 failed; 0 ignored; 0 measured
```

```
cargo build --workspace
Finished `dev` profile … EXIT 0
```

Warning’ов `unused import` / `unused` **в `network.rs` / `net_stats.rs` — ноль** (чужие warning’и в mpris/notifications/tray_menu — pre-existing, не зона).

### Зона / git

```
git status --short (только зона Task 1):
 M crates/app/src/bar/widgets/network.rs
 M crates/services/src/lib.rs          # единственная дельта: +pub mod net_stats;
?? crates/services/src/net_stats.rs
```

`git diff crates/services/src/lib.rs` — одна строка `+pub mod net_stats;`.

## Новые риски / известные баги

- Два независимых `NetState` (бар Mutex + будущая панель) — **задумано**: у каждого потребителя свой sampler. Не shared-singleton.
- `read_interface_bytes` по-прежнему синхронный procfs из render/ticker — как было; не async-регресс.

## Следующий шаг

Task 1 закрыт. Панель (Task 7+) может `use chronos_services::net_stats::{NetState, SAMPLE_INTERVAL, read_interface_bytes, update_speed}` без копипасты.
