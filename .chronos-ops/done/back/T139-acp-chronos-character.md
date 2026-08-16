# T139 — Left agent panel ChronOS character (not Zed clone)

**Статус: OPEN. После T140–T141 (functional chat+tools).**  
**Канон:** `docs/design/Agent Panel.dc.html`, `docs/design/Agent Thread.dc.html`; Theme tokens.

| | |
|---|---|
| **Skills** | `chronos-shell`, elevation if needed |
| **Зоны** | `side_panel_left/{panel,chat_view,composer,sessions_list}.rs` only |
| **Отчёт** | `docs/orchestration/tasks/report/T139-acp-chronos-character-report.md` |
| **Коммит** | `ui : left agent panel ChronOS density/identity (T139)` |

## Контекст

User: «вид спизжен у Zed без изюминки». Layout pattern OK; chrome/density not
ChronOS. Right panel + themes already have surface language — align left.

## Цель

Grim vs mockup: denser empty state, header/status ChronOS language, composer
caret/density; still Theme-aware (light/dark). **No GPL line copy from Zed.**

## Задачи

1. Empty thread: less void, mockup-aligned placeholder.
2. Header cluster: accent/status consistent with shell.
3. Message bubbles / gaps match design tokens (bg.elevated, radii).
4. Composer: clear focus ring/caret; attach/send affordance polish.
5. grim dark+light.

## Accept

- [ ] User or Architect: «feels ChronOS» not «Zed port».
- [ ] Light theme not broken (regression `8e8043e`).
- [ ] No functional regression of T137–T141.

## Out of scope

- New agents, ACP protocol, Files tab.
