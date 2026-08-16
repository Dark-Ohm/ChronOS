# Edit Mode + Hot Reload — design

_2026-07-26. Продуктовый «режим редактирования» + подключение hot-reload
слоёв, которые уже частично в дереве. Не Plasma-applet editor (отклонён
DECISIONS 2026-07-24). Не full hot-swap `crates/app` (отклонён там же)._

## Зачем

Daily driver уже: панели, темы, exclusive. Дальше нужна **кастомизация без
рестарта** и **dev-цикл save→see**, не новые фичи-модули.

Два разных «hot reload» нельзя смешивать:

| Слой | Что перезагружается | Механика | Статус |
|------|---------------------|----------|--------|
| **Config** | layout / theme / dock | inotify → apply → `notify` | theme.toml ✅; dock.toml ✅; **bar.toml ❌** |
| **Luau plugins** | plugin source | inotify PluginManager | ✅ (watcher) |
| **Dev UI dylib** | pure render fn | `hot-lib-reloader` + `crates/hotview` | ✅ network only |

**Edit Mode** — продуктовый runtime-флаг: «сейчас можно править раскладку».
Он **не** заменяет dylib-reload; он **пишет/читает config** и даёт UI
affordances. Hot-reload config — то, что делает правки «живыми».

## Не делаем (явный non-goals)

1. Plasma-слоты / «всё applets» — DECISIONS 2026-07-24.
2. Hot-swap всего `crates/app` dylib — UAF на Entity/subscribe.
3. `subsecond` / ThinLink — `unsafe` в app, отклонён bake-off.
4. Per-monitor layout, новые произвольные панели на краях — later.
5. Side panel internal layout config — later (другая модель, не BarRegistry).
6. T129 motion — parked, не смешивать.

## Phase map

### Phase 1 — Config truth + Edit Mode shell (T134)

**Цель:** `~/.config/chronos/bar.toml` = порядок виджетов; правка файла или
из edit-mode UI **без рестарта** бара; глобальный Edit Mode.

Спека layout уже есть:
`docs/superpowers/specs/2026-07-24-bar-widget-layout-config-design.md`
(Default = текущий `register_builtin`).

Доставить:

1. `bar/layout_config.rs` — load/default/validate unknown names (warn + skip).
2. Apply: rebuild registry order from config (не только push; reorder).
3. Hot-reload watcher (паттерн `theme_config::spawn_watcher`).
4. `EditModeState` global: `{ active: bool }`.
5. IPC + keybind `toggle-edit-mode` (предложение: Super+Shift+E).
6. Visual: когда `active` — badge «EDIT» на bar (или outline accent), курсор.
7. **Minimal edit actions (без drag):** в edit mode клик по виджету /
   long-press / small «⋯» — popup: Move left / Move right / Remove from section
   / Add widget… Persist → write `bar.toml` → apply (same path as watcher).

Accept: edit TOML by hand → bar reorders; toggle edit mode → badge; move
widget in UI → file + bar match; release build без `hot-reload` feature
зелёный.

### Phase 2 — Drag edit (T135, после T134)

Drag-and-drop reorder внутри Left/Center/Right; drop between lanes.
Пишет тот же `bar.toml`. Только поверх стабильного apply из Phase 1.

### Phase 3 — Dev hotview expansion (T136, parallel OK after T134 start)

- Документ dev workflow: `cargo watch -w crates/hotview` + `cargo run -p chronos --features hot-reload`.
- Вынести 1–2 pure renders (clock text, battery glyph) в `hotview` по
  образцу `render_network` — **не** state/subscribe.
- Skill `hot-lib-reloader` уже канон.

### Phase 4 — Optional later

- Panel chrome config (widths, default dock).
- Plugin-placed widgets in bar sections.
- Noctalia-style multiple bars — только после Phase 1–2 доказали себя.

## Edit Mode state (Phase 1)

```rust
// crates/app/src/edit_mode.rs (new)
pub struct EditModeState {
    pub active: bool,
}
impl Global for EditModeState {}

pub fn toggle(cx: &mut App) {
    let s = cx.global_mut::<EditModeState>();
    s.active = !s.active;
    tracing::info!(active = s.active, "edit_mode: toggled");
    cx.refresh_windows(); // bar + any chrome that paints edit chrome
}
```

- Init in `main` next to `theme_config::init`.
- IPC payload `toggle-edit-mode` (mirror `toggle-theme`).
- Hyprland bind documented in HANDOFF / hypr conf sample if present.

**Правило:** edit mode **не** ломает click targets в normal mode. В edit
mode widget primary click = edit affordance, not open popup (volume/system
open only when `!edit_mode.active`).

## Hot-reload config pipeline (shared)

Reuse `theme_config` pattern:

1. OS thread `inotify` on `~/.config/chronos/`.
2. Debounce 300ms.
3. `cx.update(|cx| apply(cx))` on GPUI executor (not tokio UI).
4. Missing/broken file → default + warn, **never silent rewrite** unless
   user action saved from UI.

`apply_bar_layout(cx)`:

- Read config.
- Clear or rebuild per-section order using known name→register table.
- Unknown name: `tracing::warn!`, skip.
- Empty section allowed.
- `cx.refresh_windows()` / bar entity `notify`.

## Relationship to Shell-IDE

Edit Mode ≠ «вошёл в IDE». IDE = side panels (agent left, tabs right).
Edit Mode = **chrome layout customization** (bar first). They coexist:
edit mode can be on while agent panel open.

## Verification (Phase 1)

```bash
chronos-rebuild && chronos-stop && chronos-start
# bar.toml missing → default order matches pre-change
# edit bar.toml order → bar updates without restart
# Super+Shift+E → EDIT chrome; move widget → file updates
# cargo build --release -p chronos   # no hot-reload feature required
# cargo run -p chronos --features hot-reload  # still builds; network hotview OK
```

## Risks

| Risk | Mitigation |
|------|------------|
| Separator multi-register (same name twice) | unique ids `separator-left` or allow multi with list semantics |
| Widget register needs section | factory table takes `BarSection` from config placement |
| Edit mode eats volume click | gate popup open on `!EditModeState.active` |
| Race double-apply | debounce + serial apply on GPUI thread |

## Open questions for Architect (defaults if no answer)

1. Keybind: **Super+Shift+E** (Recommended) vs Super+E.
2. Phase 1 UI: **⋯ menu on widget** (Recommended) vs only TOML.
3. Separators: **one name per section slot** vs multi-instance ids.

---

**Next deliverable:** T134 brief implement Phase 1 only.
