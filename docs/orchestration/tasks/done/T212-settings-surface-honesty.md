# T212 — Settings surface honesty (ACP/Editor/theme partial)

**Статус:** active **P1** (after T211 P0). Источник: T209 S6/S7/E9/B2 notes.  
**Роль:** FRONTEND.  
**Отчёт T209:** `report-log/T209-live-smoke-residuals-report.md`

## Must

1. **ACP Reload visible + works** — button exists in `acp_settings.rs`  
   (`#acp-reload`); live smoke claimed "missing" (likely clipped at narrow  
   width). Ensure Actions/Reload reachable (scroll/min height).  
   After edit `agents.toml` + Reload → list updates without shell restart  
   (`known_agents()` already re-reads file — prove live).
2. **Missing file Editor** — open non-existent path → honest empty/error  
   (path + reason), not blank surface (S6 blank grim).
3. **agents.toml edit path** — either allow Edit for `.toml` when opened  
   from ACP tab, or stop advertising "Edit agents.toml" as in-app if View-only.
4. **Light theme buffer/rail (E9)** — editor Input + right rail follow scheme  
   (or document intentional exception — prefer fix).
5. **Optional P2:** `margin.x` on full-width floating bar honesty  
   (no-op or apply inset).

## Зона

`tab/acp_settings.rs`, `tab/preview.rs` (missing file state),  
theme/surfaces for editor+rail light, maybe `bar/mod.rs` margin note.

**Не:** full inline ACP CRUD forms (still deferred).

**Отчёт:** `report/T212-settings-surface-honesty-report.md`  
Коммит: `settings : honesty reload missing-file light rail (T212)`.
