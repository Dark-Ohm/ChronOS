# T186 — Gamer rail: Library / Scenes / Captures + иконки

**Статус:** active. **Роль:** FRONTEND. **Модель: GLM 5.2.**
**Правила:** `docs/orchestration/agents/RULES.md`.
**План:** `docs/superpowers/plans/2026-08-02-gamer-hub-slice-5.md` §2.5.
**T184:** `report-log/T184-gamer-hub-recon-report.md` §6.

**Параллельно:** T185 (scene), T187 (apps). **Не пересекайся** зонами.

**Зона:**
- `crates/app/src/side_panel_right/tabs.rs`
- `crates/app/src/side_panel_right/tab/mod.rs` — **только** новые
  `TabContent` arms → `EmptyTab` / Placeholder (живые Library/Scenes — T188/T189)
- `crates/app/src/assets.rs` — include_bytes новых SVG
- `crates/app/assets/icons/rail-library.svg`
- `crates/app/assets/icons/rail-scenes.svg`
- `crates/app/assets/icons/rail-captures.svg`
- при необходимости `placeholder_description` в `tab/mod.rs`

**НЕ трогать:** `scene.rs`, `applications/**`, `view.rs` (кроме если
без этого не компилируется enum — старайся не), Files/Terminal/Build/Preview.

**Отчёт:** `docs/orchestration/tasks/report/T186-gamer-rail-tabs-report.md`.

---

## Цель

1. Три новых `PanelTab`: `Library`, `Scenes`, `Captures`.
2. `ALL` catalog + тесты (было 14 → **17**).
3. `for_mode(Gamer)`:

```
System,
Library,
Scenes,
Captures,
AcpSettings, McpSettings, LspSettings, ApiProviders, EditorSettings, HyprlandBinds
```

Итого **10** вкладок Gamer. Developer `for_mode` = прежние 14 workbench
**без** Library/Scenes/Captures.

4. `parse_id` / `id()` round-trip: `library`, `scenes`, `captures`
   (+ underscore/camel как у соседей).
5. `preferred_content_width`: Library **480**, Scenes **400**, Captures **320**.
6. Иконки SVG **без** `mix-blend-mode` / `destination-out` (T172). Обводка
   или evenodd path. **Обязательно** в `assets.rs` (урок T169 — иначе
   пустые слоты).
7. Честные empty descriptions (§13):
   - Library: list/launch games (backend filter in T187/T188)
   - Scenes: activate per-game scenes
   - Captures: **unavailable** — no capture backend (slice 6)

`TabContent::create`: все три → `Placeholder(EmptyTab)` пока T188/T189.

## Тесты (расширить, не ослабить)

- `all_has_*` → 17 tabs fixed order (Library/Scenes/Captures после work tools
  Developer catalog: логичный порядок в `ALL` — после SourceControl / перед
  settings **или** в конце work-group; **важно:** `for_mode(Developer)`
  их **не** включает).
- `gamer_rail_*` → 10 tabs, System first, settings tail unchanged order
  (`developer_settings_group_matches_gamer_settings_group_order` должен
  жить: settings = gamer[4..] после System+3 gamer tools).
- parse_id round-trip for three new.
- shared tabs relative order across modes (System + settings).

## Верификация

```
cargo test -p chronos tabs::
cargo test -p chronos -- side_panel_right
cargo build --release -p chronos
```

Живой кадр (если можешь): Gamer mode → rail 10 иконок, три новые **не
пустые** на grim. Панель два приёма. Не смог — `НЕ ПРОВЕРЕНО` с причиной.

Коммит: `rail : Library/Scenes/Captures tabs + icons (T186)`.

## Что НЕ делать

- Реальный UI Library/Scenes (T188/T189)
- scene::activate
- games filter
