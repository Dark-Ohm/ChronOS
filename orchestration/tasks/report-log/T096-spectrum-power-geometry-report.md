<!-- T096 — migrated 2026-07-22 from orchestration/report-log/hermes-report-21.md — see orchestration/tasks/MIGRATION.md -->

# Session: Hermes №21 — Tasks 10+11 spectrum + power-row — 2026-07-21

**Агент:** Hermes (Lead Architect / Grok).  
**Стек C:** div, без `gpui-component`. Cargo.toml **не** трогался.  
**Коммит:** **не** делался (бриф: Архитектор).

---

## Исход (первая строка)

**T10+T11 в дереве.** Units **11/11** (5 старых + 2 ring + 4 arm). Release green.  
Live: метры **двигаются** (CPU 27%→30%, GPU 22%→13%, dn/up с curl-load),  
power-row видна (User disabled + Log out / Restart / Power). Arm→Confirm  
unit-покрыт; живой клик ydotool dual-head не попал — **не** жмём второй  
Restart (ребут). Без animation на 14×5 барах (статика, 144fps).

---

## Сделано

| Файл | |
|---|---|
| `spectrum_row.rs` | `SpectrumHistory` ring 14, `push_sample`, `render_spectrum_row`; font_mono; палитра через caller |
| `power_row.rs` | `ArmState`/`PowerAction`, pure `on_click`/`is_confirming_click`/`on_timeout`, `render_power_row` + `cx.listener` |
| `view.rs` | watch system_resources → push histories; `sample_network` time-gated (push only on real sample); power `on_power_click` + 3s disarm match/warn; spectrum+power children; **нет** второго `on_hover` на root |
| `mod.rs` | `mod spectrum_row; mod power_row;` |

### Кровные факты

- **Render-safe net:** `update_speed(..., SAMPLE_INTERVAL)`; history push **только**  
  когда `sample.time` сменился — не 60×/s одинаковых значений.
- **Палитра:** CPU `0x5fd3e8`, RAM `0x4fa3c9`, GPU `0x33638a`, dn `0x7cc4e8`, up `0x3d6d94`.
- **GPU:** `when_some(gpu_percent)` — скрыт при `None`.
- **Power:** Switch user disabled; arm 3s; confirm → `AppState::power`;  
  timeout `match view.update` / `warn!`, не `let _ =`.
- **on_hover:** root debounce alone (T9); meters/power без hover.

### Animation

Бары **статические** (нет `transition_when` на 14×5). Reveal fade панели  
(T8/9) без изменений. rsx не использовал.

---

## Проверено фактом

```
cargo test -p chronos --bin chronos side_panel
# 11 passed

cargo build --release -p chronos
# Finished ~2m 30s

readlink /proc/$(pgrep -x chronos)/exe
# …/ChronOS/target/release/chronos  (master, not pilot)

hyprctl layers
# side_panel_hover_strip 2556×4 on DP-1
# side_panel_right 2260 15 300 1440

CHRONOS_SMOKE_SIDE_PANEL=1 → opened (pinned)
```

**Grim** `orchestration/reports/hermes-21-smoke/`:

| файл | факт |
|---|---|
| `panel-load-1.png` | CPU 27% GPU 22% dn 2.5M up 160K — bars uneven |
| `panel-load-2.png` | CPU 30% GPU 13% dn 2.3M up 132K — **разные** высоты vs load-1 |
| `panel-bottom.png` / `power-row.png` | User / Log out / Restart / Power |

stress-ng **нет** в PATH — нагрузка: background work + curl limit-rate  
(сеть видна). CPU/GPU уже ненулевые от сессии.

---

## Не дожато живьём

| | |
|---|---|
| Arm → "Confirm?" grim | ydotool не попал; machine unit 4/4 |
| Timeout disarm 4s live | unit + log path; нужен ручной клик Restart |
| Real reboot confirm | **не** делали (план: спросить пользователя) |
| stress-ng | package отсутствует |

---

## Расхождения

- Net push только на смене sample.time (план пушил каждый render — риск flat flood).
- `format_bytes_per_sec` K/M, не всегда `M`.
- Idle power-row grim есть; armed — нет (input).

---

## Риски

| Sev | |
|---|---|
| Low | Smoke env `CHRONOS_SMOKE_SIDE_PANEL` всё ещё в init — вырезать при коммите опционально |
| Info | History fills to 14 over ~14s of sys ticks; early grim shows fewer bars |

---

## Артефакты

- `crates/app/src/side_panel_right/{spectrum_row,power_row,view,mod}.rs`
- Smoke: `orchestration/reports/hermes-21-smoke/`
- Logs: `/tmp/t21-test.log`, `/tmp/t21-rel.log`, `/tmp/t21-smoke.log`
