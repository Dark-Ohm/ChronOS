<!-- T064 — migrated 2026-07-22 from orchestration/report-log/grok-report-animation-vendor.md — see orchestration/tasks/MIGRATION.md -->

# Session: вендор gpui-animation в Source/ — 2026-07-21

## Сделано (факт, не намерение)

- Новая директория `Source/gpui-animation/` (сосед `Source/gpui/`):
  - весь `src/` с **тремя fork-дельтами уже применёнными** (из recon-клона
    `/home/neo/scratch/gpui-animation-recon`, upstream `@ad77bea` / 0.2.60)
  - `Cargo.toml` — `gpui = { path = "../gpui" }`, header-коммент про vendor+PATCHES
  - `PATCHES.md` — документация трёх дельт (что/где/почему)
  - `LICENSE-MIT` + `LICENSE-APACHE` — в апстриме физических LICENSE* не
    было (`git ls-tree` только README/src/examples); созданы стандартные
    тексты под `license = "MIT OR Apache-2.0"` + автор из их Cargo.toml
  - `README.md` апстрима (атрибуция/документация API)
- **НЕ тронуто:** `Source/Cargo.toml`, `Source/NOTICE`, ChronOS, чужие
  untracked. Коммит **не** делал (по брифу — проводка Архитектора).

### Таблица файлов

| Файл | Статус | cargo check |
|---|---|---|
| `src/lib.rs` + `animation.rs` + `transform.rs` + `transition/**` | скопирован as-is | ✓ (вместе с крейтом) |
| `src/transition.rs` | пропатчен (дельта 1, 2 места) | ✓ |
| `src/interpolate.rs` | пропатчен (дельта 2 + 3) | ✓ |
| `Cargo.toml` | новый, path=`../gpui` | ✓ |
| `PATCHES.md` | новый | n/a |
| `LICENSE-MIT` / `LICENSE-APACHE` | созданы (в апстриме отсутствовали) | n/a |
| `README.md` | скопирован | n/a |
| examples/ | **не** вендорил (не в scope брифа) | — |

### Три дельты — на месте

1. **AsyncApp::update non-fallible** — `src/transition.rs:207` и `:243`:
   `cx.update(|cx| cx.refresh_windows());` без `.ok()` (коммент `// fork:`)
2. **BoxShadow.inset** — `src/interpolate.rs:390`:
   `inset: if t < 0.5 { self.inset } else { other.inset }`
3. **Style.text #[refineable]** — `src/interpolate.rs:448-452`:
   прямой `self.text.fast_interpolate(&other.text, t, &mut out.text)`

Четвёртой ошибки компиляции **не** всплыло.

## Расхождения со спекой/планом

- Временный пустой `[workspace]` в `Cargo.toml` — **нужен был** для
  `cargo check` (parent `Source/Cargo.toml` иначе: *current package believes
  it's in a workspace when it's not*). Прогнал check с `[workspace]`,
  **убрал** секцию из финального дерева (как бриф). Architect должен
  добавить `gpui-animation` в `workspace.members` **или** `exclude` при
  проводке — иначе standalone check снова упрётся.
- `Cargo.lock` / `target/` от standalone check — удалены, в vendor tree
  не оставлял.
- examples из recon (`mini_hover.rs`) не переносил — бриф: src + Cargo.toml
  + licenses; примеры не требовались.

## Не реализовано из acceptance criteria

- Workspace membership / NOTICE / `crates/app` dependency — **явно зона
  Архитектора**, не делал.
- Коммит — не делал (по брифу).

## Проверено фактом, не на словах

```
# 1) Isolation check (with temporary empty [workspace] — then removed)
cd /home/neo/projects/chronos-ecosystem/Source/gpui-animation
# temporarily: [workspace] in Cargo.toml
cargo check
# → Checking gpui-animation v0.2.60 (...)
# → Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.12s
# EXIT: 0
# (gpui lib warnings only — 63 missing-docs; no errors in gpui-animation)

# 2) After removing [workspace]
rg -n '\[workspace\]' Cargo.toml
# → no workspace section — good

# 3) Deltas present
rg -n 'fork:|inset: if t' src/
# transition.rs:207, :243 — update without .ok()
# interpolate.rs:389-390 — inset
# interpolate.rs:448 — text refineable

# 4) Source git status (only our new dir)
git -C /home/neo/projects/chronos-ecosystem/Source status --short
# → ?? gpui-animation/
# (no M on Cargo.toml / NOTICE / other members)

# 5) Without [workspace], bare check fails as expected (workspace absorb):
# error: current package believes it's in a workspace when it's not
# (documented; Architect wires members)
```

## Новые риски / известные баги

- **Severity process:** пока `gpui-animation` не в `workspace.members` и
  не в `exclude`, `cargo check` из этой директории без локального
  `[workspace]` падает — ожидаемо.
- **Severity low:** LICENSE* созданы нами (апстрим не шипил файлы) —
  copyright years 2025-2026 + author из их Cargo.toml; если у
  chi11321 другие NOTICE-требования — Architect может уточнить при
  записи в `Source/NOTICE`.
- **Severity info:** bump upstream потребует re-apply PATCHES.md; без
  файла правки теряются молча — файл именно для этого.

## Статус ARCHITECTURE.md / DECISIONS.log

- Не обновлялись (вендор-дерево + отчёт; канон/NOTICE — Architect).

## Source cleanliness

```
git -C Source status --short
?? gpui-animation/
```

Shared-файлы (`Cargo.toml`, `NOTICE`, другие крейты) — **не изменялись**.
