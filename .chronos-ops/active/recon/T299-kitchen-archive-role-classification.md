# T299 — RECON: разметить роль для 167 архивных тикетов кухни

**Роль: RECON** (read-only разведка по чужому дереву — тут дерево своё, но
задача та же: прочитать и классифицировать, не писать продуктовый код).

## Контекст

Кухня ChronOS переезжает из `docs/orchestration/tasks/` в
`.chronos-ops/` (`.chronos-ops/RULES.md` — канон структуры). Архив
(`done/`, `report-log/`, `rejected/`) дробится по ролям
(`front/back/qa/recon/design`) при переезде в
`.chronos-ops/{done,reports-log,reject}/<role>/`.

334 файла архитектор уже расклассифицировал и перенёс эвристикой по
ключевым словам в заголовке/имени файла (`git mv`, история сохранена).
**167 файлов эвристика не распознала** — остались на месте, в
`docs/orchestration/tasks/{done,report-log,rejected}/`. Полный список —
в конце этого файла.

## Задача

Для каждого из 167 файлов:
1. Открыть файл, прочитать `# TNNN — ...` заголовок и первые ~20 строк
   (обычно там "Цель"/"Статус"/зона файлов — этого достаточно, не нужно
   читать целиком).
2. Присвоить одну роль по таблице ниже (та же, что в `.chronos-ops/RULES.md`).
3. Записать результат построчно в отчёт как CSV:
   `роль|относительный_путь_от_docs/orchestration/tasks/`
   (тот же формат, что и в `git mv`-скрипте архитектора, чтобы просто
   склеить и прогнать).

## Роли — как решать спорные случаи

| Роль | Что сюда | Пример признаков |
|---|---|---|
| `front` | UI, взаимодействие, тема, панели, композер, бар, лаунчер, доки, OSD, tray | правки в `crates/app/src/{bar,dock,launcher,side_panel_*,notifications,osd,tray_menu}` |
| `back` | Сервисы, IPC, протоколы, скрипты, демоны, Luau/plugins | правки в `crates/services`, `crates/luau`, IPC-сокет, hyprctl, D-Bus |
| `qa` | Живые смоки, репро багов, регрессии, охота за блокерами | "смок", "живой прогон", "репро", "блокер снят/найден" в тексте |
| `recon` | Разведка по чужому дереву (Source/, reference/, апстрим), аудит API | "recon", "аудит", "fork drift", "investigat" |
| `design` | Макеты `docs/design/`, палитра, тема, визуальный спек | ссылки на `.dc.html`, палитру, мокапы |

Если тикет реально трогает **две роли** (например front-код + QA-смок в
одном файле) — бери роль **по факту кода**, не по факту проверки: если
менялся `crates/app`, это `front`, даже если полфайла — смок-лог. QA — это
роль для тикетов, где *единственная* работа — сбор улик, без продуктового
кода.

Если за 30 секунд не понятно — не гадай, помечай `?` и пиши одну строку
почему в отчёте отдельным списком "не смог классифицировать". Врать
уверенной ролью хуже, чем честно вернуть `?` — архитектор потом перепроверит
это дерево лично.

## Зона файлов

**Только читать.** Ничего не двигать, не коммитить, не трогать
`.chronos-ops/` и `docs/orchestration/` — перенос (`git mv`) делает
архитектор сам после приёмки отчёта, по тем же правилам, что и первые 334
(RULES.md: "В `reports-log/` и `done/` пишет только архитектор").

## Отчёт

`docs/orchestration/tasks/report/T299-kitchen-archive-role-classification-report.md`

Содержимое: CSV-блок (`роль|путь`) на все 167 строк + список `?`-случаев
с однострочной причиной. Ничего больше не нужно — ни диффов, ни сборки,
это чистая разметка.

## Список всех 167 файлов (относительно `docs/orchestration/tasks/`)

Полный список — эвристика архитектора не распознала ни один из них по
ключевым словам в заголовке/имени. Классифицировать каждый.

