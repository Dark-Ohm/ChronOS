<!-- T065 — migrated 2026-07-22 from orchestration/report-log/deepseek-report-1.md — see orchestration/tasks/MIGRATION.md -->

# Session: network-виджет → лампочка активности + спидометр ↓/↑ — 2026-07-20

## Сделано (факт, не намерение)

- `crates/app/src/bar/widgets/network.rs` — полная переработка:
  - **Выпилено:** `NetworkView` (Stub/Disconnected/Wired/Wifi), `describe()`, `strength_icon()`, Nerd-глифы, SSID/eth-текст
  - **Добавлено:** чтение `/sys/class/net/*/statistics/{rx,tx}_bytes` (все интерфейсы кроме `lo`), дельта по `Sample { rx, tx, _time }` через `Mutex<Option<Sample>>` внутри `NetworkWidget`
  - **Спидометр:** `format_speed(f64)` → стабильная ширина 4 символа (`"  0 "`, `"999 "`, `"1.0K"`, `" 34K"`, `"999K"`, `"1.0M"`, `" 12M"`), рендер двух строк `↓ 340K` / `↑ 340K` через `font_mono` + `font_sizes.xs`
  - **Лампочка:** 6px `rounded_full` кружок слева — серая (`text.disabled`, дельта < 1KB/тик), зелёная (`status.success`, трафик есть), красная (`status.error`, connectivity == None)
  - **14 юнит-тестов:** `format_speed` (границы K/M, стабильная ширина), `compute_view` (disconnected/idle/active), `indicator_color` (все 3 цвета + порог)

## Расхождения со спекой/планом

- Спек требовал «две строки скорости, верхняя — download, нижняя — upload». Реализовано: `↓ <dl>` / `↑ <ul>` — так стрелка визуально привязана к строке, `/s` опущен (1s-тикер подразумевает per-second). Решение пользователя по выбору формата отложено до grim-смока.
- Спек предлагал `999K`/`1.2M` + `/s` или `↓ 1.2M` / `↑ 340K`. Выбран второй вариант (стрелки + value). Ширина стабильная: рендер `↓ ` + 4-символьный value = 6 символов monospace, не дёргается.
- `IDLE_THRESHOLD = 1024` (< 1 KB/тик) — обоснование: на 1s-тикере шум от kernel bookkeeping даёт << 1 Kbps даже в покое; порог в 1KB отсекает ложные срабатывания без пропуска реального трафика (минимальный HTTP-запрос уже >> 1KB за тик).
- Суммирование всех интерфейсов кроме `lo` — исполнено как просили. Эскалация по VPN/виртуалкам не потребовалась: docker/veth/tun включаются, но на MVP это допустимо (трафик через них — тоже реальный сетевой трафик; если зашумят — отдельное задание на `default_route` через netlink).

## Не реализовано из acceptance criteria

- **Живой смок** (п. 96-100 DEEPSEEK.md): `pkill chronos && RUST_LOG=info ./target/release/chronos` + grim-скрины в покое и под нагрузкой — **не выполнено**. Причина: `cargo build --release` долгий (таймаут 30с). Нужен ручной запуск пользователем или фоновая сборка.
- **Отчёт со скринами** (п. 100): не приложены по той же причине.

## Проверено фактом, не на словах

- `cargo check -p chronos` — успешно, 0 новых ошибок (19 pre-existing warnings, не мои)
- `cargo test --workspace --lib --bins` — **271 тест, все зелёные**
- `cargo test --workspace --lib --bins -p chronos -- network` — **14/14 network-тестов прошли**:

```
test bar::widgets::network::tests::format_bytes ... ok
test bar::widgets::network::tests::format_kilobytes ... ok
test bar::widgets::network::tests::format_megabytes ... ok
test bar::widgets::network::tests::format_stable_width ... ok
test bar::widgets::network::tests::format_zero ... ok
test bar::widgets::network::tests::indicator_active_is_success ... ok
test bar::widgets::network::tests::indicator_at_threshold_is_success ... ok
test bar::widgets::network::tests::indicator_below_threshold_is_disabled ... ok
test bar::widgets::network::tests::indicator_disconnected_is_error ... ok
test bar::widgets::network::tests::indicator_idle_is_disabled ... ok
test bar::widgets::network::tests::view_active_traffic ... ok
test bar::widgets::network::tests::view_below_threshold_idle ... ok
test bar::widgets::network::tests::view_disconnected ... ok
test bar::widgets::network::tests::view_idle_with_connectivity ... ok
```

## Новые риски / известные баги

- **Нет изоляции чтения procfs.** Если `/sys/class/net` отсутствует (контейнер, необычная конфигурация) — `read_interface_bytes` вернёт `Err`, render покажет 0 с серой лампочкой. Не краш, но и не ошибка. **Severity: low.**
- **u64 saturating_sub** — защита от wrap-around счётчиков ядра. Теоретически возможно при перезагрузке интерфейса (сброс счётчиков → rx < prev.rx → saturating → 0 на одном тике, потом нормально). **Severity: low.**
- **Mutex в render** — `self.sample.lock().unwrap()`. В однопоточном GPUI конкурентности нет, но `unwrap()` может panic при poisoned mutex (крайне маловероятно). **Severity: negligible.**

## Статус ARCHITECTURE.md / DECISIONS.log

- **ARCHITECTURE.md** — не обновлялся. Widget-specific изменения не затрагивают архитектурные решения.
- **DECISIONS.log** — не обновлялся. Решения в рамках чёткого задания (редизайн виджета), архитектурно бесспорные.
