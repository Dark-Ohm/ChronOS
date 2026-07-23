<!-- T099 — migrated 2026-07-22 from orchestration/report-log/zed-report-7.md — see orchestration/tasks/MIGRATION.md -->

# Session: Zed №7 — Трек 2: udisks2-сервис + живая секция дисков — 2026-07-21

**Исход: DONE** (unit+live enumerate зелёные; release binary; grim с живыми дисками; mount/unmount на VTOYEFI через тот же udisks2 API; PowerOff/Eject на Ventoy **не** гонялся — данные).

## Сделано (факт, не намерение)

### `crates/services/src/udisks/` (новый)

- `types.rs` — `DiskInfo` (`label`, `size_label`, `fraction`, `removable`, `ejectable`, `mount_point`, `block_path`, `drive_path`), `DisksCommand::{Mount,Unmount,Eject}`, чистые: `format_bytes`, `size_label`, `usage_fraction`, `pick_mount_point`, `decode_ay`/`decode_mount_points`, `device_basename` + 7 unit-тестов.
- `mod.rs` — zbus-прокси (`Manager`/`Block`/`Filesystem`/`Drive` на system bus), `DisksSubscriber` (`Service<Data=Vec<DiskInfo>>`, `Mutable`, poll 2.5s + reconnect backoff), `dispatch`/`mount`/`unmount`/`eject` fire-and-forget, `enumerate_disks` (фильтр `IdUsage==filesystem` && !`HintIgnore`, usage через sysinfo/statvfs по mount), eject = `Drive.Eject` если ejectable, иначе `Drive.PowerOff`. Live-тест `live_enumerate_sees_filesystem_devices`.

### Проводка (shared — минимально)

- `crates/services/src/lib.rs` — `pub mod udisks;`, re-export `DiskInfo`/`DisksCommand`/`DisksSubscriber`, поле `Services.udisks`, `init_all` → `DisksSubscriber::new()`.
- `crates/app/src/state.rs` — `AppState::disks(cx) → &DisksSubscriber`.
- `crates/app/src/side_panel_right/view.rs` — поле `disks: Vec<DiskInfo>`, `subscribe`/`watch`, `render_disks_section(&self.disks, cx)`.
- `crates/app/src/side_panel_right/disks.rs` — живой рендер: usage-карточка на каждый `DiskInfo` (бар `#a6e3a1`); **internal** — только полоса; **removable** — mount/unmount/eject (`on_click` → `DisksCommand`); пустой список → «нет дисков»; disabled-состояние кнопок (mount когда уже смонтирован, unmount когда нет).

### Не коммитил (по брифу).

## Расхождения со спекой/планом

| План | Факт | Почему |
|---|---|---|
| Usage «statvfs по mount point» | sysinfo `Disks` (Linux = statvfs/proc под капотом) | `unsafe_code = deny`, nix/libc statvfs не тащили; dep уже в crate |
| `DiskInfo` без `drive_path` | добавлен `drive_path: Option<String>` | Eject/PowerOff требуют path drive, не block |
| `ejectable` = Drive.Ejectable | `ejectable = Ejectable \|\| (Removable && CanPowerOff)` | На этой машине USB HDD: `Ejectable=false`, `CanPowerOff=true` — иначе кнопка «извлечь» бесполезна |
| Watch `InterfacesAdded/Removed` | poll 2.5s | MVP; бриф: «watch лучше, poll ок» |
| Живой клик eject/PowerOff на USB | **не** делал | Единственный съёмный — Ventoy с данными; unmount/mount проверен на `VTOYEFI` (32M EFI) |

## Не реализовано из acceptance criteria

- UI-клик eject/PowerOff до физического отключения USB — нет spare-флешки; API-ветка написана, live-факты `Ejectable`/`CanPowerOff` сняты busctl.
- D-Bus signal watch вместо poll — follow-up.
- Hotplug-смок «воткни USB → появился» — USB уже был вставлен; poll+enumerate это покроет, отдельный plug-cycle не гонялся.

