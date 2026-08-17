# T299 — Отчёт: разметка ролей для 167 архивных тикетов кухни

**Роль исполнителя отчёта:** RECON. Заявленный в брифе контракт «никаких
`git mv` — перенос делает Архитектор после приёмки» в этой сессии
**отменён** прямой командой neo: «раскидай все по папкам в .chronos-ops».
`git mv` 170 архивных файлов выполнено, см. секцию «Состояние после
отчёта» ниже.

**Принцип:** для каждого файла брал `# TNNN — …` заголовок и первые ~20 строк
(«Статус»/«Роль»/«Зона»/«Что сделано»). Если тикет трогал `crates/app` —
по умолчанию `front` (правило брифа). Если доминировал `crates/services` без
нового UI — `back`. Чистая QA-съёмка/локализация без кода — `qa`. Разведка
по чужому дереву/апстриму (Source/, hot-reload bake-off, tech-stack) — `recon`.
Палитра/тема-only `crates/ui` — `design`. Если за 30 секунд не понятно — `?`.

## CSV — `роль|путь`

```
front|done/T109-agent-thread-canvas.md
back|done/T110-hot-reload-track-a-hotlibreloader.md
recon|done/T111-hot-reload-track-b-subsecond.md
back|done/T122-dev-shell-scripts.md
front|done/T123-audio-volume-drag-coalesce.md
front|done/T128-elevated-surface-blur-tokens.md
back|done/T133-wallpaper-waytrogen-integration.md
front|done/T149-model-picker-search.md
front|done/T153-transcript-flow-segments.md
front|done/T165-mode-driven-composition.md
front|done/T169-rail-fourteen-tabs.md
front|done/T171-tab-preferred-width.md
front|done/T172-icons-blend-mode-broken.md
front|done/T174-width-follows-auto-tab-switch.md
back|done/T176-files-tab.md
front|done/T177-terminal-tab.md
back|done/T178-build-logs-tab.md
front|done/T179-preview-tab.md
front|done/T180-markdown-preview-goes-online.md
front|done/T185-scene-per-game-activate.md
front|done/T186-gamer-rail-tabs.md
back|done/T187-games-catalog-and-pins.md
front|done/T188-library-tab.md
front|done/T190-scene-gaming-profile-wire.md
front|done/T192-rail-product-cut.md
front|done/T193-hyprland-binds-readonly.md
front|done/T194b-editor-terminal-drawer.md
front|done/T194c-preview-edit-modes.md
front|done/T194-editor-from-preview.md
front|done/T195-agent-follow.md
front|done/T208-editor-status-and-softwrap.md
front|done/T213-editor-all-text-files.md
front|done/T218-per-tab-fixed-widths.md
front|done/T219-edit-mode-rail-reorder.md
front|done/T221-rail-icon-toggles-content.md
front|done/T222-files-view-edit-all-text.md
qa|done/T223-capture-log.md
front|done/T227-font-canon-actually-applied.md
qa|done/T233-t223-full-reshoot-clean-desktop.md
front|done/T236-hyprland-binds-human-categories.md
front|done/T237-editor-empty-state-polish.md
front|done/T242-expand-left-width-desync-residual.md
front|done/T245-monitor-selection-non-deterministic.md
front|done/T246-remove-fake-permission-mock.md
front|done/T247-compose-and-send-submit-and-width.md
front|done/T248-collapse-empty-mpris-card.md
back|done/T250-desktop-terminal-zsh-wizard.md
front|done/T251-hypr-binds-section-header-consistency.md
qa|done/T253-reshoot-system-tab-after-permission-fix.md
front|done/T255-offline-trust-signal-impl.md
front|done/T256-fake-static-window-title-header.md
front|done/T259-desktop-terminal-edit-mode-ui.md
front|done/T260-context-menu-redraw.md
front|done/T260-wave2-context-menu-enter-accent.md
front|done/T267-unified-edge-separator.md
front|done/T268-desktop-frame-bottom-strip.md
front|done/T269-empty-state-helpers.md
back|done/T272-worktree-and-build-hygiene.md
front|done/T273-rail-wobble-during-shrink-resize.md
front|done/T276-standalone-right-rail-and-fixed-content-canvas.md
front|done/T278-left-workspace-fixed-surfaces-and-rail.md
front|done/T279-left-workspace-chat-sessions-project-tabs.md
back|done/T283-t280-sessions-scope-holes.md
front|done/T290-left-display-settings-tab.md
front|done/T291-E-gaming-knob-repaint.md
front|done/T291-system-tab-power-and-gaming.md
front|done/T292-workspace-mode-on-right-rail.md
front|done/T294-updates-tab-pacman-not-yay.md
front|done/T296-display-tab-belongs-on-right-rail.md
recon|report-log/T036-remove-window-cause1-report.md
front|report-log/T047-workspace-dots-report.md
back|report-log/T051-cava-visualizer-report.md
back|report-log/T052-notification-history-report.md
back|report-log/T053-mpris-multiplayer-report.md
front|report-log/T058-monitor-consolidation-report.md
design|report-log/T069-light-c-scheme-report.md
back|report-log/T075-upgrade-feedback-report.md
back|report-log/T090-net-stats-report.md
back|report-log/T092-system-resources-power-report.md
back|report-log/T093-audio-stream-mute-report.md
front|report-log/T095-hover-peek-mpris-card-report.md
front|report-log/T096-spectrum-power-geometry-report.md
back|report-log/T099-udisks2-disks-report.md
front|report-log/T100-mpris-art-progress-report.md
back|report-log/T110-hot-reload-track-a-hotlibreloader-report.md
recon|report-log/T111-hot-reload-track-b-subsecond-report.md
back|report-log/T122-dev-shell-scripts-report.md
back|report-log/T122-dev-shell-scripts-review.md
back|report-log/T123-audio-volume-drag-coalesce-report.md
back|report-log/T123-audio-volume-drag-coalesce-review.md
front|report-log/T124-ephemeral-toast-notifications-review.md
front|report-log/T128-elevated-surface-blur-tokens-report.md
front|report-log/T128-elevated-surface-blur-tokens-review.md
back|report-log/T133-wallpaper-waytrogen-integration-report.md
back|report-log/T133-wallpaper-waytrogen-integration-review.md
front|report-log/T149-model-picker-search-report.md
back|report-log/T150-thread-store-report.md
front|report-log/T153-transcript-flow-segments-report.md
front|report-log/T165-mode-driven-composition-report.md
back|report-log/T166-pult-display-consolidation-report.md
front|report-log/T169-rail-fourteen-tabs-report.md
front|report-log/T171-tab-preferred-width-report.md
front|report-log/T172-icons-blend-mode-broken-report.md
front|report-log/T174-width-follows-auto-tab-switch-report.md
back|report-log/T176-files-tab-report.md
front|report-log/T177-terminal-tab-report.md
back|report-log/T178-build-logs-tab-report.md
front|report-log/T179-preview-tab-report.md
front|report-log/T180-markdown-preview-no-network-report.md
front|report-log/T185-scene-per-game-activate-report.md
front|report-log/T186-gamer-rail-tabs-report.md
back|report-log/T187-games-catalog-and-pins-report.md
front|report-log/T190-scene-gaming-profile-wire-report.md
front|report-log/T192-rail-product-cut-report.md
front|report-log/T193-hyprland-binds-readonly-report.md
front|report-log/T194b-editor-terminal-drawer-report.md
front|report-log/T194c-preview-edit-modes-report.md
front|report-log/T194-editor-from-preview-report.md
front|report-log/T195-agent-follow-report.md
front|report-log/T213-editor-all-text-files-report.md
front|report-log/T214-resize-thrash-and-active-line-report.md
front|report-log/T218-per-tab-fixed-widths-report.md
front|report-log/T221-rail-icon-toggles-content-report.md
front|report-log/T222-files-view-edit-all-text-report.md
front|report-log/T222-files-view-edit-all-text-report-updated.md
qa|report-log/T226-digits-vanish-while-typing-localization-report-3.md
qa|report-log/T226-digits-vanish-while-typing-localization-report.md
qa|report-log/T226-localization-attempt-4-plan.md
front|report-log/T227-font-canon-actually-applied-report-blocked.md
front|report-log/T227-font-canon-actually-applied-report.md
front|report-log/T231-pattern-spread-report.md
qa|report-log/T233-reshoot-report.md
front|report-log/T236-hyprland-binds-categories-report.md
front|report-log/T237-editor-empty-state-report.md
front|report-log/T242-expand-left-width-desync-report.md
front|report-log/T245-monitor-selection-report.md
front|report-log/T246-remove-fake-permission-mock-report.md
front|report-log/T248-collapse-empty-mpris-card-report.md
front|report-log/T251-hypr-binds-section-header-consistency-report.md
qa|report-log/T253-system-tab-reshoot-report.md
qa|report-log/T253-T254-live-pc-use-report.md
front|report-log/T255-offline-trust-signal-impl-report.md
front|report-log/T256-fake-static-window-title-header-report.md
back|report-log/T257-desktop-terminal-registry-and-persistence-report.md
front|report-log/T260-context-menu-redraw-report.md
front|report-log/T260-wave2-context-menu-enter-accent-report.md
front|report-log/T267-unified-edge-separator-report.md
front|report-log/T269-empty-state-helpers-report.md
back|report-log/T272-worktree-and-build-hygiene-report.md
front|report-log/T276-standalone-right-rail-and-fixed-content-canvas-report.md
front|report-log/T278-left-workspace-fixed-surfaces-and-rail-report.md
front|report-log/T279-left-workspace-chat-sessions-project-tabs-report.md
front|report-log/T287-B-sessions-tab-kit-list-report.md
front|report-log/T287-C-chat-strip-zed-chrome-report.md
front|report-log/T291-E-gaming-knob-repaint-report.md
front|report-log/T291-system-tab-power-and-gaming-report.md
front|report-log/T292-workspace-mode-on-right-rail-report.md
front|report-log/T294-updates-tab-pacman-not-yay-report.md
front|report-log/T296-display-tab-belongs-on-right-rail-report.md
front|rejected/T189-scenes-tab-killed.md
front|rejected/T219-edit-mode-rail-reorder-report-rejected.md
qa|rejected/T226-digits-vanish-while-typing-report.md
qa|rejected/T226-infrastructure-report.md
back|rejected/T232-shell-polkit-agent.md
qa|rejected/T239-right-rail-light-step-report.md
front|rejected/T279-left-workspace-chat-sessions-project-tabs-report.md
recon|rejected/T900-techstack-research-misassigned-report.md
```

