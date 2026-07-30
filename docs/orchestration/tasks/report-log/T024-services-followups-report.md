<!-- T024 — migrated 2026-07-22 from docs/orchestration/report-log/hermes-report (copy 1).md — see docs/orchestration/tasks/MIGRATION.md -->

# HERMES — отчёт по заданию №7 (services follow-ups)

_Дата: 2026-07-17. Сессия после ребута (контекст из HERMES.md + HANDOFF.md)._
_Автор правок: Hermes (зона services + bar/widgets network/battery)._

## Статус
Код готов, собирается, тесты зелёные, 3 коммита сделаны. **Живой release-смок
не доведён до 5 минут** — бинарь завершился пустым логом вне GUI-сессии
(см. «Верификация / нерешённое»). Логика флапа по коду устранена.

## Задача 1 — `NetworkData.wired: bool` + фикс idle-флапа

### Данные
`crates/services/src/network/types.rs`: добавлено `pub wired: bool`
(Default=false). Заменяет эвристику бара `Full && ssid.is_none()`.

### Детект (honest, из NM)
`crates/services/src/network/mod.rs`:
- Новый прокси `ActiveConnectionProxy` (interface
  `org.freedesktop.NetworkManager.Connection.Active`), свойство `Type`.
- `async fn detect_wired(conn)`: берёт `NetworkManager.ActiveConnections`,
  для каждого пути строит прокси и читает `Type`; возвращает `true`
  при `Type == "802-3-ethernet"`.
- `wired` заполняется при коннекте и на каждом изменении connectivity.

### Фикс флапа «signal timeout; retrying»
Причина: внутренний `tokio::time::timeout(CONNECT_TIMEOUT=5s)` висел на
`stream.next()` на idle-стриме. Тихая сеть без событий — НОРМА, но таймаут
рвал соединение и пересоздавал его каждые ~60с (5 пар WARN/INFO за 5 мин).
Решение: таймаут на idle-стриме **убран**. Вместо него `tokio::select!`
между приходом сигнала и heartbeat-пингом раз в `HEARTBEAT=30s`
(`mgr.connectivity().await.is_err()`). Реконнект теперь только при реальном
отказе пинга, а не при тишине.

## Задача 2 — `UPowerData.has_battery: bool`

### Данные
`crates/services/src/upower/types.rs`: добавлено `pub has_battery: bool`
(Default=false).

### Детект (honest, из UPower)
`crates/services/src/upower/mod.rs`:
- Новые прокси: `UPowerProxy` (`EnumerateDevices`) и `UPowerDeviceProxy`
  (`org.freedesktop.UPower.Device`, свойство `Type`).
- `async fn detect_has_battery(conn)`:
  - Fallback: `DisplayDevice.IsPresent` (на десктопе synthetic-stub → false).
  - Основной: `EnumerateDevices`, перебор, `Type == 2` (Battery) → true.
- `has_battery` заполняется при коннекте (статично для машины; на смене
  процент/состояния не пересчитывается — это корректно).

### Решение по battery-виджету (зафиксировано)
`crates/app/src/bar/widgets/battery.rs`: `!data.has_battery → пустой div`.
Старую эвристику (`Unknown`+0%) **оставил вторым рубежом** — страховка на
случай регресса детекта к Unknown/0 на реальном железе. Честное поле первично.

## Задача 3 — network-флап
См. Задача 1 (убран idle-таймаут, добавлен heartbeat). Побочный эффект —
паразитные `window not found` в gpui из-за пересоздания соединения — по идее
уходит вместе с флапом, но в живом прогоне это НЕ проверено (см. ниже).

## Верификация

### Сделано (реальный выхлоп)
- `cargo build --workspace` → Finished, 0 ошибок. (2 warning в
  `notifications/view.rs` — вне зоны, pre-existing.)
- `cargo test --workspace --lib --bins` → **116 passed; 0 failed**
  (4 + 36 + 25 + 48 + 3). В т.ч. новые:
  `bar::widgets::network::tests::wired_on_full_with_wired_flag` и
  `full_without_wired_flag_is_wifi` (регрессионный: Full без wired → Wifi,
  а не Wired).
- Живой D-Bus факт-чек на этом десктопе (busctl, системная шина):
  - NM `Connectivity=4` (Full); `ActiveConnection/2` `Type="802-3-ethernet"`
    ⇒ detect_wired ⇒ **true** ⇒ бар покажет «eth» честным полем.
  - UPower `EnumerateDevices` ⇒ пусто; `DisplayDevice.IsPresent=false`
    ⇒ has_battery=**false** ⇒ battery-виджета нет. Сошлось с ожиданием.

### Нерешённое (честно)
Живой 5-минутный release-смок НЕ прогнан до конца: `./target/release/chronos`
поднялся, отработал 43s и вышел с EXIT=0 и **пустым логом** — видимо, упал на
инициализации GUI/дисплея (нет wayland-сессии в этом терминале) до старта
вещания сервисов, либо фильтр RUST_LOG отсёк вывод. Подтвердить «5 минут без
signal timeout; retrying» в живой среде я здесь не смог. Код по логике это
гарантирует (heartbeat вместо таймаута), но эмпирически в твоей среде не
прогнал. Запусти бинарь в графической сессии и погляди лог — должно быть
чисто.

## Коммиты (поимённо, по зонам)
```
1d7e285 network : поле wired + фикс idle-флапа
60c09c7 upower : поле has_battery
a22b53f bar : network/battery на честных полях
```
Стейджил ТОЛЬКО свои 6 файлов. НЕ тронул (и не коммитил) чужие правки:
`crates/services/Cargo.toml` (`notify="8"` — Mimo),
`crates/services/src/lib.rs` (+`applications` — Mimo),
`crates/app/src/state.rs`, `grok-report.md`, удалённые `*.md` отчёты и пр.
Cargo.lock изменился только из-за `notify` (Mimo) — не коммитил.

## Touched files (абсолютные пути)
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/network/mod.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/network/types.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/upower/mod.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/upower/types.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/app/src/bar/widgets/network.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/app/src/bar/widgets/battery.rs

## Открытые вопросы / Follow-ups
- Довести живой release-смок в GUI-сессии (см. «Нерешённое»).
- При желании — unit-тест на `run`-луп с mock-DBus, чтобы флап ловился в CI
  без графики.
