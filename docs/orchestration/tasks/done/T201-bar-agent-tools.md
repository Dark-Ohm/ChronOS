# T201 — Agent tools: get/set/list bar config (widgets + appearance)

**Статус:** active **blocked on T199 accept**. **Роль:** BACKEND
(зона `crates/app/bar` + skill/docs; UI не трогать).
**Модель:** GLM 5.2 / Sonnet / DeepSeek.
**План:** `docs/superpowers/plans/2026-08-02-live-customization.md` §5 T201.
**Канон:** `docs/PRODUCT.md` § Live desktop customization п.5:
`list_bar_widgets` / `get_bar_config` / `set_bar_config` (patch merge).
**RECON:** `report-log/T198-…`. **Правила:** `RULES.md`.

**Не начинай**, пока T199 не в `done/` + report-log ACCEPTED (appearance
types + cache + sanitize). T200 (apply) **желателен** для live proof, но
tools могут писать файл → inotify apply, если T200 ещё не влит — tools
всё равно валидны (write disk); live visual = T200.

**Параллельно запрещено:** `side_panel_*` UI, T194c, T204.

**Зона:**
- `crates/app/src/bar/` — new `agent_api.rs` **or** methods on layout/appearance
  modules: pure get/patch/list + save
- optional thin CLI: `crates/app` binary flag / `chronos bar …` **only if**
  already have subcommand pattern; else pure Rust API + skill is enough
- skill for Hermes: e.g. `skills/chronos-bar-config/SKILL.md` or
  `docs/skills/…` — **where project already puts agent skills**; if none,
  `docs/agent/bar-config-tools.md` + path noted in report
- unit tests: patch merge, reject garbage, roundtrip

**НЕ:** GPUI System settings page (T202); presets UI; Follow UI (T195/T203);
fork; hyprctl tools (optional note only).

**Отчёт:** `docs/orchestration/tasks/report/T201-bar-agent-tools-report.md`.

---

## Цель

Агент (Hermes / любой) меняет бар **только** через structured API → тот же
`bar.toml`, что редактирует человек. Один словарь ключей (PRODUCT).

### API (имена можно чуть иначе; семантика обязательна)

```text
list_bar_widgets()
  → { left: [...], center: [...], right: [...], known: [...],
      available: [...BUILTIN + known plugins...] }

get_bar_config()
  → full snapshot: version, appearance {…}, widgets { left/center/right/known }

set_bar_config(patch)
  → merge patch into current, sanitize, save bar.toml, return
    { ok, applied: snapshot, warnings: [...] }
  patch examples:
    { "appearance": { "height": 40, "radius": 12 } }
    { "widgets": { "center": ["mpris"] } }   // replace section lists if present
    { "widgets": { "remove": ["cava"], "add_right": ["clock"] } }  // optional sugar
```

**Merge rules:**
- Missing keys in patch = leave current.
- Widget section present as full array = replace that section.
- Appearance subfields partial merge.
- Always run `sanitized()` from T199 before save.
- Save failure / invalid → **no silent corrupt**; keep last-good cache;
  return err string for agent.
- After successful save: call existing `apply(cx)` **if** invoked in-process
  with `App`; if API is pure+disk-only, document that inotify watcher applies
  (T134/T200). Prefer: `apply_from_disk(cx)` helper used by tools when `cx`
  available.

**set_theme** (PRODUCT): **out of T201 core** unless free — one-liner
pointing at `theme_config` is enough as residual; do not expand scope.

### Surface for the agent

Pick **one** primary (report which):

1. **Preferred:** pure functions + **Hermes skill** that says: read/write
   `~/.config/chronos/bar.toml` via normal file tools **using this schema**,
   OR call `chronos-bar` if you add a tiny CLI:
   `chronos bar get | jq` / `chronos bar set --patch '{...}'`
2. **Also good:** JSON-RPC / IPC message already used by shell IPC — only if
   cheap mirror of existing `ipc/messages.rs` patterns.

Do **not** invent a second parallel config format.

### Tests

- get returns defaults when no file
- patch height only preserves widgets
- remove unknown widget → sanitize/warn
- floating true forces exclusive false (T199 rule)
- roundtrip save/load temp dir (not user HOME)

```
cargo test -p chronos bar::
```

Коммит: `bar : agent get/set/list config API (T201)`.

---

## Отчёт

```markdown
# T201 report
## API surface (signatures + path)
## How agent invokes (skill / CLI / IPC)
## Merge + sanitize
## Tests
## Что НЕ сделано (theme tool, hypr, …)
```
