# T165 — композиция шелла по режиму: вкладки рейла и состав дока

**Роль:** FRONTEND. **Ветка:** `master` (коммит ниже).  
**Зависимость:** T164 API сцены (принята).

---

## Что сделано

### Рейл — наборы вкладок

`crates/app/src/side_panel_right/tabs.rs`:

- `PanelTab::ALL` остаётся **полным каталогом** (10 вкладок) — покрытие
  иконок/подписей по-прежнему обходит все.
- `for_mode(mode)` — дефолт режима:
  - **Developer** — все 10 (System, Files, Editor, Terminal + 6 settings).
  - **Gamer** — System + settings (Files/Editor/Terminal уходят). System
    первая в обоих.
- `resolve_for_mode(mode, scene_override)` — сцена > режим; неизвестные id
  `warn` + skip; пустой остаток → фолбэк на режим.
- `id` / `parse_id` — стабильные id для `scenes.toml`.

**Почему Gamer без Files/Editor/Terminal:** §4.2 — game hub / deck, не
вторая IDE. Settings group сохраняется (§4.1: «keeps the settings group
intact»). Shared tabs (System + settings) в одном относительном порядке.

### Рейл / view

- `rail.rs` — рисует переданный набор, не `ALL`.
- `view.rs` — точечно: резолв набора, если активная вкладка исчезла →
  `System`, панель **не** закрывается. Без ветвления по режиму в рендере.

### Док

`dock/config.rs`:

- `default_pinned_for_mode` — Developer: historical default; Gamer:
  `steam`, `discord`, `firefox`, `kitty`.
- `resolve_pinned(cx)` / `resolve_pinned_with` — сцена > режим > stored.
- Developer предпочитает пользовательский `dock.toml`; Gamer берёт mode
  default (иначе док визуально не менялся бы).

### Потребитель бара (граница зоны)

`bar/widgets/dock.rs` — одна строка: `config::resolve_pinned(cx)` вместо
`cached().pinned`. Без этого живой док не читает композицию. Зона задания
писала `bar/**` в «не трогать» — это **необходимая** однострочная
проводка; в отчёте явно.

### scene API

Сняты `#[allow(dead_code)]` с `current` / `rail_tabs_override` /
`dock_override` (подключены). `active_tab_override` пока allow — нет
потребителя в T165.

### lib.rs

`workspace_mode` и `scene` экспортированы в lib, чтобы
`side_panel_right` (lib + bin) видел их.

---

## Чем доказано

| Команда | Результат |
|---|---|
| `cargo test -p chronos --bins` | **221 passed; 0 failed** |
| `cargo clippy -p chronos --all-targets` | Finished (pre-existing warnings) |
| `cargo build --release -p chronos` | Finished optimized |
| `rg -n 'workspace_mode::(set\|toggle)' --type rust crates/` | только IPC + виджет бара + коммент в scene — **новых нет** |

### Живой прогон

```
chronos-stop
RUST_LOG=info,chronos=debug …/target/release/chronos
# IPC: toggle-side-panel-right, set-workspace-mode:gamer
```

| Проверка | Улика |
|---|---|
| Рейл Developer (10 вкладок) | `/tmp/t165/02-developer-panel.png` — rail справа, System активен, panel open |
| Док Developer | `/tmp/t165/01-developer-dock.png` — work pins |
| Режим Gamer | bar `/tmp/t165/03-gamer-bar.png` — подпись **Gamer** |
| Рейл Gamer (без work tools) | `/tmp/t165/03-gamer-panel.png` — меньше иконок, System+settings |
| Док Gamer | `/tmp/t165/03-gamer-dock.png` — steam/discord вместо IDE pins |
| Панель не закрылась | `hyprctl layers`: `side_panel_right` 2000,30 **560×1410** до и после switch |
| Лог | `switched mode="Gamer"`, `scene: no last scene, using mode defaults`; без panic |

---

## Обоснование наборов (одна строка на вкладку)

| Вкладка | Developer | Gamer | Почему |
|---|---|---|---|
| System | ✓ 1-я | ✓ 1-я | `default()`, посадочная, telemetry/hub |
| Files | ✓ | — | workbench tree; не hub |
| Editor | ✓ | — | config/scripts; не hub |
| Terminal | ✓ | — | PTY; не hub |
| AcpSettings | ✓ | ✓ | settings group shared |
| McpSettings | ✓ | ✓ | settings group shared |
| LspSettings | ✓ | ✓ | settings group shared |
| ApiProviders | ✓ | ✓ | settings group shared |
| EditorSettings | ✓ | ✓ | settings group shared |
| HyprlandBinds | ✓ | ✓ | settings group shared |

---

## Что НЕ сделано

- `active_tab_override` сцены не применяется при restore (API есть, consumer
  не в scope; mode-switch fallback на System есть).
- Preview/Inspector/Build/SourceControl (14 вкладок спеки) — вкладок в
  enum ещё нет; добавятся отдельным слайсом.
- Gamer-specific deck tabs (FPS, mixer, …) — §4.2, слайс 5/6.
- Не коммитил `docs/orchestration/**` (gitignored / не зона кода).

---

## Зона файлов

| Файл | Статус |
|---|---|
| `side_panel_right/tabs.rs` | переписан API |
| `side_panel_right/rail.rs` | tabs param |
| `side_panel_right/view.rs` | resolve + fallback |
| `dock/config.rs` | resolve_pinned |
| `scene.rs` | dead_code off |
| `lib.rs` | export scene/workspace_mode |
| `bar/widgets/dock.rs` | **1 строка вне зоны** — consumer wire |

Не тронуты: `scene` persist logic, `monitor`, `side_panel_left`, `dock/context_menu`, workspace_mode set/toggle.
