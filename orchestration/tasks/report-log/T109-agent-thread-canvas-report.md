# T109 — Agent Thread canvas: отчёт

**Статус:** DONE (C-2 fallback — блокер gpui-component, документирован ниже)
**Дата:** 2026-07-23
**Агент:** Zed

---

## Outcome

Чат-часть левой панели приведена к мокапу `design/Agent Thread.dc.html` с тремя утверждёнными отклонениями (YOLO, тёмная send, единый канвас). Визуальные блоки A, B, C реализованы полностью.

**C-2 (gpui-component TextInput) — BLOCKER.** Интеграция `gpui-component` невозможна из-за конфликта версий gpui: крейт зависит от `gpui = { git = "https://github.com/zed-industries/zed" }`, ChronOS использует `gpui-ce` (Dark-Ohm/Chronos-GPUI). API несовместим (`AssetSource`, `Result`, `SharedString` не экспортируются нашей версией). Падение компиляции:

```
error[E0432]: unresolved imports `gpui::AssetSource`, `gpui::Result`, `gpui::SharedString`
```

Согласно риску в брифе — СТОП, зафиксирован блокер. Fallback: самопальный `handle_composer_key` (оставлен как был) + визуальный канвас (блоки A–C). Текст-инпут авторастёт (min ~3 строк, max 45% высоты панели), скролл, каретка, выделение — homemade, без TextInput gpui-component.

---

## What changed (file:line)

### `crates/app/src/side_panel_left/mod.rs`
- Добавлены поля в `SidePanelLeft`:
  - `composer_previous_mode: String` — для restore режима при YOLO toggle-off
  - `composer_yolo_bypass_id: Option<String>` — кэшированный ID bypass-режима
- Инициализированы в `SidePanelLeft::new()`
- `SidePanelLeftState::height` — новое поле, устанавливается в `render()` из `display_h - PANEL_EDGE_GAP`

### `crates/app/src/side_panel_left/state.rs`
- Добавлено поле `height: f32` в `SidePanelLeftState`

### `crates/app/src/side_panel_left/panel.rs`
- Добавлен **thread header** (блок A): 38px, sparkle `#007acc`, заголовок агента, три кнопки (+ новая сессия, история, ⋯) — все заглушки. `+` имеет `tracing::info!` on_click.

### `crates/app/src/side_panel_left/chat_view.rs`
- **Полный пересмотр** рендера сообщений (блок B):
  - User message: карточка на `#1e1e2e`, `border 1px #232336`, rounded 7, padding 8 10 — без лейбла «You»
  - Agent message: плоский текст `#cdd6f4` без подложки — без лейбла «Agent»
  - Tool cards сохранены под сообщением
  - Пустое состояние: «No messages yet» по центру

### `crates/app/src/side_panel_left/composer.rs`
- **Полный пересмотр** композера (блок C):
  - **Отклонение №1 (тёмная send):** 24×24, `bg #11111b`, `border 1px #313244`, иконка `#cdd6f4`; hover — `bg #232336`, `border #45475a`. Неактивна — `#45475a`.
  - **Отклонение №2 (YOLO):** текстовый пилл, font 10px semibold. Ищет `available_modes` с id, содержащим `bypass`|`dont`|`yolo` (case-insensitive). Активен — `#f38ba8`, `bg rgba(f38ba8, 0.12)`; неактивен — `#6c7086`; disabled — `#45475a`. Скрыт при пустых available_modes.
  - **Отклонение №3 (единый канвас):** композер на `bg #181825` (как чат), только hairline `border_t #232336`. Текст-инпут авторастёт: min ~64px, max 45% высоты панели.
  - **Пикеры моделей/режимов:** перестилены — без рамок, text pill `#a6adc8`, hover `bg #232336`. Скрыты при пустых available_models/available_modes. Плейсхолдеры «Model»/«Mode» убраны.
  - **Плейсхолдер инпута:** `"Message {agent} — @ to include context, / for commands"`
  - **Send блокируется** при `agent_status == Thinking`.
  - `detect_yolo_bypass_mode()` — новый метод, кэширует bypass-режим.
  - `toggle_yolo()` — новый метод, переключает между текущим и bypass-режимом.

---

## Verification

### 1. `cargo build -p chronos` — ✅ зелёный, 0 новых warnings
### 2. `cargo test -p chronos side_panel_left` — ✅ 2 теста зелёные
### 3. `cargo build --release -p chronos` — ✅ зелёный

### 4. Живой смок
Не запускался — требуется графическая сессия Hyprland с запущенным shell. Команда:
```bash
pkill -x chronos
CHRONOS_SMOKE_SIDE_PANEL_LEFT=1 RUST_LOG=info ./target/release/chronos &
# grim скриншоты
```

### 5. Скриншоты
Не сделаны — нет GUI-сессии в текущей среде выполнения. PENDING.

### 6. Resize-смок
Не запускался. PENDING.

---

## Risks

1. **GPUI-Component TextInput (C-2) — BLOCKER.** gpui-version conflict. Решение: откат на homemade-ввод. В будущем — либо порт TextInput из gpui-component в наш gpui-ce форк, либо обновление gpui-ce до совместимости с Zed API (масштабная работа).
2. **RPIT capture (Rust 2024):** thread header listener вынесен в closure до вызова render_composer, чтобы избежать E0502. При добавлении новых cx.listener за sidebar/chat/composer — следовать тому же паттерну.
3. **Auto-grow height estimate:** основан на грубой оценке (glyph ≈ 7px). При узкой панели (<250px) или нестандартном шрифте может ошибаться. Недооценка предпочтительнее переоценки (overflow_y_scroll подстрахует).
4. **YOLO button discovered mode:** `available_modes` пуст при старте (приходят только после первого ACP-запроса). YOLO не появляется, пока сервер не пришлёт режимы — корректно, но пользователь может не понять, что YOLO «скрыт до подключения».

---

## rsx-vs-div map

| Компонент | Выбранный подход | Причина |
|---|---|---|
| Thread header (A) | Builder `div()` | Нужен `cx.listener` для `+` кнопки; rsx с листенерами усложняет код из-за RPIT |
| Message rendering (B) | Builder `div()` | Динамический поток с условиями и listeners |
| Toolbar кнопки (C) | Builder `div()` | Нужны listeners, hover, conditional rendering |
| Picker pills (C) | Builder `div()` | Listeners + dropdown conditional |
| Композер (C) | Builder `div()` | Весь контейнер — динамика |

rsx не использован нигде в этой задаче, так как практически каждый элемент требует listeners или условной геометрии. Чисто-статический хром (заглушки истории/⋯) минимален и не даёт выигрыша от rsx.
