# T186 report — Gamer rail: Library / Scenes / Captures + иконки

> **ПРИНЯТА 2026-08-02 архитектором.** `102fef4`; tabs:: 29/29; icons in
> assets.rs; Developer 14 / Gamer 10; live grim — NOT VERIFIED (ok for code
> gate, T191 P1/P8).

**Роль:** FRONTEND. **Зона:** `side_panel_right/tabs.rs`, `side_panel_right/tab/mod.rs`
(только новые `TabContent` arm'ы → Placeholder), `assets.rs`, три новых SVG.

## Что сделано

1. **Три новых `PanelTab`**: `Library`, `Scenes`, `Captures` (группа «Gamer
   at-rest hub tools (§4.2)» в enum, после `SourceControl`, до settings).
2. **`ALL` 14 → 17**: новые три вставлены между work-tools и settings-группой
   (индексы 8/9/10), settings-хвост сдвинулся на 11–16. Порядок work-tools и
   settings внутри групп не тронут.
3. **`for_mode`**:
   - `Developer` — 14 workbench-вкладок явным списком (System + 7 work tools +
     6 settings); **не** `ALL.to_vec()` (ALL теперь 17). Library/Scenes/
     Captures отсутствуют.
   - `Gamer` — 10: `System, Library, Scenes, Captures,` + 6 settings в
     прежнем порядке.
4. **`parse_id` / `id()` round-trip**: `library` / `scenes` / `captures`.
   `parse_id` — case-insensitive + hyphen→underscore (как у соседей).
5. **`preferred_content_width`**: Library **480**, Scenes **400**, Captures
   **320** (Captures включён в группу empty-state 320 — честный unavailable).
6. **`label` / `icon_path`**: «Library»/«Scenes»/«Captures» + три новых
   `icons/rail-*.svg`.
7. **Иконки SVG** (`crates/app/assets/icons/`):
   - `rail-library.svg` — полка с тремя карточками игр на базе.
   - `rail-scenes.svg` — стек карт-пресетов (fill-opacity 0.35/0.65/1.0).
   - `rail-captures.svg` — камера (корпус + видоискатель + линза).
   - Все viewBox 256, `fill="currentColor"`, **без** `mix-blend-mode` /
     `destination-out` (урок T172). `xmllint --noout` — валидный XML.
8. **`assets.rs`**: все три зарегистрированы в `icons!` (алфавит) — иначе
   пустые слоты (урок T169). `include_bytes!` подтвердил существование файлов
   на этапе `cargo check`.
9. **`TabContent::create`**: явные arm'ы `Library | Scenes | Captures →
   Placeholder(EmptyTab)` с комментом «placeholder until T188/T189; Captures
   — no backend this slice (§13)». Catch-all остался для постоянных
   placeholder'ов.
10. **`placeholder_description`** (§13, без «coming soon» / дат):
    - Library: «List, pin and launch detected games»
    - Scenes: «Activate per-game scenes and profiles»
    - Captures: «Unavailable - no capture backend» (честный unavailable —
      backend'а захвата нет в этом слайсе, slice 6).

## Чем доказано

```
$ cargo test -p chronos tabs::
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 124 filtered out
# (бинарь: те же 29 passed; 0 failed)

$ cargo test -p chronos -- side_panel_right
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 213 filtered out

$ cargo build --release -p chronos
Finished `release` profile [optimized] target(s) in 2m 54s
# errors: 0  (grep -icE 'error\[|error:|panicked' → 0)
```

SVG-валидация:
```
$ xmllint --noout crates/app/assets/icons/rail-library.svg   → OK
$ xmllint --noout crates/app/assets/icons/rail-scenes.svg     → OK
$ xmllint --noout crates/app/assets/icons/rail-captures.svg   → OK
$ grep -c 'mix-blend-mode\|destination-out' rail-{library,scenes,captures}.svg → 0/0/0
```

Регистрация в `assets.rs`:
```
$ grep -n 'rail-library\|rail-scenes\|rail-captures' crates/app/src/assets.rs
48:    "rail-captures.svg",
52:    "rail-library.svg",
56:    "rail-scenes.svg",
```

Новые/переписанные тесты (все PASS):
- `all_has_seventeen_tabs_in_fixed_order` (был `..._fourteen_...`) — все 17
  индексов.
- `developer_rail_is_fourteen_workbench_tabs_without_gamer_tools` (был
  `..._full_catalog_of_fourteen`) — 14 явных вкладок, отсутствие 3 gamer,
  `dev != ALL`.
- `gamer_rail_is_ten_tabs_with_three_hub_tools` (был
  `..._stays_seven_...`) — 10 вкладок, System first, settings-хвост в
  порядке.
- `developer_settings_group_matches_gamer_settings_group_order` —
  `gamer[1..]` → `gamer[4..]` (3 hub-вкладки между System и settings).
- `parse_id_round_trip_for_gamer_hub_tools`,
  `parse_id_accepts_case_and_hyphen_variants_for_gamer_hub_tools`,
  `parse_id_rejects_unknown_names_including_new_ones` (добавлены
  singular-bogus: `lib`/`scene`/`capture`/`games`/`librarys`).
- `gamer_hub_tabs_have_distinct_icon_paths`.
- `library_preferred_width_is_480`, `scenes_preferred_width_is_400`;
  Captures добавлен в `empty_state_tabs_preferred_width_is_320`.
- Авто-coverage по `ALL` (теперь 17): `every_tab_has_a_non_empty_label`,
  `every_tab_has_a_distinct_icon_path`, `every_preferred_width_in_valid_range`,
  `every_tab_has_a_nonempty_placeholder_description`,
  `placeholder_descriptions_are_unique`, `empty_tab_has_a_label` — все PASS.

## Что НЕ сделано

- **Живой кадр не снят** — среда Terminal Shell без запущенного compositor/
  Chronos; `grim` применить не к чему. `НЕ ПРОВЕРЕНО` визуально: иконки на
  рейле в Gamer mode (10 слотов, три новых не пустые) — за архитектором
  (T190 P1/P8). Код/тесты/сборка зелёные, рендер-корректность SVG по
  структуре (валидный XML, `fill="currentColor"`, регистрация в `assets.rs`)
  — но не подтверждена пиксельно.
- **Реальный UI Library/Scenes** — T188/T189 (по заданию).
- **`scene::activate`** — T185/T189 (зона scene.rs, не трогал).
- **Games filter / `.desktop` Categories** — T187 (зона applications).
- **`Developer` `for_mode`** больше не `ALL.to_vec()` — намеренно (ALL вырос
  до 17, Developer rail остался 14). Тест переписан с явным списком.

## Зона / выход за пределы

Выходов за зону нет. `view.rs`, `rail.rs`, `scene.rs` не трогал (у них нет
исчерпывающего `match` на `PanelTab` — проверено `grep
'PanelTab::HyprlandBinds =>'` по `crates/`: только `tabs.rs` и `tab/mod.rs`).
`scene.rs` упоминает `PanelTab` только в комментариях (T185 зона).
