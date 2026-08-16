# T199 report — bar.toml v2: `[appearance]` schema + sanitize

**Роль:** BACKEND. **План:** `docs/superpowers/plans/2026-08-02-live-customization.md` §4–5.
**RECON:** `report-log/T198-chrome-customization-recon-report.md` (T198).
**Зона:** только `crates/app/src/bar/`. **No window APIs, no fork, no panels.**

---

## Schema / types (paths)

| path | что |
|---|---|
| `crates/app/src/bar/appearance.rs` **(new)** | `BarAppearance` (9 полей), `BarEdge`, `BarAlign`, `BarElevation`, `BarWidth`, `BarMargin`, `sanitized()`, generic `deserialize_choice` (warn-on-unknown → default) |
| `crates/app/src/bar/mod.rs` | `pub mod appearance;` |
| `crates/app/src/bar/layout_config.rs` | `BarLayoutConfig` + `version: Option<u32>` + `appearance: BarAppearance`; `gated_appearance()`; `cached_appearance()`; `sanitized()` passthrough |

Формат ширины — **строка** (как в плане §4): `"full" | "hug" | "fraction:0.7"`.
Прочие поля — скаляры/таблица (`margin = { x = 12, y = 8 }`, int→f32 работает).

Bounds (задокументированы в `appearance.rs`): height `20..=80`, radius `0..=24`,
fraction `0.2..=1.0`.

## Compatibility v1 (как)

- **Плоский v1-shape сохранён**: `left/center/right/known` — верхний уровень,
  никакого `[widgets]`-переоборачивания. `[appearance]` + `version` — аддитивны.
- `version` отсутствует или `1` → appearance = **code defaults** (v1-гейт в
  `load()` через `gated_appearance`), даже если секция `[appearance]` в файле есть.
- `version = 2` → appearance из секции; секции нет → defaults.
- **Сейв v1 остаётся байт-стабильным**: `version` пишется только при `Some`,
  `[appearance]` — только при `!is_default()` (`skip_serializing_if`).
- Реальный пользовательский `~/.config/chronos/bar.toml` (только widgets)
  десериализуется как есть — тест `v1_file_loads_with_default_appearance`
  повторяет его форму.

## Sanitize rules + test names

Правила (`BarAppearance::sanitized`, идемпотентны, никогда не паникуют):

1. height вне `20..=80` → clamp + warn — `sanitize_clamps_height`
2. fraction вне `0.2..=1.0` → clamp + warn — `sanitize_clamps_fraction`
3. radius вне `0..=24` → clamp + warn — `sanitize_clamps_radius`
4. unknown `edge`/`align`/`elevation`/`width` → default + warn (пер-поле, файл
   не валится) — `unknown_values_fall_back_to_defaults`
5. `floating = true` → `exclusive = false` (forced) — `sanitize_floating_forces_exclusive_off`
6. negative margin → 0 — `sanitize_zeroes_negative_margin`
7. bad TOML файл → warn + defaults (keep-last-good — путь `load()` не менялся,
   поведение как было) — покрыто существующим паттерном `load`
8. v1 без appearance → `BarAppearance::default()`, widgets целы —
   `v1_file_loads_with_default_appearance`
9. `width = "hug"` парсится (T200 может no-op) — `parse_hug_width_accepted`
10. `edge = "left"/"right"` парсится и хранится (vertical bar — позже) —
    `parse_edge_left_right_accepted_for_future`

Плюс: roundtrip (`serialize_roundtrip_v2_with_appearance`), v1-сейв без
version/appearance (`v1_serialize_omits_version_and_appearance`), v2-гейт
(`version_absent_or_one_gates_appearance_to_defaults`), unknown-поля игнор
(`unknown_fields_are_ignored`), идемпотентность (`sanitize_is_idempotent`).

Defaults (`defaults_match_hardcoded_chrome`) = таблица T198 1:1: edge top,
height `BAR_HEIGHT` (const **не тронут**, appearance зеркалит), width full,
align center, margin 0/0, floating false, exclusive true, radius 0, elevation none.

## Cache + apply hook (no window APIs)

