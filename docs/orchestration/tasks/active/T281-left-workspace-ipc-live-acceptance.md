# T281 — левая workspace: IPC, focus, dock и live-приёмка Slice A

**Статус:** BLOCKED BY T280.
**Приоритет:** P1 — финальный integration/live gate Slice A.
**Роль:** integration + Hyprland live verification.
**Зависимость:** принятый T280.

## Канон

Выполнить только Tasks 7–8 из
`docs/superpowers/plans/2026-08-13-left-ai-workspace-slice-a.md`.
Это не разрешение начинать Slice B/C.

## Цель

Свести lifecycle, tabs, project sessions и существующий IPC в одну state
machine, собрать release binary и доказать архитектуру живьём на Hyprland.

## Публичный контракт

Сигнатуры и IPC payload не менять:

```rust
pub fn toggle(cx: &mut App);
pub fn expand_with_composer(cx: &mut App);
pub fn compose_and_send(text: String, cx: &mut App);
```

- `Super+A`: closed → обе fixed surfaces в visual rail-only; любое open →
  закрыть обе и сбросить dock.
- `expand-left`: обеспечить Chat, dock и focus composer.
- `compose-and-send`: обеспечить Chat+dock, focus и ровно одну отправку после
  readiness; сохранить T247.
- Session select открывает Chat и фокусирует composer.
- Active tab click схлопывает только visible content; fixed content surface
  остаётся. В dock active click — no-op.
- Dock сохраняет текущую width и rail резервирует текущую полную width.
  Tab switch в dock не применяет tab policy. Undock сохраняет width; следующий
  обычный tab switch применяет policy.
- Rail keyboard interactivity None; content OnDemand.

IPC `service.rs`/`messages.rs` — verify-only: при сохранённых публичных
сигнатурах они не должны меняться.

## Автоматические gates

Сначала написать pure reducer tests для всей таблицы transitions из плана,
включая rollback, dock, remembered widths, drag 960→40→обратно и focus flag.

```bash
cargo test -p chronos side_panel_left --lib --bins
cargo test -p chronos ipc --lib --bins
cargo test -p chronos-services --lib threads
cargo test -p chronos --lib --bins
cargo build --release -p chronos
rg -n 'window\.resize\(' crates/app/src/side_panel_left
```

Ожидается: всё зелёное, `rg` пустой, `Source/gpui/` не изменён.

## Live Hyprland gate

Запустить release через project scripts. `hyprctl layers` обязан показать:

- closed: ноль left workspace/hover surfaces;
- любое open, включая visual rail-only: ровно rail 40 px + content canvas
  920 px;
- canvas bounds 920 px неизменны на всём drag;
- content exclusive `-1`; exclusive owner только rail;
- overlay zone 40, dock zone — текущая полная width.

Проверить руками:

1. Super+A open/close и все rail buttons.
2. Chat drag 960→40→960, handle доступен на zero slice.
3. Нет wobble, wallpaper flash, gap, закрашенной полосы, пропавшего separator
   или спрятанного под rail handle.
4. Transparent void пропускает pointer.
5. Fixed/resizable tab policies и honest Slice B/C shells.
6. Dock/undock и dock-wins-collapse.
7. Project search/recent/branch/actions; bar pill отсутствует.
8. Project switch не показывает чужую session; restart восстанавливает
   последнюю валидную session каждого проекта.
9. Session select, `expand-left`, `compose-and-send` дают focus и один submit
   из closed/rail-only/content/docked states.

Не запускать `wf-recorder`: live evidence — `hyprctl layers`, grim и logs.

## Финальный критерий

Тесты не закрывают UX. Только `+` владельца означает принятие; `-` означает,
что T281 не выполнен. До `+` не обновлять архитектурные документы как будто
поведение доказано.

## Отчёт

Создать inbox-файл
`docs/orchestration/tasks/report/T281-left-ai-workspace-slice-a-report.md`.

Включить commits T278–T281, команды/exits, `hyprctl layers` measurements,
пути screenshots/logs, все непроверенные пункты и owner verdict. Исполнитель
не переносит отчёт в `report-log/`: это делает Архитектор после приёмки.

