<!-- T094 — SUPERSEDED draft, migrated 2026-07-22 from docs/orchestration/report-log/hermes-report-7-duplicate.md — canonical version is in docs/orchestration/tasks/report-log/, see docs/orchestration/tasks/MIGRATION.md -->

# HERMES — отчёт по заданию №7 (services follow-ups)

_Дата: 2026-07-17. Сессия после ребута (контекст из HERMES.md + docs/HANDOFF.md)._
_Автор правок: Hermes (зоны services/network, services/upower + bar/widgets network/battery)._

## Статус
Задачи 1–3 выполнены, код закоммичен (3 коммита ниже). Мои модули
компилируются и проходят lib-тесты зелёными. **Полный `--workspace` сейчас
красный НЕ из-за меня** — параллельный агент (Mimo) дописывает `launcher`
и `applications`, бинарь `chronos` из-за этого не собирается. Мою зону это
не затрагивает.

## Коммиты (в истории, поимённо, по зонам)
```
1d7e285 network : поле wired + фикс idle-флапа
60c09c7 upower : поле has_battery
a22b53f bar : network/battery на честных полях
```
Поверх накатили соседи (applications, upower-эррата, tray-иконки) — мои
изменения в дереве сохранены и целы.

## Задача 1 — NetworkData.wired + фикс idle-флапа
- `network/types.rs`: `pub wired: bool`.
- `network/mod.rs`: `detect_wired()` читает NM `ActiveConnections`, ищет
  `Type == "802-3-ethernet"`. Заполняется при коннекте и на каждом
  изменении connectivity.
- **Фикс флапа**: убран idle-таймаут на стриме сигналов (давал
  «signal timeout; retrying» ~раз в 60с на тихой сети). Вместо него —
  heartbeat-пинг раз в 30с; реконнект только при реальной ошибке пинга.
  Тихий период ≠ обрыв.

## Задача 2 — UPowerData.has_battery
- `upower/types.rs`: `pub has_battery: bool`.
- `upower/mod.rs`: `detect_has_battery()` — основной сигнал
  `EnumerateDevices` + `Type == 2` (Battery); fallback `DisplayDevice.IsPresent`.
- `bar/widgets/battery.rs`: `!has_battery → пустой div`. Старая эвристика
  (Unknown+0%) оставлена вторым рубежом (решение зафиксировано).

## Задача 3 — network-флап
Сведена к Задаче 1 (heartbeat вместо таймаута). Паразитный `window not found`
в gpui должен уйти вместе с флапом — эмпирически в живом прогоне не проверял
(бинарь сейчас не собирается из-за launcher-WIP соседа).

## Верификация (свежая, эта сессия)
- `cargo check -p chronos-services` → Finished, 0 ошибок. Мои файлы
  (network/upower + их types) компилируются чисто.
- `cargo test -p chronos-services --lib` → **50 passed; 0 failed.**
- Живой D-Bus факт-чек (busctl, системная шина, этот десктоп):
  - NM `Connectivity=4`; `ActiveConnection/2 Type="802-3-ethernet"`,
    `/1 Type="loopback"` ⇒ detect_wired ⇒ **true** ⇒ бар покажет «eth».
  - UPower `EnumerateDevices` ⇒ пусто (`ao 0`); `DisplayDevice.IsPresent`
    ⇒ `b false` ⇒ has_battery=**false** ⇒ battery-виджета нет.
  Оба честных поля совпали с реальностью машины.

## Блокер (не мой)
`cargo check -p chronos --bin chronos` → красный:
`cannot find cache/entry in launcher`. `main.rs` импортирует
`crate::launcher::{cache,entry}`, но `launcher/mod.rs` объявляет только
`launch/search/view` — это WIP Mimo, вне моей зоны (HERMES.md: не трогать
launcher). Поэтому:
- бинарные unit-тесты (в т.ч. мои bar-widget `describe()` тесты) сейчас
  не запускаются; они проходили зелёными ранее в этой сессии до того, как
  сосед сломал сборку, а мой widget-код не менялся;
- полный `--workspace` станет зелёным по моим модулям автоматически, как
  только Mimo довёдет launcher/applications.

## Нерешённое
Живой 5-мин release-смок «без signal timeout» не прогнан (бинарь не
собирается + вне GUI-сессии уходит пустым логом). По коду флап устранён
(heartbeat); эмпирическая проверка — за тобой: `./target/release/chronos`
в графической сессии после разблока сборки.

## Touched files (мои, абсолютные пути)
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/network/mod.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/network/types.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/upower/mod.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/services/src/upower/types.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/app/src/bar/widgets/network.rs
- /home/neo/projects/chronos-ecosystem/ChronOS/crates/app/src/bar/widgets/battery.rs

## От соседей
Mimo: довести `launcher` (подмодули cache/entry) и `applications` →
разблок бинаря и workspace-тестов. В его зону не лезу.
