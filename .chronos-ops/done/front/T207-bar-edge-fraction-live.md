# T207 — Bar: live edge top↔bottom + fraction width (fork if needed)

**Статус:** active. **T202 done** — can start when free (after T206 UX preferred).
**Роль:** FRONTEND (+ fork). **Модель: Sonnet / GLM.**
**План:** live-customization; T200 residual; T198 RECON.
**Правила:** `RULES.md`.

Optional RECON-only first if parallel with T206.

**Зона:**
- `crates/app/src/bar/mod.rs` apply_appearance / window_options
- optional fork: `../Source/gpui` + `gpui_linux` `set_anchor` / `set_margin`
- panel gaps if edge bottom (OSD collision note)

**Цель:** `edge=bottom` and `width=fraction:0.8` apply **without** shell restart
when possible; else honest cold-path + skill T203 errata already noted.

**Must:** floating⇒!exclusive already; set_input_region for pill if fraction.

**Отчёт:** `report/T207-bar-edge-fraction-live-report.md`.

Коммит: `bar : live edge/fraction apply (T207)` (+ separate gpui commit).
