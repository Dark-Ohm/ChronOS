# T197 — вернуть Terminal в default rail

**Статус:** PARKED — после T194 (или когда скажет архитектор). Не срочно.
**Роль:** FRONTEND.
**Причина:** T192 вырезал Terminal из `for_mode` как «IDE-хвост»; продукт
решил — **зря**, вернуть.

**Зона:** `tabs.rs` `for_mode(Developer)` (+ Gamer если нужно) — вставить
`PanelTab::Terminal` после Files/Editor (порядок: System, Files, Editor,
Terminal, HyprlandBinds, ACP, System settings — уточнить при старте).
Тесты `developer_rail_is_six_product_tabs` → seven/eight.

**НЕ:** менять terminal engine; только rail presence.

**Отчёт:** `report/T197-restore-terminal-tab-report.md`.