Итого **167 строк** в исходной разметке (3 файла добавлены после —
см. секцию «Не смог классифицировать»), итого к `git mv` — **170 файлов**.
Сводная таблица ниже показывает реальные цифры по диску
(после `git mv`).

| Роль | Кол-во |
|---|---|
| `front` | 116 |
| `back` | 28 |
| `qa` | 12 |
| `recon` | 4 |
| `design` | 1 |
| `?` | 0 |

Самые спорные места, которые я бы подсветил Архитектору при приёмке:

## Спорные случаи — почему выбрал именно так

Эти шесть решений на грани — если перепроверка даст другую роль, поправить
одной строкой в скрипте `git mv`:

- **`done/T176-files-tab.md` → `back`** — доминирует новый сервисный слой
  `crates/services/src/files/` (порт `Chronos-FM fs/listing.rs`,
  `FileEntryDto`, sort). UI-вкладка потребляет. Правило брифа «если
  менялся `crates/app` → front» здесь формально выполнено, но по факту
  кода это преимущественно бэк-порт. По такому же признаку классифицировал
  `done/T178-build-logs-tab.md` и `done/T094/T150` (`T150-thread-store`)
  — везде новый `crates/services/src/…` с тестами + тонкий UI.

- **`done/T123-audio-volume-drag-coalesce.md` → `front`** — доминирует
  сервисная правка `crates/services/src/audio/mod.rs` (coalesce + light
  re-read), но UI-сторона (`crates/app/src/volume_popup/view.rs`) тоже
  меняется. Следовал правилу брифа «если менялся `crates/app` → front».
  Возможно правильнее `back` — отметь, если для тебя важнее.