## Проверено фактом, не на словах

```
cargo test -p chronos-services --lib udisks -- --nocapture
# 9 passed (8 pure + 1 live_enumerate_sees_filesystem_devices)
# live: DisksSubscriber path → 3 filesystem devices

cargo build --workspace
# Finished dev profile, exit 0

cargo build --release -p chronos
# Finished release profile, ~2m29s, exit 0

RUST_LOG=info CHRONOS_SMOKE_SIDE_PANEL=1 ./target/release/chronos
# log: DisksSubscriber connected (3 filesystem device(s))
```

**Живой enumerate vs host (2026-07-21):**

| Host (`lsblk`/`df`) | UI / сервис |
|---|---|
| `/dev/nvme0n1p2` 473G, 72% on `/` | `nvme0n1p2` `337G / 473G`, **без** кнопок (internal) |
| `/dev/sdb1` Ventoy 466G, 5% | `Ventoy` `22G / 466G`, кнопки mount/размонт/извлечь |
| `/dev/sdb2` VTOYEFI 32M | `VTOYEFI` `20M / 32M`, кнопки |
| `/boot` HintIgnore | скрыт (верно) |
| zram/swap, bare sda | нет Filesystem → скрыты |

**Mount/Unmount (busctl = тот же D-Bus, что `Filesystem.Mount`/`Unmount`):**

```
# VTOYEFI BEFORE: mounted /run/media/neo/VTOYEFI
busctl … Filesystem Unmount → findmnt empty, lsblk MOUNTPOINT empty
busctl … Filesystem Mount   → s "/run/media/neo/VTOYEFI", findmnt back
```

**Eject path (live props, без вызова PowerOff):**

```
Drive ST500…: Ejectable=false, CanPowerOff=true, Removable=true
→ код: PowerOff (не Eject)
```

**Grim:** `orchestration/reports/zed-udisks-smoke/dp1-full.png` (+ `panel-crop.png`, `disks-section.png`). На DP-1: три живые карточки (nvme0n1p2 / Ventoy / VTOYEFI), internal без action-row.

Гигиена своей зоны: `todo!`/`unimplemented!` нет; `let _ =` fallible нет.

## Новые риски / известные баги

- **Severity medium:** btrfs multi-mount (`/`, `/home`, …) схлопывается в одну карточку с usage от `/` — правильно для MVP, но не «каждая точка монтирования».
- **Severity low:** size_label used (sysinfo) vs `df` может отличаться на 1–2G (337 vs 335) — округление/резервы FS.
- **Severity medium / ops:** PowerOff на USB HDD с несколькими партициями (Ventoy+VTOYEFI) снесёт **весь** drive — UI eject на любой removable-карточке того drive зовёт один drive_path. Ожидаемо для «безопасно извлечь», но клик с VTOYEFI = poweroff всего sdb.
- **Severity low / tree:** параллельный WIP чужого трека (mpris art/progress: `mpris/*`, `mpris_card.rs`, bar mpris) в working tree — **не мой**, в отчёт/коммит не тащить.

## Shared-строки для Архитектора (финальная сводка коммита)

Только ZED-трек (поимённо):

```
crates/services/src/udisks/mod.rs          (new)
crates/services/src/udisks/types.rs         (new)
crates/services/src/lib.rs                  # +mod, +reexport, +Services.udisks, +init_all
crates/app/src/state.rs                     # +AppState::disks
crates/app/src/side_panel_right/disks.rs    # live render
crates/app/src/side_panel_right/view.rs     # disks field + watch + render arg
```

Сообщение: `services/app : udisks2 DisksSubscriber + live side-panel disks section`

**Не включать** в этот коммит: mpris/*, mpris_card.rs, bar/widgets/mpris.rs, docs moves, agents, skills.

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлял. При приёмке: одна строка в ARCHITECTURE §services про udisks2 (poll, Mount/Unmount/PowerOff). DECISIONS — не требуется (новый сервис по существующему zbus-шаблону).

## Коммит

Не делал (бриф: «Не коммить»).
