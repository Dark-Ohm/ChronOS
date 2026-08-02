# T193 — Hyprland binds tab (read-only v1)

**Статус:** active. **Роль:** FRONTEND. **Модель: Grok 4.5** (модули простые)
или GLM 5.2.
**Канон:** `docs/PRODUCT.md`; live config: `~/.config/hypr/modules/`.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно T192:** не трогай `tabs.rs` / `for_mode`. Только content
`HyprlandBinds` + `tab/mod.rs` arm. Label/rail presence — T192; если T192
ещё не влит, вкладка уже есть в ALL — достаточно заполнить UI.

**Зона:**
- `crates/app/src/side_panel_right/tab/hypr_binds.rs` — **create**
- `tab/mod.rs` — `TabContent::HyprlandBinds` → entity (не Placeholder)
- `view.rs` — non-exhaustive match arms if needed (как Library)
- pure parse helpers + unit tests

**НЕ:** write to hypr files; `tabs.rs` for_mode; kitchen redesign; scenes.

**Отчёт:** `docs/orchestration/tasks/report/T193-hyprland-binds-readonly-report.md`.

---

## Цель

Read-only список биндов из **модульного** конфига:

```
~/.config/hypr/modules/20-binds-kitchen.lua
~/.config/hypr/modules/25-binds-chronos.lua
```

Fallback: если `modules/` нет — `~/.config/hypr/hyprland.lua` monolit (warn
в UI) или честный empty «No modules — see docs».

### UI

- Секции: **ChronOS** (25) / **Kitchen** (20) — или один список с колонкой Source
- Колонки: Mod+Key · Action (укороченный) · file:line optional
- Клик по строке → открыть файл в Preview/Editor target (global
  `PreviewTarget` / Files pattern — **open path only**, без edit если T194
  нет; достаточно `PreviewTarget` set + switch tab if API exists, иначе
  log path + `cx` open via existing Files/Preview channel — см. T179
  `preview_target`)
- Empty / error: §13, не panic

### Парсер (pure)

Минимально regex/line scan для:

```lua
hl.bind("SUPER + Q", ...)
hl.bind(mainMod .. " + E", ...)
```

Не полный Lua AST. Unknown lines skip. Unit tests on fixture strings from
real `20-binds-kitchen.lua` / `25-binds-chronos.lua` samples (copy snippets
into test, not live HOME in CI).

Reload: кнопка «Reload» перечитывает файлы (no hyprctl required for RO list).

## Верификация

```
cargo test -p chronos hypr_binds::
cargo test -p chronos -- side_panel_right
cargo build --release -p chronos
```

Живой: Gamer/Dev → Hyprland binds → виден SUPER+L / SUPER+Q; grim желателен.

Коммит: `hypr : binds tab read-only (T193)`.

## v2 (НЕ в этой задаче)

Write-back + human explanations — отдельный T later.