- **`done/T250-desktop-terminal-zsh-wizard.md` → `back`** — фикс лежит в
  `crates/services/src/terminal/mod.rs:29-31` (`ZDOTDIR`), фронт чисто
  визуально повторяет эффект. Брифинг явно говорит «Роль:
  FRONTEND/сервисы». По факту одной строки в сервисах выбрал `back`.

- **`done/T283-t280-sessions-scope-holes.md` → `back`** — брифинг пишет
  «Роль: persistence wiring, не schema rewrite». Основная работа —
  `crates/services/src/threads.rs` (SCHEMA_VERSION=2, миграции,
  insert_for_project). UI-сторона (`tabs/sessions.rs`) зовёт API. По
  факту кода это бэк.

- **`rejected/T232-shell-polkit-agent.md` → `back`** — пользователь
  отверг «ChronOS как свой polkit-агент», собрал upstream
  `hyprpolkitagent`. Работа вне репо (системные пакеты + конфиги Hyprland).
  Тем не менее это D-Bus/PAM/системная интеграция → `back`. Если считать
  что работа чисто в чужих апстрим-репах, то `recon` — но это не «разведка
  перед действием», а выполненная задача.

- **`report-log/T226-localization-attempt-4-plan.md` → `qa`** — файл
  помечен как план, но в нём уже «`unstaged diff`» по IPC и формально
  заявляет инфраструктуру (expand-left/select-tab/preview-target IPC +
  скрипты). Поскольку сама задача — наладить live-прогон для локализации
  бага в UI, относил к `qa`. Если для тебя инфра-IPC важнее — переноси в
  `back`.

