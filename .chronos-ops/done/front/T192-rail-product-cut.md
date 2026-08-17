# T192 — rail: product cut (default tabs only)

**Статус: ЗАКРЫТА 2026-08-02 — ПРИНЯТА** (архитектор). **Модель: GLM 5.2.**
Отчёт: `report-log/T192-rail-product-cut-report.md`. Коммит: `6660d2f`. tabs:: 29/29. Live grim — NOT VERIFIED.
**Канон:** `docs/PRODUCT.md` §2 / §4.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно:** T193 (Hyprland binds RO) — зона **не** `tabs.rs`/`for_mode`.
Ты владеешь `tabs.rs` + иконки/labels при rename. T193 не трогает `for_mode`.

**Зона:**
- `crates/app/src/side_panel_right/tabs.rs` — `for_mode`, labels, parse_id aliases
- тесты в `tabs.rs`
- при необходимости `tab/mod.rs` **только** placeholder_description / labels
  (не UI content других вкладок)
- `assets.rs` только если переименовываешь иконку EditorSettings→System settings

**НЕ:** preview.rs, library, scene, left panel, hyprland_binds content (T193).

**Отчёт:** `docs/orchestration/tasks/report/T192-rail-product-cut-report.md`.

---

## Цель

Default rail отражает **продукт**, не спеку Shell-IDE на 14–17 заглушек.

### Developer `for_mode` (целевой набор)

Порядок (System first, settings tail):

1. **System**
2. **Files**
3. **Editor** — временно это всё ещё `PanelTab::Preview` **или** rename enum
   (см. ниже): label «Editor», icon preview/editor. Полный edit path — **T194**.
4. **HyprlandBinds** — label «Hyprland binds» (контент RO — T193)
5. **AcpSettings** — label «ACP agents» (CRUD — T196; placeholder ok)
6. **System settings** — бывший `EditorSettings` → label «System settings»

**Убрать из default Developer rail (не удалять из enum ALL обязательно):**

- Terminal, Inspector, Build, SourceControl (empty/IDE)
- McpSettings, LspSettings, ApiProviders
- пустой **Editor** work-tool variant (`PanelTab::Editor` empty IDE) — не в for_mode
- Scenes (product kill)

`PanelTab` variants можно **оставить** в `ALL` для parse/scene override, но
`for_mode(Developer)` их не показывает. Тесты `for_mode` переписать под новый
набор; `ALL.len()` можно не ломать (catalog ≠ rail).

### Gamer `for_mode`

1. System  
2. **Library**  
3. Captures optional — **можно оставить** empty (list folder later) или hide  
4. settings tail: AcpSettings + System settings + HyprlandBinds  

**Scenes — не в rail.**

Shared relative order settings: System/settings consistency tests update.

### Labels (минимум)

| id | label |
|---|---|
| Preview (until T194 rename) | **Editor** |
| EditorSettings | **System settings** |
| AcpSettings | **ACP agents** |
| HyprlandBinds | Hyprland binds |

### Placeholder descriptions

Обновить под PRODUCT (нет «coming soon»). Captures: «Screenshot folder» ok.

## Верификация

```
cargo test -p chronos tabs::
cargo build --release -p chronos
```

Живой: Gamer/Developer rail — **нет** LSP/MCP/Build/Scenes; есть Files +
Editor(label) + Library(Gamer). Кадр желателен.

Коммит: `rail : product default tabs (T192)`.

## Что НЕ делать

- Edit в Preview (T194)
- Парсер hypr binds UI (T193)
- Follow agent (T195)
- ACP CRUD (T196)
