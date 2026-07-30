# T133 — Wallpaper × waytrogen: first-class GUI integration

**Статус: OPEN, не назначен.**  
**Канон:** `DECISIONS.log` 2026-07-25 — integrate waytrogen, do not rewrite it.  
**Исправление брифа:** предыдущая версия сводила waytrogen к «spawn + две кнопки».  
Неверно. **GUI waytrogen — это и есть gallery.** Мы её **подключаем**, не вырезаем
и не заменяем своим picker'ом.

## Разделение ролей (зафиксировать)

| Кто | Роль | UI |
|---|---|---|
| **Waytrogen** | Gallery / browser / multi-backend setter (их продукт) | **Их полный GUI** — recursive library, GIF/video, transitions, JSON state… |
| **ChronOS** | Shell: hotpath next/set, IPC, optional entry points, companion install | **Не** свой wallpaper browser. Тонкая интеграция + engine `awww` |

**«Не переписываем waytrogen» ≠ «вырезаем GUI».**  
Значит: **не пишем GPUI-галерею вместо них.** Их окно — главный Browse.

Уже в дереве (engine, не gallery):
- `crates/services/src/wallpaper/` — `awww` (CLI-паттерны из waytrogen Unlicense)
- `wallpaper_ctl` — folder cycle `~/Pictures/Wallpapers`
- IPC `wallpaper-next` / `wallpaper-set:`

## Цель T133

Сделать waytrogen **видимым first-class companion** в шелле:

1. **Browse / Gallery** всегда ведёт в **waytrogen GUI** (их app, full UX).  
2. **Next** (и set) — быстрый hotpath шелла без открытия GUI.  
3. После закрытия waytrogen — **resync** state шелла с `awww query` (их GUI ставит обои «мимо» нашего Mutable — это баг-продукта, чинить).  
4. Нет waytrogen → **не** подсовывать самодельный picker; честный empty/CTA «установить companion» + ссылка.  
5. Docs: «engine ours / gallery waytrogen — install together?»

## Product copy

> **Wallpapers:** ChronOS drives the desktop via `awww` (next / set / IPC).  
> **Gallery UI:** [waytrogen](https://github.com/nikolaizombie1/waytrogen) — not ChronOS; we integrate it.  
> Install both for the full experience.

В UI кнопка Browse: **«Waytrogen»** или **«Open gallery (waytrogen)»** — имя **их** продукта, не «Browse…» без бренда.

## Задачи

### Task 1 — Detect + launch waytrogen as the gallery app

`wallpaper_ctl` (or `wallpaper_companion.rs`):

```text
waytrogen_available() -> bool     // PATH / optional CHRONOS_WAYTROGEN
open_waytrogen_gallery() -> Result<(), GalleryError>
  // Missing | SpawnFailed(io)
```

- Spawn **их** GUI: `waytrogen` (default no-args = full app per their CLI).  
- Respect CLI if useful (from `reference/waytrogen-main` `cli_parser.rs`):  
  e.g. only if safe/documented — do not invent flags. Default GUI open is enough.  
- Single-instance: if already running, prefer focus/re-exec as their app allows  
  (if unknown — second spawn ok, log).  
- **Never** open a ChronOS-built image grid as fallback gallery.

### Task 2 — Resync shell state after gallery use

When waytrogen changes wallpaper via `awww`, our `WallpaperState` goes stale  
(documented limitation). Integration must fix the **UX hole**:

- On `open_waytrogen_gallery`: spawn, then either  
  - **A (preferred):** `tokio` wait on child exit → `query`/`Refresh` wallpaper service → update Mutable; or  
  - **B:** poll `awww query` while child alive (debounce 500ms) + final query on exit.  
- Wire refresh into existing subscriber (add `refresh()` / re-query if missing).  
- Unit-testable pure parse already exists (`parse_query`); use it.

### Task 3 — IPC (shell ↔ world, including optional waytrogen scripts)

| Payload | Action |
|---|---|
| `wallpaper-next` | keep |
| `wallpaper-set:path` | keep |
| `wallpaper-gallery` | **open waytrogen GUI** (+ resync path Task 2) |
| `wallpaper-refresh` | **new** — force re-query awww into service (for external scripts) |

Document that waytrogen **external script** (their feature) can call  
`wallpaper-refresh` or `wallpaper-set:` so both apps stay aligned —  
**optional** snippet in docs, not required to patch waytrogen upstream.

### Task 4 — Shell UI: entry that sells the companion

Not «two tiny buttons and forget». Surface that makes waytrogen **the** gallery:

**Primary (required):** dedicated block on right panel System tab (or new  
«Desktop» card):

- Current wallpaper path/thumb if cheap (optional)  
- **Next** — shell hotpath  
- **Open waytrogen** — large/primary action when installed  
- **Install hint** when missing: short text + `yay -S waytrogen` / AUR link  
  (copy only; do not run package manager without user)

**Secondary (required):** IPC + hypr binds so gallery is one key away.

**Bar widget:** optional if panel done; not a mini-gallery.

### Task 5 — Docs / companion story

`docs/wallpaper.md` + README pointer:

1. Architecture diagram: ChronOS engine ‖ waytrogen GUI ‖ awww daemon  
2. What we own vs what they own  
3. Install companion (Arch AUR `waytrogen`)  
4. IPC table  
5. Hypr binds snippet (gallery = waytrogen, next = shell)  
6. External-script bridge idea for waytrogen config  

### Task 6 — Live smoke

```text
[ ] waytrogen installed → Open waytrogen shows THEIR full GUI (not a stub)
[ ] set wallpaper in waytrogen → after close (or poll) ChronOS state matches awww
[ ] wallpaper-next still cycles ~/Pictures/Wallpapers without opening GUI
[ ] waytrogen removed from PATH → UI shows install CTA; no fake ChronOS gallery
[ ] grim: panel card + waytrogen window (proof we launch their app)
```

Отчёт:  
`docs/orchestration/tasks/report/T133-wallpaper-waytrogen-integration-report.md`

## Зона файлов

**Писать:**
- `crates/app/src/wallpaper_ctl.rs` (+ companion helpers)
- `crates/services/src/wallpaper/` — only if need public `refresh`/`query` path
- `crates/app/src/ipc/**`
- `side_panel_right` System desktop/wallpaper card (or agreed surface)
- `docs/wallpaper.md`, README link

**Читать only:**
- `reference/waytrogen-main` (CLI, external script)  
- NOTICE attribution  

## Что НЕ делать

- ❌ GPUI image grid / «наш waytrogen»  
- ❌ Vendoring waytrogen sources into crates/  
- ❌ Hard package dependency (shell runs without it)  
- ❌ Скрывать имя waytrogen за нейтральным «Browse»  
- ❌ Считать accept «просто spawn без resync и без UI-карточки»

## Accept / Reject

**Accept:**  
- Gallery action opens **waytrogen’s full GUI**  
- Resync after gallery use  
- Next hotpath intact  
- Missing companion = CTA, not silent fail / not fake UI  
- Docs name waytrogen as partner gallery  
- Live grim of their window launched from shell  

**Reject:**  
- Only `Command::new("waytrogen").spawn()` + two unlabeled buttons  
- Homegrown gallery «пока waytrogen нет»  
- No state resync  
- Docs that say we replaced their UI  

## Commit style

`wallpaper : first-class waytrogen gallery integration (T133)`

## Report path

`docs/orchestration/tasks/report/T133-wallpaper-waytrogen-integration-report.md`