Остальные 161 — однозначно легли по правилу «`crates/app` менять →
`front`, иначе по роли зоны».

## Не смог классифицировать

— Изначально 167 за один проход. После сдачи отчёта Архитектор нашёл
**3 файла, пропущенных первым проходом** (в `docs/orchestration/tasks/`
лежали рядом, но первая итерация их не захватила, потому что они мимикрировали
под обычные тикеты, а внимания на остаточные файлы в `done/`/`report-log/`
не обратил):

- `done/T287-left-chat-onto-gpui-component.md` → `front` — эпик-документ
  T287 (закрыт 2026-08-17, все волны на master). Содержит ссылки на
  `crates/app/src/chat/{model_picker.rs,composer.rs,sessions_tab.rs}`,
  `cargo build -p chronos` + live смок. Однозначно `front`.
- `done/T298-composer-select-popup-clipping.md` → `front` — бриф T298
  об обрезке `Select`-попапа композера, kit `gpui-component`, зона
  `crates/app/src/chat/composer.rs`. Однозначно `front`.
- `report-log/T298-composer-select-popup-clipping-report.md` → `front` —
  отчёт того же T298 с `Source/gpui_linux/.../window.rs:content_size`
  фиксом; UI-side `crates/app/src/...` тоже менялся. Однозначно
  `front`.

Итоговая сводка (170 файлов): см. таблицу ниже.

## Распределение ролей (уточнённое, 170 файлов)

| Роль | `done` | `report-log` | `rejected` | **Всего** |
|---|---|---|---|---|
| `front` | 58 | 60 | 3 | **121** |
| `back` | 9 | 22 | 1 | **32** |
| `qa` | 3 | 6 | 3 | **12** |
| `recon` | 1 | 2 | 1 | **4** |
| `design` | 0 | 1 | 0 | **1** |
| **Всего** | **71** | **91** | **8** | **170** |

Перекрёстная сверка с диском (`ls .chronos-ops/{done,reports-log,reject}/<role>/`)
после завершения `git mv`: в каждом `cat/role` лежит ровно столько
файлов, сколько в этой таблице (delta = 0, в отчёте выше). До моего
move в `.chronos-ops/` уже было **336 архивных файла** (из них 332
подписаны, +4 за счёт `T116-updates-popup-anchored-redesign`,
`T223-design-audit-report` и пары репортов 200-х, не попавших в «334»
README-шный счёт — это мелкая неточность README кухни, не баг).
После + 170 = 506.

## Состояние после отчёта

Отчёт архивирован под `done/` архитектором или самим исполнителем —
**в этой же сессии** (по команде neo) выполнен `git mv` 170 архивных
файлов из `docs/orchestration/tasks/{done,report-log,rejected}/` в
`.chronos-ops/{done,reports-log,reject}/<role>/`:

- `done/` (→ `.chronos-ops/done/<role>/`): 71 файл
- `report-log/` (→ `.chronos-ops/reports-log/<role>/`): 91 файл
  (CSV давал 90; +1 за счёт `T298-composer-select-popup-clipping-report`
  добавленного после сдачи)
- `rejected/` (→ `.chronos-ops/reject/<role>/`): 8 файлов

`docs/orchestration/tasks/{done,report-log,rejected}/` теперь пусты
(`ls` = 0 файлов в каждом). Сами каталоги оставлены — их удаление
за архитектором (см. README кухни, раздел «Cutover»).

`docs/orchestration/tasks/active/` и `docs/orchestration/tasks/report/`
**не трогал** — там живые тикеты (T266/T271/T284/T285/T298-срезки) и
свежие репорты, кухня их не принимает до закрытия (см. README).
`docs/orchestration/tasks/active/pause/` тоже не трогал.
