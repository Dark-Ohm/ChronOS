# T199 — bar.toml v2: `[appearance]` schema + sanitize (no UI)

**Статус:** active. **Роль:** BACKEND (осознанно `crates/app/bar` — как T160).
**Модель:** GLM 5.2 / Sonnet / DeepSeek V4 Pro.
**План:** `docs/superpowers/plans/2026-08-02-live-customization.md` §4–5.
**Канон:** `docs/PRODUCT.md` § Live desktop customization.
**RECON:** `docs/orchestration/tasks/report-log/T198-chrome-customization-recon-report.md`
(**ACCEPTED WITH NOTE** — читай acceptance block внизу).
**Правила:** `docs/orchestration/agents/RULES.md`.

**Зависимость:** T198 done. **T200** ждёт **этот** коммит — не делай apply/UI.

**Параллельно:** T194b done; T194c (preview/edit) может идти отдельно —
**не** трогай `side_panel_right/tab/`.

**Зона (только эти пути):**
- `crates/app/src/bar/layout_config.rs` — расширить load/save/cache/sanitize
  **или** новый соседний модуль `crates/app/src/bar/appearance.rs` +
  `mod appearance` в `bar/mod.rs` / re-export
- unit tests в том же файле(ах)
- **опционально** пример/коммент в `docs/` **не** обязателен

**НЕ:**
- `bar/mod.rs` `window_options` / render / `open_window` (это T200)
- fork `../Source`
- panels / dock geometry / theme token overrides
- agent tools (T201)
- presets UI (T202)
- запись в `~/.config/chronos/bar.toml` пользователя при тестах — только
  temp dirs / in-memory fixtures

**Отчёт:** `docs/orchestration/tasks/report/T199-bar-appearance-schema-report.md`.

---

## Цель

Декларативная схема appearance **без** применения к окну. После T199:

1. v1 `bar.toml` (только left/center/right/known) **грузится как сегодня** —
   appearance = **code defaults**, `version` отсутствует → treat as v1.
2. v2 с `version = 2` + `[appearance]` парсится, `sanitized()`, кэшируется.
3. Невалидные поля → warn + clamp/default, **не panic**, keep-last-good на
   parse fail (как layout load сейчас: bad parse → default/warn).
4. **Никакого** `window.resize` / re-anchor / refresh geometry в этой задаче.

---

## Целевая схема (из плана §4 + T198)

```toml
version = 2

[appearance]
edge = "top"              # "top" | "bottom"  (left|right — parse, store, NOT required to validate as supported-for-apply yet; unknown → default top + warn)
height = 30               # px, f32/u32 — clamp e.g. 20..=80 (pick sane bounds, document in code)
width = "full"            # enum: Full | Fraction(f32 0.2..=1.0) | Hug
                          # TOML: "full" | "hug" | "fraction:0.7"  OR table — pick ONE format, test it
align = "center"          # "start" | "center" | "end"  (meaningful when width != full)
margin = { x = 0, y = 0 } # non-negative; used when floating/inset
floating = false
exclusive = true          # when floating=true, sanitized() MUST force exclusive=false (or exclusive_zone off) + warn once
radius = 0                # px, clamp 0..=24
elevation = "none"        # "none" | "soft" | "strong"

[widgets]                 # OPTIONAL reshape — see Compatibility below
left = [...]
center = [...]
right = [...]
known = [...]
```

### Compatibility (жёстко)

**Предпочтительно:** сохранить **плоский** v1 shape:

```toml
left = [...]
center = [...]
right = [...]
known = [...]
version = 2   # optional

[appearance]
height = 32
```

То есть **не** ломать существующий `~/.config/chronos/bar.toml` пользователя
(только widgets). Если хочешь `[widgets]` table — **оба** формата через
serde/custom deserialize: flat widgets **must** still load.

`version` отсутствует или `1` → appearance defaults, widgets as now.
`version = 2` → appearance from section or defaults if section missing.

### Defaults (= текущий hardcoded, T198)

| field | default |
|---|---|
| edge | top |
| height | 30.0 (`BAR_HEIGHT` const — **не удаляй** const; appearance default mirrors it) |
| width | full |
| align | center (unused when full) |
| margin | 0,0 |
| floating | false |
| exclusive | true |
| radius | 0 |
| elevation | none |

---

## API surface (минимум)

Что-то вроде (имена на твоё усмотрение, семантика обязательна):

```rust
// pure data
pub struct BarAppearance { /* fields above */ }
impl Default for BarAppearance { /* table above */ }
impl BarAppearance {
    pub fn sanitized(self) -> Self; // clamps + floating⇒!exclusive
}

// integrated with existing layout load
// either:
//   BarLayoutConfig { version, appearance, left, center, right, known }
// or separate BarConfig { layout, appearance } loaded from same file

pub fn cached_appearance() -> BarAppearance;
// update_cache must refresh appearance when apply() runs for widgets
```

`apply()` / watcher path: после load+sanitize обновлять **и** layout cache
**и** appearance cache. T200 подпишется на тот же apply — **ты** не зовёшь
window APIs, но cache после reload файла обязан быть свежим.

Serialize roundtrip: save v2 with appearance → load → equal sanitized.

---

## Sanitize rules (обязательные тесты)

1. `height` out of range → clamp
2. `fraction` out of range → clamp or Full+warn
3. unknown `edge` / `elevation` / `align` → default + warn (tracing)
4. `floating = true` → `exclusive = false` (forced)
5. negative margin → 0
6. bad TOML file → warn, keep previous cache if any; first load → defaults
   (mirror existing `BarLayoutConfig::load` behavior — **read it**)
7. v1 file without appearance → `BarAppearance::default()`, widgets intact
8. `width = "hug"` **parse OK** (T200 may no-op hug; schema must accept)

---

## Верификация

```
cargo test -p chronos bar::
# or more precise filter for your module names
cargo test -p chronos layout_config::
cargo test -p chronos appearance::
```

Все зелёные. Clippy clean on new code (`unwrap_used` warn — не добавляй
новых unwrap в prod paths).

**Живой шелл не нужен** (schema-only).

Коммит: `bar : appearance schema v2 in bar.toml (T199)`.
Поимённый `git add` только зона. Не коммить чужой dirt.

---

## Отчёт — формат

```markdown
# T199 report
## Schema / types (paths)
## Compatibility v1 (how)
## Sanitize rules + test names
## Cache + apply hook (no window APIs)
## Verification commands + output summary
## Что НЕ сделано
```

## Acceptance criteria (архитектор)

- [ ] v1 user `bar.toml` shape still deserializes
- [ ] appearance defaults match T198 hardcoded table
- [ ] floating forces exclusive off
- [ ] no window/fork/panel code in diff
- [ ] tests cover sanitize + v1/v2 load
