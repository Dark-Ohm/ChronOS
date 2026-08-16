# T125 — ПРИНЯТ WITH CAVEATS (2026-07-25)

**Статус: ACCEPTED WITH CAVEATS.** Commit `fc71215`. Brightness track hit + optimistic errata.
Review: report-log/T125-*-review.md.

---

<!-- T125 — System popup: anchored + mockup System chrome (brightness / power / gaming).
     Агент не в имени брифа. Паттерн: T121 volume + chronos-gpui-popup. -->

# T125 — System popup (anchored redesign)

**Статус: OPEN, не назначен.**  
**Мокап:** `docs/design/System Popup.dc.html` (Dark + Light C)  
**Код:** `crates/app/src/system_popup/{mod,view,gaming_mode}.rs`  
**Триггер:** `crates/app/src/bar/widgets/system.rs`  
**Эталон якоря:** `updates_popup` / `volume_popup` post-T117/T121  
**Skill:** `chronos-gpui-popup`, `anchored-popups`, `gpui-rsx`

## Цель

Попап **System** (bar hexagon-sigil) — та же popup-дисциплина, что volume:

1. **AnchoredPopup** к bounds иконки system (+ LayerShell fallback).  
2. Визуал 1:1 mockup **360px**, header «System» + ✕, Light C watermark/glow.  
3. Три блока: **Brightness** · **Power profile** · **Gaming mode** (уже есть backend).  
4. Live smoke, unit green ≠ done.

**Не** переписывать gaming/brightness **сервисы** с нуля — UI + window lifecycle.

## Текущее vs мокап

| | Сейчас | Мокап / цель |
|---|---|---|
| Open | `on_click` → `toggle(window,cx)` | `mouse_down` + `anchor_rect` + `parent` |
| Window | LayerShell TOP\|RIGHT, **300px** | AnchoredPopup, **360px** |
| Header | MVP | «System» + ✕ 22×22 (rsx ok) |
| Brightness | ☀ label + fill-bar + −/+ Step | icon + mono `%` + − / track+fill / + (mockup) |
| Power | 3-seg Quiet/Balanced/Performance | segmented control, active `#007acc` |
| Gaming | toggle + effect string | toggle 34×19 + muted description line |
| Light C | partial tokens | watermark + glow + elevated shadow if `theme.is_light` |
| Height | fixed estimate | fit content (no scroll needed — short panel); **no footer clip** (BASE_HEIGHT lesson T121) |

Backend already wired:

- `BrightnessCommand::Step` / `Refresh` / set path  
- `UPower` / power profile set  
- `gaming_mode::toggle`  

**Не** раздувать scope native DDC rewrite.

## Задачи

### Task 1 — Bar system widget: bounds + mouse_down

Зеркало `bar/widgets/volume.rs` / `updates.rs`:

- `Rc<Cell<Bounds<Pixels>>>` + canvas + **`.relative()`**  
- `on_mouse_down(Left)` →  
  `system_popup::toggle(anchor_rect, parent, window, cx)`  
- Сохранить текущую иконку (hexagon sigil / accent).

### Task 2 — Window: AnchoredPopup + 360

`system_popup/mod.rs`:

- `POPUP_WIDTH = 360.`  
- `open(cx, anchor_rect, parent)` / `toggle(anchor_rect, parent, window, cx)`  
- `BottomRight` + `BottomLeft`, grab, `SLIDE_X|FLIP_X`, offset y=4  
- Fallback `PopupNotSupportedError` → LayerShell TOP|RIGHT  
- `close` / `close_this` reentrancy (HANDOFF ghost-window)  
- On open: keep `BrightnessCommand::Refresh`  
- Height: measure content budget (header + brightness + divider + profile + divider + gaming) with **margin**; if too short, footer/toggle clips — raise constants (T121 BASE_HEIGHT lesson). Prefer fixed tall-enough window over adaptive drama.

### Task 3 — View chrome 1:1 mockup

`view.rs` (rsx header optional, builder for interactive):

| Piece | Approach |
|---|---|
| Header System + ✕ | `rsx!` or thin builder |
| Light watermark / glow | `theme.is_light` (volume/updates recipe) |
| Brightness row | mono `%`, −5 / track fill / +5 (or keep STEP); track 4px |
| Optional | click/drag on brightness track like volume (nice-to-have; mockup is ± + fill). If drag: **own** drag marker type, not shared with volume |
| Power profile | 3 equal segments, active accent fill + dark text, inactive muted |
| Gaming | toggle + description string from mockup (or current effect list if more accurate) |

Colors: Theme tokens preferred; mockup hex OK for accent `#007acc` active segment.

### Task 4 — Live smoke

```bash
chronos-rebuild && chronos-stop && chronos-start
# click system sigil → popup under icon
# brightness −/+ or track → displays change (ddcutil path)
# power profile segments → profile switches
# gaming toggle → mode on/off (bar/dock hide per existing gaming_mode)
# grim dark (+ light if switchable)
```

Отчёт:  
`docs/orchestration/tasks/report/T125-system-popup-anchored-redesign-report.md`  
— rsx vs div map, height constants, live evidence.

## Зона файлов

**Писать:**
- `crates/app/src/system_popup/{mod,view}.rs`
- `crates/app/src/bar/widgets/system.rs`
- optional touch `gaming_mode.rs` only if toggle/state needs notify for repaint

**Не трогать:**
- volume/updates/notifications/history (except reading patterns)
- brightness service implementation (unless bug blocks UI)
- tray_menu, side panels

## Что НЕ делать

- Не 300px fixed corner «чуть подкрасить» без anchor  
- Не `on_click` для grab-popup open  
- Не shared `VolumeSliderDrag` marker for brightness drag  
- Не `let _ = fallible` без лога  
- Не фабриковать live smoke  

## Accept / Reject

**Accept:** anchored near system icon; 360 System chrome; brightness + profile + gaming work live; release build; report with rsx/div map.

**Reject:** only CSS-ish restyle on LayerShell 300; dual-instance start; unit-only; broken close_this.

## Out of scope

- Full gaming mode feature rewrite  
- New power profiles beyond Quiet/Balanced/Performance mapping  
- System settings app / deep links  
