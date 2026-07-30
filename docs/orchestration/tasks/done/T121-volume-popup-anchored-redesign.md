# T121 — ПРИНЯТ WITH CAVEATS (2026-07-25)

**Статус: ACCEPTED WITH CAVEATS.** Anchored Sound popup + sliders.
Commit: `54a54c0`. Live smoke PENDING.
Review: `report-log/T121-volume-popup-anchored-redesign-review.md`.
Отчёт: `report-log/T121-volume-popup-anchored-redesign-report.md`.

---

<!-- T121 — Volume popup: anchored + mockup Sound UI + sliders.
     Агент не в имени брифа. После T120 (разные зоны файлов).
     Skills: gpui-fork-start-here, gpui-rsx-markup, gpui-rsx, anchored-popups. -->

# T121 — Volume popup (anchored redesign)

**Статус: OPEN, не назначен.**  
**Очередь:** следующий после T120 (notifications). Можно начинать когда T120
в report / accept — зоны **не пересекаются**, но не параллель один миньон
на оба без явного split.

| | |
|---|---|
| **План (Task 1–5)** | `docs/superpowers/plans/2026-07-24-volume-popup-anchored-redesign.md` |
| **Мокап** | `docs/design/Volume Popup.dc.html` (Dark + Light C) |
| **Эталон якоря** | `updates_popup/` + `bar/widgets/updates.rs` |
| **Отчёт** | `docs/orchestration/tasks/report/T121-volume-popup-anchored-redesign-report.md` |

## Как выполнять

- **superpowers:subagent-driven-development** — implementer на Task N +
  task-review; **или**
- **superpowers:executing-plans** — один проход по чеклистам plan-файла.

ChronOS: «subagent» = локальный миньон по **T121**, не spawn Архитектора.
Бриф **без** личного имени агента. Указатель `docs/orchestration/agents/<TOOL>.md`
→ этот файл.

Skills (прочитать до UI Task 3–4):
- `gpui-fork-start-here` — это форк, не crates.io; AnchoredPopup / gpui-rsx
- `gpui-rsx-markup` — vendored macro, `rsx_expand!` при странных ошибках
- `gpui-rsx` — mockup→rsx, rsx vs builder, E0425/E0631, id + scroll
- `anchored-popups` — grab, mouse_down, parent bounds

---

## Цель

Попап звука с bar volume widget:

1. **AnchoredPopup** к иконке volume (+ LayerShell fallback).
2. Визуал **Sound** 360px по мокапу (не MVP 300px fill-bar + ±5%).
3. **Drag/click sliders** sink + source → `AudioCommand`.
4. Device dropdowns + footer **Mute output** / **Mute mic**.
5. Live grim; unit ≠ done.

## Текущее состояние (факты)

| | Сейчас | Мокап / цель |
|---|---|---|
| Open | `on_click` → `toggle(window,cx)` | mouse_down + anchor_rect + parent |
| Window | LayerShell TOP\|RIGHT, **300px** | AnchoredPopup, **360px** |
| Header | «Volume» + ✕ | **«Sound»** + ✕ |
| Sections | «Speakers» / «Microphone» | **Volume** / **Microphone** + device subtitle |
| Level UI | fill-bar visual + **−5% / mute / +5%** | **slider** 0–100 + mono `%`/`Muted` |
| Footer | нет | dual outlined mute buttons |
| Devices | expand under title, ●/○ | floating menu ~220px + checkmark |
| Backend | `AudioCommand` уже полный для мокапа | **не** плодить второй audio stack |

Код: `crates/app/src/volume_popup/{mod,view}.rs`,  
`crates/app/src/bar/widgets/volume.rs`,  
`crates/services/src/audio/` (read-only unless gap).

## Задачи (детали — plan)

1. **Bell-аналог для volume:** canvas + `.relative()` + mouse_down; scroll volume **сохранить**.
2. **mod.rs:** anchored + fallback + width 360 + height estimate.
3. **Chrome (rsx):** header/footer/labels; Light C watermark если `is_light`.
4. **Sliders + menus (builder):** drag, unmute-on-change, SetDefault*.
5. **Live + report** с **rsx vs div map**.

## rsx vs builder (жёстко)

| rsx! | builder div() |
|---|---|
| card shell, header Sound+✕ | slider track/thumb + drag |
| light watermark/glow | device row list |
| footer mute labels (можно rsx) | `cx.listener` expanded menus |

Обязательно: `use gpui::div`; `hover={|s|…}` без типа `Div`; `.id` на
interactive. Не патчить vendored `gpui-rsx`.

## Зона файлов

**Писать:** `volume_popup/*`, `bar/widgets/volume.rs`  
**Не трогать:** notifications/history, updates_popup, system_popup (кроме
чтения), tray, side_panel_*, T115, audio backend без нужды.

## Верификация

```bash
cargo build --release -p chronos
# live: click volume → slider → wpctl; mute footer; mic; device pick; grim
```

Optional: `cargo run -p chronos-services --example audio-dispatch-smoke`

## Accept / Reject

**Accept:** anchored near icon; slider moves real volume; mute footer works;
mockup skeleton (Sound, 360, dual sections, footer); rsx/div map in report;
live evidence.

**Reject:** only −5%/+5% left as primary UX; fixed corner without anchor
attempt; silent fallible; «compiles» without grim; personal agent name as
deliverable id.
