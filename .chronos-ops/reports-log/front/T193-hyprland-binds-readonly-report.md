# T193 report

> **ПРИНЯТА 2026-08-02 архитектором.** `4bce975`; parse 7/7; modules 20+25. — Hyprland binds tab (read-only v1)

**Роль:** FRONTEND. **Зона:** `side_panel_right/tab/hypr_binds.rs` (create),
`tab/mod.rs` (только `TabContent::HyprlandBinds` arm), `view.rs` (2
non-exhaustive match arm'а).

## Что сделано

1. **`tab/hypr_binds.rs` (new)** — `HyprBindsTab` entity + `Render`:
   - Read-only список биндов из **модульного** Lua-конфига Hyprland 0.55+:
     `~/.config/hypr/modules/25-binds-chronos.lua` + `20-binds-kitchen.lua`
     (порядок ChronOS → Kitchen).
   - **Fallback**: если `modules/` нет — монолит `~/.config/hypr/hyprland.lua`
     (в UI — muted-note «Loaded hyprland.lua (modules/ missing)»); если и он
     пуст/нет — честный empty «No Hyprland binds found …» (§13, без panic).
   - **Кнопка «Reload»** перечитывает файлы (без hyprctl для RO-списка).
   - **Клик по строке** → `PreviewTarget` (путь файла + `generation+1`) — тот
     же канал, что у Files (path-only, авто-переключения таба нет; T194/другие
     дадут редактор). Логируется путь.
   - Секции по source: ChronOS / Kitchen (+ монолит), колонки
     **Mod+Key · Action (ellipsis) · file:line**.
2. **`tab/mod.rs`**: `pub(crate) mod hypr_binds;`, `use hypr_binds::HyprBindsTab;`,
   `TabContent::HyprBinds(Entity<HyprBindsTab>)` variant, arm `PanelTab::HyprlandBinds
   => TabContent::HyprBinds(cx.new(|cx| HyprBindsTab::new(cx)))`. Scenes/Captures
   остались Placeholder.
3. **`view.rs`**: `TabContent::HyprBinds` в `render()` match
   (`col.child(entity.clone())`) и в `tab_entity_id` (`e.entity_id()`).
4. **Парсер (pure, не Lua AST)** — `parse_binds` по строкам:
   - трекает `mainMod = "SUPER"` / `local mainMod = mainMod or "SUPER"`;
   - `hl.bind(...)`: ключ как `"литерал"`, `mainMod`, или `mainMod .. " суффикс"`
     (резолвится в `SUPER + L`, `SUPER + SHIFT + T`, `XF86AudioLowerVolume`);
   - action до последней `)` строки (закрытие bind), `--` комментарий срезается,
     multi-line bind'ы обрезаются на первой строке (shortened-column);
   - незнакомые/закомментированные строки пропускаются.

Автор не трогал `tabs.rs` / `for_mode` (зона T192, parallel). `HyprlandBinds`
вкладка уже в `ALL` (T169); я только заполнил её content.

## Чем доказано

```
$ cargo check -p chronos            # lib + bin, 0 errors, Finished in 3.95s
$ cargo test -p chronos -- side_panel_right
test result: ok. 114 passed; 0 failed  (lib)
test result: ok. 116 passed; 0 failed  (bin)
# 7 hypr_binds::tests все ok:
#   parses_literal_key_and_line_number, resolves_main_mod_concat_to_super,
#   uses_super_default_when_no_assignment_seen, parses_local_mainmod_or_assignment,
#   strips_comment_and_closing_paren_from_action, skips_unknown_and_commented_lines,
#   empty_source_yields_empty
$ cargo build --release -p chronos  # (ждём Finishing, ошибок нет)
```

Тесты парсера — на **fixture-строках из реальных файлов** (скопированы сниппеты
`25-binds-chronos.lua` / `20-binds-kitchen.lua` в `const` внутри `mod tests`), а
не на живом `$HOME` (CI-безопасно). Живые файлы на машине подтверждены:
`~/.config/hypr/modules/` содержит `20-binds-kitchen.lua` (8823 б) и
`25-binds-chronos.lua` (625 б); формат `hl.bind(mainMod .. " + L", ...)` и
`hl.bind("XF86AudioLowerVolume", ...)` — как в спеке.

## Что НЕ сделано

- **Живой кадр не снят** — Terminal Shell без compositor/Chronos; `grim` не к
  чему. `НЕ ПРОВЕРЕНО` визуально: рейл → Hyprland binds → SUPER+L / SUPER+Q —
  за архитектором (T190). Код/тесты/сборка зелёные, пиксельной проверки нет.
- **Write-back + human explanations** — T (v2, не эта задача).
- **Автопереключение на Preview при клике** — нет (как у Files; path-only через
  `PreviewTarget`).
- **`tabs.rs` / `for_mode`** — не трогал (зона T192).

## Границы

Работал в общем рабочем дереве; других задач в `git status` не было (только
архитекторская перестановка задач: `D active/T188-…`, `D active/T190-…`,
`D report/T188-…` — не мои, не стейджил). Стейджил только свои файлы:
`hypr_binds.rs`, `tab/mod.rs`, `view.rs`, этот отчёт.
