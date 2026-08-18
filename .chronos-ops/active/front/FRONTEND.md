# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

**Активное:**
- **T302** — `T302-left-panel-content-blank.md`. P1, живая находка:
  контентная зона левой панели рендерится пустой, сквозь неё видны
  обои. Начинай с этого.
- **T304** — `T304-tabcontent-create-generalize-to-app.md`. Предварительный
  для T305, режется первым (общий `tab/mod.rs`, параллелить с T305 нельзя).
- **T305** — `T305-control-center-popup-host.md`. Стартует только после
  приёмки T304. Settings-табы right rail уезжают в единый anchored-popup
  (control-center, видео-референс владельца).
- **T303** — `T303-frame-wrap-border-geometry-mismatch.md`. P2 (снижено —
  геометрия уже в `d01820e`), хвост: `wrap.thickness`, debug-лог, живой grim.
  Родитель **T284 закрыт** 2026-08-18 (`done/front/`) — в T303 только хвост,
  переоткрывать T284 нельзя.
- **T301** — `T301-composer-select-text-ellipsis.md`. P3, хвост T298:
  текст в Select-попапе всё ещё режется без эллипсиса.
