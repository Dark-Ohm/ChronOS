# T185 — scene: per-game fields + activate + seed hub

**Статус:** active. **Роль:** BACKEND.
**Правила:** `docs/orchestration/agents/RULES.md`.
**План:** `docs/superpowers/plans/2026-08-02-gamer-hub-slice-5.md` (A, утверждён).
**Разведка:** `docs/orchestration/tasks/report-log/T184-gamer-hub-recon-report.md`
(прочитать §4, §9; эррата про dock — не твоя зона).

**Параллельно с тобой:** T186 (rail FRONTEND), T187 (apps+games.toml BACKEND).
Зоны **не пересекаются**. Не жди их.

**Зона (только):**
- `crates/app/src/scene.rs`

**НЕ трогать:** `tabs.rs`, `tab/**`, `gaming_mode.rs` (T190), `applications/**`
(T187), `workspace_mode.rs` (кроме если `set` уже зовёт `restore_for_mode` —
не менять), UI.

**Отчёт:** `docs/orchestration/tasks/report/T185-scene-per-game-activate-report.md`.
После отчёта — стоп. Не `done/`, не «принята».

---

## Цель

1. Расширить `Scene` optional-полями (v1, без миграции).
2. `pub fn activate(cx, id) -> Result<(), ActivateError>` — user path:
   выставить `active`, обновить `[last]` для mode сцены, **`save_config`**.
3. Seed builtin hub, если в конфиге нет ни одной gamer-сцены (или нет
   id=`hub`) — при `init` / явной `ensure_builtin_hub`, **без** затирания
   пользовательских сцен.
4. `restore_for_mode` остаётся **read-only на диске** (T164).

## Поля `Scene` (добавить с `#[serde(default)]`)

| field | type | default | смысл |
|---|---|---|---|
| `kind` | `String` | `""` | `"hub"` \| `"game"` (пусто: hub если `app` пуст, иначе game) |
| `app` | `String` | `""` | desktop id / launch key (для game) |
| `apply_gaming_profile` | `bool` | `false` | T190 применит; **ты только хранишь** |

Опционально (если дёшево, тоже default empty) — можно отложить в `extra`:
`audio_sink`, `microphone`, `hyprland_workspace` как `String` default `""`.
Минимум — три поля выше.

## `activate`

```text
// псевдоконтракт
pub enum ActivateError { NotFound, InvalidMode, Io /* если надо */ }

pub fn activate(cx: &mut App, id: &str) -> Result<(), ActivateError>
```

Поведение:
1. Найти сцену в `config.scene` (после filter_valid для выбора; оригинал
   конфига не выкидывать невалидные записи с диска).
2. Если нет — `NotFound`.
3. `active = Some(scene.clone())`.
4. `last[mode_label] = id` (developer/gamer).
5. `save_config` — снять `#[allow(dead_code)]`, вызывать **только** отсюда
   (и seed, если seed пишет файл — см. ниже).
6. `tracing::info!(scene=%id, mode=..., "scene: activated")`.
7. **Не** звать `GamingModeState` (T190).
8. **Не** звать `workspace_mode::set`.

## Seed hub

Если после `load_config` нет сцены с `id == "hub"` и `mode` gamer:

- добавить в **память** `Scene`:
  - id=`hub`, name=`Game Hub`, mode=`gamer`, kind=`hub`
  - rail_tabs: `system, library, scenes, captures, acp_settings, mcp_settings,
    lsp_settings, api_providers, editor_settings, hyprland_binds`
    (строки — T186 добавит parse_id; неизвестные T186 ids пока warn-скипнутся
    в `resolve_for_mode` — это ок до merge T186)
  - active_tab=`library`, dock=`steam,discord,firefox,kitty`
- `last.gamer = "hub"` если ключа не было
- **Запись на диск seed:** да, один раз через `save_config`, если файла не
  было **или** hub отсутствовал — чтобы следующий cold start видел hub.
  Если файл есть с чужими сценами без hub — **добавь** hub, не стирай чужое.

Тесты pure: parse round-trip новых полей; activate обновляет last (на
in-memory cfg + mock path если нужно — смотри как тестируют load сейчас);
restore_for_mode **не** пишет диск (существующий контракт).

## Верификация

```
cargo test -p chronos scene::
cargo clippy -p chronos --all-targets
```

Коммит: `scene : per-game fields + activate + hub seed (T185)`.
`git add` поимённо, только `scene.rs` (+ тесты в том же файле).

## Что НЕ делать

- UI, rail tabs enum, Library/Scenes views
- apply gaming profile
- categories / games.toml (T187)
