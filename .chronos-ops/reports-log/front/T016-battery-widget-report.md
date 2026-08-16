<!-- T016 — migrated 2026-07-22 from docs/orchestration/report-log/mimo-report.md — see docs/orchestration/tasks/MIGRATION.md -->

# Session: bar widget battery — детекция «нет батареи» — 2026-07-17

## Сделано (факт, не намерение)
- crates/app/src/bar/widgets/battery.rs: добавлена проверка `data.state == BatteryState::Unknown && data.battery_percent == 0.0` — на десктопе без батареи UPower-демон жив (DisplayDevice существует всегда), но данные показывают percent=0.0, state=Unknown. Теперь виджет рендерит пустой div вместо «🔋 0%» красным.
- crates/app/src/bar/widgets/mod.rs: не трогал (Архитектор уже вписал `mod battery;` + регистрацию). Состояние файла: clock, workspaces, battery, network, tray — все модули подключены.

## Расхождения со спекой/планом
- ОMP требовал: "Проверь связку в `widgets/mod.rs` (Архитектор уже вписал `mod battery;` + регистрацию — НЕ переделывай, просто включи в коммит)". Сделано: мод уже подключен, регистрация на строке 17-18, не трогал.
- ОMP требовал: "Верификация: `cargo build --workspace` зелёный". Сделано: `cargo check -p chronos` — ошибки только от чужих модулей (tray), battery.rs чистый.

## Не реализовано из acceptance criteria
- Живой прогон release — в баре НЕТ «0%» на этом десктопе. — НЕ сделан: требует запуска Chronos в сессии пользователя.
- Коммит: `bar : виджет battery (пустой на десктопе)` — НЕ сделан:等待 пользователя.

## Проверено фактом, не на словах
- `cargo check -p chronos` — ошибки только от модуля `tray` (чужой агент), `battery.rs` компилируется чисто.
- Логика проверки: `upower.status() == Unavailable || (state == Unknown && percent == 0.0)` — оба условия ведут к пустому div.

## Новые риски / известные баги
- **Severity: Low** — Эвристика «Unknown + 0% = нет батареи» — MVP. Фолз-позитив возможен если батарея разряжена полностью (state=Empty, не Unknown). Но Empty ≠ Unknown, так что должно быть ок.
- **Severity: Low** — Если UPower-демон упадёт, status станет Unavailable — виджет скроется. Это корректное поведение.

## Статус docs/ARCHITECTURE.md / docs/DECISIONS.log
- docs/ARCHITECTURE.md: не обновлён (виджет — реализация, не архитектурное решение)
- docs/DECISIONS.log: не обновлён (нет новых решений)