```
done/T109-agent-thread-canvas.md
done/T110-hot-reload-track-a-hotlibreloader.md
done/T111-hot-reload-track-b-subsecond.md
done/T122-dev-shell-scripts.md
done/T123-audio-volume-drag-coalesce.md
done/T128-elevated-surface-blur-tokens.md
done/T133-wallpaper-waytrogen-integration.md
done/T149-model-picker-search.md
done/T153-transcript-flow-segments.md
done/T165-mode-driven-composition.md
done/T169-rail-fourteen-tabs.md
done/T171-tab-preferred-width.md
done/T172-icons-blend-mode-broken.md
done/T174-width-follows-auto-tab-switch.md
done/T176-files-tab.md
done/T177-terminal-tab.md
done/T178-build-logs-tab.md
done/T179-preview-tab.md
done/T180-markdown-preview-goes-online.md
done/T185-scene-per-game-activate.md
done/T186-gamer-rail-tabs.md
done/T187-games-catalog-and-pins.md
done/T188-library-tab.md
done/T190-scene-gaming-profile-wire.md
done/T192-rail-product-cut.md
done/T193-hyprland-binds-readonly.md
done/T194b-editor-terminal-drawer.md
done/T194c-preview-edit-modes.md
done/T194-editor-from-preview.md
done/T195-agent-follow.md
done/T208-editor-status-and-softwrap.md
done/T213-editor-all-text-files.md
done/T218-per-tab-fixed-widths.md
done/T219-edit-mode-rail-reorder.md
done/T221-rail-icon-toggles-content.md
done/T222-files-view-edit-all-text.md
done/T223-capture-log.md
done/T227-font-canon-actually-applied.md
done/T233-t223-full-reshoot-clean-desktop.md
done/T236-hyprland-binds-human-categories.md
done/T237-editor-empty-state-polish.md
done/T242-expand-left-width-desync-residual.md
done/T245-monitor-selection-non-deterministic.md
done/T246-remove-fake-permission-mock.md
done/T247-compose-and-send-submit-and-width.md
done/T248-collapse-empty-mpris-card.md
done/T250-desktop-terminal-zsh-wizard.md
done/T251-hypr-binds-section-header-consistency.md
done/T253-reshoot-system-tab-after-permission-fix.md
done/T255-offline-trust-signal-impl.md
done/T256-fake-static-window-title-header.md
done/T259-desktop-terminal-edit-mode-ui.md
done/T260-context-menu-redraw.md
done/T260-wave2-context-menu-enter-accent.md
done/T267-unified-edge-separator.md
done/T268-desktop-frame-bottom-strip.md
done/T269-empty-state-helpers.md
done/T272-worktree-and-build-hygiene.md
done/T273-rail-wobble-during-shrink-resize.md
done/T276-standalone-right-rail-and-fixed-content-canvas.md
done/T278-left-workspace-fixed-surfaces-and-rail.md
done/T279-left-workspace-chat-sessions-project-tabs.md
done/T283-t280-sessions-scope-holes.md
done/T290-left-display-settings-tab.md
done/T291-E-gaming-knob-repaint.md
done/T291-system-tab-power-and-gaming.md
done/T292-workspace-mode-on-right-rail.md
done/T294-updates-tab-pacman-not-yay.md
done/T296-display-tab-belongs-on-right-rail.md
report-log/T036-remove-window-cause1-report.md
report-log/T047-workspace-dots-report.md
report-log/T051-cava-visualizer-report.md
report-log/T052-notification-history-report.md
report-log/T053-mpris-multiplayer-report.md
report-log/T058-monitor-consolidation-report.md
report-log/T069-light-c-scheme-report.md
report-log/T075-upgrade-feedback-report.md
report-log/T090-net-stats-report.md
report-log/T092-system-resources-power-report.md
report-log/T093-audio-stream-mute-report.md
report-log/T095-hover-peek-mpris-card-report.md
report-log/T096-spectrum-power-geometry-report.md
report-log/T099-udisks2-disks-report.md
report-log/T100-mpris-art-progress-report.md
report-log/T110-hot-reload-track-a-hotlibreloader-report.md
report-log/T111-hot-reload-track-b-subsecond-report.md
report-log/T122-dev-shell-scripts-report.md
report-log/T122-dev-shell-scripts-review.md
report-log/T123-audio-volume-drag-coalesce-report.md
report-log/T123-audio-volume-drag-coalesce-review.md
report-log/T124-ephemeral-toast-notifications-review.md
report-log/T128-elevated-surface-blur-tokens-report.md
report-log/T128-elevated-surface-blur-tokens-review.md
report-log/T133-wallpaper-waytrogen-integration-report.md
report-log/T133-wallpaper-waytrogen-integration-review.md
report-log/T149-model-picker-search-report.md
report-log/T150-thread-store-report.md
report-log/T153-transcript-flow-segments-report.md
report-log/T165-mode-driven-composition-report.md
report-log/T166-pult-display-consolidation-report.md
report-log/T169-rail-fourteen-tabs-report.md
report-log/T171-tab-preferred-width-report.md
report-log/T172-icons-blend-mode-broken-report.md
report-log/T174-width-follows-auto-tab-switch-report.md
report-log/T176-files-tab-report.md
report-log/T177-terminal-tab-report.md
report-log/T178-build-logs-tab-report.md
report-log/T179-preview-tab-report.md
report-log/T180-markdown-preview-no-network-report.md
report-log/T185-scene-per-game-activate-report.md
report-log/T186-gamer-rail-tabs-report.md
report-log/T187-games-catalog-and-pins-report.md
report-log/T190-scene-gaming-profile-wire-report.md
report-log/T192-rail-product-cut-report.md
report-log/T193-hyprland-binds-readonly-report.md
report-log/T194b-editor-terminal-drawer-report.md
report-log/T194c-preview-edit-modes-report.md
report-log/T194-editor-from-preview-report.md
report-log/T195-agent-follow-report.md
report-log/T213-editor-all-text-files-report.md
report-log/T214-resize-thrash-and-active-line-report.md
report-log/T218-per-tab-fixed-widths-report.md
report-log/T221-rail-icon-toggles-content-report.md
report-log/T222-files-view-edit-all-text-report.md
report-log/T222-files-view-edit-all-text-report-updated.md
report-log/T226-digits-vanish-while-typing-localization-report-3.md
report-log/T226-digits-vanish-while-typing-localization-report.md
report-log/T226-localization-attempt-4-plan.md
report-log/T227-font-canon-actually-applied-report-blocked.md
report-log/T227-font-canon-actually-applied-report.md
report-log/T231-pattern-spread-report.md
report-log/T233-reshoot-report.md
report-log/T236-hyprland-binds-categories-report.md
report-log/T237-editor-empty-state-report.md
report-log/T242-expand-left-width-desync-report.md
report-log/T245-monitor-selection-report.md
report-log/T246-remove-fake-permission-mock-report.md
report-log/T248-collapse-empty-mpris-card-report.md
report-log/T251-hypr-binds-section-header-consistency-report.md
report-log/T253-system-tab-reshoot-report.md
report-log/T253-T254-live-pc-use-report.md
report-log/T255-offline-trust-signal-impl-report.md
report-log/T256-fake-static-window-title-header-report.md
report-log/T257-desktop-terminal-registry-and-persistence-report.md
report-log/T260-context-menu-redraw-report.md
report-log/T260-wave2-context-menu-enter-accent-report.md
report-log/T267-unified-edge-separator-report.md
report-log/T269-empty-state-helpers-report.md
report-log/T272-worktree-and-build-hygiene-report.md
report-log/T276-standalone-right-rail-and-fixed-content-canvas-report.md
report-log/T278-left-workspace-fixed-surfaces-and-rail-report.md
report-log/T279-left-workspace-chat-sessions-project-tabs-report.md
report-log/T287-B-sessions-tab-kit-list-report.md
report-log/T287-C-chat-strip-zed-chrome-report.md
report-log/T291-E-gaming-knob-repaint-report.md
report-log/T291-system-tab-power-and-gaming-report.md
report-log/T292-workspace-mode-on-right-rail-report.md
report-log/T294-updates-tab-pacman-not-yay-report.md
report-log/T296-display-tab-belongs-on-right-rail-report.md
rejected/T189-scenes-tab-killed.md
rejected/T219-edit-mode-rail-reorder-report-rejected.md
rejected/T226-digits-vanish-while-typing-report.md
rejected/T226-infrastructure-report.md
rejected/T232-shell-polkit-agent.md
rejected/T239-right-rail-light-step-report.md
rejected/T279-left-workspace-chat-sessions-project-tabs-report.md
rejected/T900-techstack-research-misassigned-report.md
```