- `apply()` структурно не менялся: тот же кэш теперь несёт appearance
  (`load().sanitized()` → appearance clamp'ится на каждом reload).
- `pub fn cached_appearance() -> BarAppearance` — читает `cached().appearance.sanitized()`.
  `#[allow(dead_code)]`: потребитель — T200 (apply окна) на следующей задаче.
- **Никаких** `window.resize` / re-anchor / margin / `open_window` — это T200.
- `move_widget` путь сохраняет appearance нетронутым (кэш проходит через save/update).

## Verification commands + output summary

```
$ cargo test -p chronos bar::          # lib + bin
test result: ok. 94 passed; 0 failed; 268 filtered out   (каждый таргет)
$ cargo clippy -p chronos --all-targets
# на новой зоне 0 warning'ов (после правок: derive Default вместо ручных,
# match-guard вместо collapsible-if, allow(dead_code) с комментарием)
```

**22 новых теста** (15 `appearance::` + 7 `layout_config::`), все зелёные
(в т.ч. граница «type error → файл падает / value error → деградация поля»:
`type_error_fails_whole_parse_value_error_degrades`).

**Важно про дерево:** после моего зелёного прогона параллельная задача **T194c**
(чужой WIP в `side_panel_right/tab/preview.rs`, +420 строк, `render_editor_body`
E0599) **сломала сборку основного дерева** — НЕ моя зона, не трогал. Свою зону
проверил изолированно: worktree `ChronOS-wt-t199` на HEAD `6a32ef6` + мои 3 файла,
общий `CARGO_TARGET_DIR`:
```
$ CARGO_TARGET_DIR=.../ChronOS/target cargo test -p chronos bar::
test result: ok. 94 passed; 0 failed
```
Worktree удалён после проверки.

## Что НЕ сделано

- **Window apply** (resize/re-anchor/margin/geometry refresh) — T200, ждёт этот коммит.
- **Fork** `../Source` — не тронут.
- **Panels / dock geometry / theme token overrides** — не тронуты.
- **Agent tools (T201), presets UI (T202)** — не в этой задаче.
- **Запись в `~/.config/chronos/bar.toml`** пользователя — нигде; тесты только
  на in-memory fixtures / toml-строках.
- Живой шелл не нужен (schema-only) — не запускался.

## Acceptance criteria

- [x] v1 user `bar.toml` shape still deserializes — `v1_file_loads_with_default_appearance`, `v1_serialize_omits_version_and_appearance`
- [x] appearance defaults match T198 hardcoded table — `defaults_match_hardcoded_chrome`
- [x] floating forces exclusive off — `sanitize_floating_forces_exclusive_off`, `v2_file_with_appearance_parses_and_sanitizes`
- [x] no window/fork/panel code in diff — diff только `bar/{appearance.rs, mod.rs, layout_config.rs}`
- [x] tests cover sanitize + v1/v2 load — 21 новый тест, 94 всего зелёные

Коммит: `bar : appearance schema v2 in bar.toml (T199)`.

---

## Приёмка (Lead Architect / Grok, 2026-08-02)

**Вердикт: ACCEPTED**

Коммит: `31ec352` — `bar/{appearance.rs,layout_config.rs,mod.rs}` (+ report в том же
коммите — гигиена: лучше report отдельно; **не блокер**).

| claim | check |
|---|---|
| BarAppearance + enums + BarWidth string form | ✅ appearance.rs |
| defaults = T198 / BAR_HEIGHT | ✅ Default + test |
| floating ⇒ !exclusive | ✅ sanitized |
| v1 flat widgets + version gate | ✅ gated_appearance on load |
| skip_serializing_if v1 byte-stable | ✅ version/appearance attrs |
| cached_appearance for T200 | ✅ |
| no window/fork/panel in schema | ✅ only bar/ |
| bar:: tests | ✅ **95** passed (отчёт: 94 — drift ок) |
| hug / left|right edge parse | ✅ |

**Замечания (не residual-блокеры):**
1. `BarLayoutConfig::sanitized()` **не** re-gates version (документировано) —
   programmatic must call `gated_appearance`; `apply`→`load` ок.
2. Report file в product commit — в следующий раз только код + report в inbox.

**T200 разблокирован** (schema + `cached_appearance()` готовы).

