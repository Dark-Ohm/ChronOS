# T198 — RECON: hardcoded chrome props → gap to bar appearance schema

**Статус:** active. **Роль:** RECON.
**План (утверждён):** `docs/superpowers/plans/2026-08-02-live-customization.md`.
**Канон:** `docs/PRODUCT.md` § Live desktop customization.
**Правила:** `docs/orchestration/agents/RULES.md`.

**Параллельно:** T194 Editor (FRONTEND) — **не пересекайся** с `tab/preview*`,
`bar/` можно только **читать**.

**Ты ничего не пишешь в продуктовый код.** Только отчёт.

**Отчёт:** `docs/orchestration/tasks/report/T198-chrome-customization-recon-report.md`.

---

## Цель

Карта для T199–T200: что **уже** в конфиге/hot-reload, что **зашито** в коде,
что нужно для сценария «бар снизу / floating / fraction width / radius / widgets».

## Читать

| путь | зачем |
|---|---|
| `crates/app/src/bar/mod.rs` | window_options, anchors, BAR_HEIGHT, open/resize |
| `crates/app/src/bar/layout_config.rs` | bar.toml schema, hot-reload, apply_layout |
| `crates/app/src/theme_config.rs` | theme hot-reload |
| `crates/ui/src/theme/**`, `elevation.rs` | tokens shadow/blur |
| `crates/app/src/dock/config.rs` | dock.toml parity |
| `crates/app/src/edit_mode.rs` | visual layout edit |
| layer-shell skill / fork LayerShellOptions | what can change live |
| `~/.config/chronos/bar.toml` (read-only) | real user file |

## Ответить

### 1. Table: property → status

For each (file:line where hardcoded or configured):

- bar edge (top/bottom)
- height
- width full vs fraction
- align
- margin
- floating
- exclusive zone
- radius / clip
- shadow / elevation / blur
- bg color / theme binding
- widget lists L/C/R
- hot-reload: which of the above re-apply without process restart today?

### 2. Apply path

When `bar.toml` changes, what runs? Can `window_options` / layer surface be
updated mid-session (resize, re-anchor) without `remove_window`+new? Cite fork
or app code.

### 3. Gap to plan §4 schema

For each field in plan `[appearance]` — exists / needs new field / blocked by
compositor.

### 4. Risks for T200

Top→bottom flip, floating+exclusive, hug width measurement.

### 5. Out of scope confirmed

Vertical bar, multi-bar — later unless free.

## Формат отчёта

```markdown
# T198 report
## 1 Property table (file:line)
## 2 Hot-apply path today
## 3 Schema gap
## 4 Risks T199/T200
## 5 Out of scope
## Что НЕ сделано
```

Коммит: only report if you commit. `recon : T198 chrome customization map`.
