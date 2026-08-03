# T203 — Agent dogfood: NL → bar schema + Follow shows bar.toml

**Статус:** active **T201 ACCEPTED `51219ab` — go**; T195 Follow **optional accelerator**.
**Роль:** FRONTEND (+ skill/docs). **Модель: Sonnet 5**.
**План:** live-customization §5 T203 + epic DoD §7.
**Канон:** PRODUCT — «слова агента = те же ключи, что в файле».
**Правила:** `RULES.md`.

**Зависимости:**
- **T201** accepted (get/set/list or documented file schema skill)
- **T200** accepted (hot apply) — otherwise dogfood is write-only
- **T195** Follow — if not done: minimal «last config path» toast **or**
  open `bar.toml` in Editor via `PreviewTarget` when agent tool reports
  path; full Follow UI can stay residual

**Зона:**
- Hermes / ChronOS skill package from T201 — **extend** with NL examples
  and forbidden actions
- optional: left agent empty-state / system prompt snippet path if ChronOS
  injects session prompts (find existing character/prompt inject; don't
  invent second stack)
- thin glue: when bar config saved by tool, set `PreviewTarget` to
  `bar.toml` if Follow-like flag on — **only** if cheap with T195 state;
  else document manual «open bar.toml» from T202

**НЕ:** new agent backend; multi-agent; redesign chat; bar schema changes;
T204 rails.

**Отчёт:** `docs/orchestration/tasks/report/T203-bar-agent-dogfood-report.md`.

---

## Цель (epic demo)

Пользователь в agent panel:

> бар снизу, 80% ширины по центру, скругление 12, тень, без cava, clock справа

Без logout / pkill:
1. Агент мапит фразы → schema keys (`edge=bottom`, `width=fraction:0.8`,
   `align=center`, `radius=12`, `elevation=soft|strong`, widgets patch).
2. Пишет через T201 API or skill-guided file edit.
3. T200 apply → бар меняется.
4. User sees **which file** changed (Follow or Editor open on `bar.toml`).

### Deliverables

1. **Skill / prompt card** (must):
   - table: NL phrase → field
   - full example multi-change turn
   - «never pkill chronos / never recompile for chrome»
   - point at `get_bar_config` before patch (read-modify-write)
   - sanitize limits (height clamp, floating⇒!exclusive)

2. **Smoke script or checklist** in report (not necessarily automated):
   - steps + expected bar.toml fragment
   - commands to cat config after turn

3. **Optional code:** one hook so successful `set_bar_config` logs
   `tracing::info!(path, "bar: agent applied")` and optionally
   `PreviewTarget { path: bar.toml, intent: View }`.

### Out of scope

- Guaranteeing Hermes model compliance (skill quality only)
- Undo UI (mention bak if exists)
- Hyprland decoration NL

### Verification

```
# if code:
cargo test -p chronos bar::
```

Live dogfood (preferred): real agent turn with phrase above + grim before/after.
If no agent session: mark LIVE NOT VERIFIED, skill file must still land.

Коммит: `agent : bar customization dogfood skill (T203)`  
(and code commit only if hook added).

---

## Отчёт

```markdown
# T203 report
## Skill path + NL→key table
## Hook / Follow integration (or residual)
## Live dogfood evidence / NOT VERIFIED
## Что НЕ сделано
```

## Acceptance

- [ ] Skill exists and uses **same keys** as T199 schema / T201 API
- [ ] Epic phrase is fully specified as patch example
- [ ] No instruction to restart shell as happy path
- [ ] Honest live section
