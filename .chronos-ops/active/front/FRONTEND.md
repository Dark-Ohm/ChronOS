# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

## Очередь

1. **T337** — `T337-bar-height-min-readable.md`. P1. `HEIGHT_MIN` — высота,
   на которой бар читается. `appearance.rs` + слайдер `bar_settings.rs`.
   **T331 HOLD**, пока не будет нового пола.
2. **T332** — `T332-anchored-popup-click-away.md`. P1. Click-away Sound/Calendar.
   **Разблокирован:** T329 принят 2026-08-21, плита календаря уже в дереве.
3. **T333** — `T333-active-project-reselect-noop.md`. P1. Reselect проекта
   не чистит session. Параллелен T331 (`side_panel_left/`).
4. **T334** — `T334-updates-upgrade-all-honest-label.md`. P1. Upgrade all
   не обещает AUR. `updates.rs` only; yay apply не делать (T294).
5. **T335** — `T335-acp-settings-open-fits-320.md`. P2. Open agents.toml
   на 320 px. `acp_settings.rs`. Reload `flex_none` не снимать (T212).
6. **T336** — `T336-resolve-tab-before-ensure.md`. P2. IPC вне mode set
   не спавнит terminal. `view.rs` `on_tab_select`.
7. **T339** — `T339-wallpaper-next-empty-feedback.md`. P1. Next не молчит.
8. **T340** — `T340-scheme-selected-chip-contrast.md`. P1. Чипы ≥ 4.5:1.
9. **T341** — `T341-blur-module-install.md`. P1. Install 45-surface-effects.
   **Приоритет вырос:** без блюра вся семья попапов — прозрачное стекло,
   а не матовое (видно на кадре T329/02).
10. **T342** — `T342-surface-alpha-chrome.md`. P1. Alpha на раму/Start.
    Не `calendar_popup/` (T329 принят).
11. **T343** — `T343-bar-border-survives-height-hot-reload.md`. P2. `bar/mod.rs`.
    Параллелен T337. T313-хвосты в TBD не трогать.

**Живой смок мышью — единственный рабочий рецепт (2026-08-21, T329):**
`ydotool mousemove --absolute` на этой машине НЕ попадает в заказанные
координаты (акселерация: `-x 2404 -y 10` → фактически `3475, 39`, т.е.
второй монитор). Warping — только через Lua-Hyprland 0.56.2, и позицию
ОБЯЗАТЕЛЬНО сверять:
```bash
export YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket
hyprctl dispatch 'hl.dsp.cursor.move({x=2404, y=10})'   # старый `movecursor 2404 10` — Lua-ошибка
hyprctl cursorpos                                        # СВЕРИТЬ до клика
ydotool click 0xC0
```
Якорный попап (Sound/Calendar) — xdg_popup-ребёнок бара: его НЕТ ни в
`hyprctl layers`, ни в `hyprctl clients`. Открылся или нет — решает
только `grim`.

**Страница System settings шире (T313, 2026-08-20):**
`PanelTab::EditorSettings.preferred_content_width()` = **800** (было
410). Держать ≥ 720 (`GRID_BREAKPOINT`, `tab/ui.rs:25`) — иначе все
двухколоночные сетки T231 на этой странице молча схлопываются в одну
колонку, а респонсив-код становится мёртвым.

**Имена режимов рамы сменились (T312, 2026-08-20):** в коде
`FrameStyle::Normal` / `FrameStyle::Wrapped`, в конфиге
`style = "normal" | "wrapped"`. Старые `hide` / `wrap` живут как алиасы
разбора — не выпиливать, иначе конфиги пользователей схлопнутся в
дефолты (T268). Нижней планки как отдельной сущности больше нет:
`[bottom_strip]` парсится, но не читается, низ в `wrapped` — это
`wrap.bottom`.

**Схемы:** `builtin_schemes()` теперь 4 — Default, Light, Solarized Dark,
Mocha Mousse (тёмная, Pantone 17-1230). Любая новая схема обязана
пройти WCAG-ворота T317 автоматически: тест итерирует
`builtin_schemes()` и требует `text.muted` ≥ 4.5:1 на `bg.primary`.
Применение схемы — только через `theme_config::select` (один apply-путь).

**Кольцо апертуры (`aperture_ring`, `frame.rs`) переделке не подлежит** —
форма принята владельцем 2026-08-19 после трёх неверных заходов. Прежде
чем трогать скругления оболочки, прочитать разбор в
`reports-log/front/T318-rail-as-frame-edge-implementation-report.md`:
там расписано, почему `rounded_*` на рельсе и угловые заплатки дают
вывернутую кривизну, а работает только кольцо с бордером.

**Закрыто 2026-08-18/19 (детали — `MIGRATION.md`):** T301, T302, T303,
T305, T307, T308, T311 (единая плита), T314 (живая эксклюзивная зона —
рельс стал кромкой кадра), T318 (оболочка обводит окно), T317 (WCAG muted), T319 (геометрия по краям), T321 (живая геометрия + эррата на нулевой размер), T322 (панели без
пересоздания), T316 (закрыт как сделанный T318 — кода нет), T312 (режимы
normal/wrapped, планка снята), T313 (пикер схем + Mocha Mousse,
вкладка 800), T320 (вкладки
вернулись в панель, control-center снят).
