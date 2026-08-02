# Live customization — Implementation Plan

**Дата:** 2026-08-02. **Канон:** `docs/PRODUCT.md` § Live desktop customization.  
**Цель:** максимальная кастомизация chrome **и** максимальная интуитивность  
(агент на естественном языке + визуал/пресеты + raw config для про).

---

## 1. Что «максимально» значит (границы)

### В scope (shell chrome — declarative, hot-apply)

| Область | Примеры |
|---|---|
| **Bar geometry** | top/bottom (later left/right), height, width mode (full / fraction / hug), horizontal align, margins, exclusive zone on/off |
| **Bar chrome** | radius, shadow/elevation, blur tier, opacity, bg from theme tokens or override |
| **Bar widgets** | left/center/right lists, order, enable/disable (уже `bar.toml`) |
| **Theme** | scheme, later token overrides in theme.toml |
| **Dock** | pins, size, edge (уже частично `dock.toml`) |
| **Panels** | default widths, open state (осторожно — session vs config) |
| **Density mode** | Developer/Gamer composition (уже workspace mode) |

### Вне scope (не раздувать бинарь)

- Полный GTK-like control center всех DE
- Произвольный CSS/shader marketplace
- Live-edit **кода** виджетов пользователем (только dev hot-reload)
- Hyprland decoration всего стола (частично dots; шелл не заменяет compositor theme целиком)

**Максимум** = всё, что пользователь считает «мой бар / мой шелл», а не «перепиши Plasma».

---

## 2. Что «интуитивно» значит (три лица, один config)

```
                    ┌─────────────────────┐
   NL to agent  ───►│  config files       │◄─── Edit mode / presets UI
                    │  (single source)    │
   raw toml/editor ─►│  hot-reload apply   │
                    └─────────┬───────────┘
                              ▼
                         live chrome
```

1. **Агент (primary для сложных запросов)**  
   «Бар снизу, floating, 70% ширины, тень, убери cava, добавь clock» → tools →
   write config → apply. Follow показывает diff.

2. **Визуал / пресеты (primary для «потыкать»)**  
   - Edit mode (уже): порядок виджетов  
   - Пресеты: `top-full`, `bottom-pill`, `minimal`, `gaming-quiet`  
   - System settings: слайдеры/тогглы, связанные со схемой (не 200 полей сразу)

3. **Raw config (power users + agent transparency)**  
   Один понятный `bar.toml` (+ theme). Модули hypr — отдельно.

Интуиция = **мгновенный фидбек** + **нельзя оставить сломанный бар** (validate +
rollback) + **слова агента = те же ключи, что в файле** (нет двух словарей).

---

## 3. Состояние as-of 2026-08-02 (факт)

| Есть | Нет |
|---|---|
| `bar.toml` left/center/right widgets + inotify hot-reload | bar **position** (hardcode TOP) |
| `theme.toml` scheme + hot-reload | bar **width/float/margin/radius/shadow** as config |
| Edit mode widget layout | agent tools get/set bar |
| `BAR_HEIGHT` constant in code | schema version + presets file |
| LayerShell anchors in `bar/mod.rs` | live resize of layer surface from config change |

Вывод: **виджеты** уже на правильном пути; **геометрия/внешность** — следующий разрыв.

---

## 4. Целевой `bar.toml` (v2 schema sketch)

```toml
version = 2

[appearance]
edge = "top"          # top | bottom  (left|right = later vertical bar)
height = 30           # px
width = "full"        # full | fraction:0.7 | hug
align = "center"      # start | center | end (when not full)
margin = { x = 12, y = 8 }   # used when floating / inset
floating = false
exclusive = true      # reserve exclusive zone when not floating
radius = 0            # px; >0 implies clip
# shadow / blur: token names or simple enable
elevation = "none"    # none | soft | strong  → maps to theme ElevationTokens

[widgets]
left = ["dock", "separator", "workspaces"]
center = ["mpris", "cava"]
right = ["volume", "clock"]
known = [ ... ]
```

v1 files without `version` load as today (widgets only; appearance = code defaults).

---

## 5. Волны задач

| ID | Роль | Что | Зависит |
|---|---|---|---|
| **T198** | RECON | Карта: все hardcoded chrome props (bar/dock/panels) file:line; gap to target schema | — |
| **T199** | BACKEND | `bar.toml` v2 appearance section + sanitize/defaults + tests; **no UI** | T198 |
| **T200** | FRONTEND | Apply appearance: window_options / resize / re-anchor **without kill**; hot-reload path | T199 |
| **T201** | BACKEND | Agent tools (or ACP-facing service API): get/set/list widgets + appearance patch | T199 |
| **T202** | FRONTEND | Presets + System settings «Bar» page (sliders/toggles bound to same schema) | T200 |
| **T203** | FRONTEND | Agent path dogfood: skills/prompt so NL maps to schema keys; Follow shows bar.toml | T201, T195 optional |

T194 (Editor) может идти **параллельно** T198–T199 (разные зоны).

**Интуиция v1:** presets + agent tools + instant apply.  
**Интуиция v2:** visual drag chrome chrome (size handles) — later, after schema stable.

---

## 6. Риски

1. **Layer-shell re-anchor mid-session** (top→bottom) — compositor-specific; need live test on Hyprland 0.56, possible destroy/recreate surface without killing whole app.
2. **Floating bar exclusive zone** — must clear exclusive when floating else gaps wrong.
3. **Agent writes invalid toml** — sanitize + keep last-good in memory + disk bak.
4. **Maximal UI clutter** — System settings shows **presets + 6–8 controls**, advanced in «raw editor» link (T194).

---

## 7. Definition of done (epic)

Пользователь в agent panel:

> бар снизу, 80% ширины по центру, скругление 12, тень, без cava, clock справа

…и **без logout / без pkill** видит новый бар; `bar.toml` читаем и совпадает;  
Undo / смена пресета возвращает предыдущее.

---

## 8. Утверждение

**УТВЕРЖДЁН 2026-08-02** пользователем.
Terminal: Zed-style bottom drawer in Editor (T194), not rail tab.
Первая задача: **T198 RECON**.
