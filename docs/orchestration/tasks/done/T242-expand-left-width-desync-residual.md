# T242 — ширина панели десинкается с window.resize: интермиттентный residual, застревает на w=40 (левая И правая)

> **Расширено 2026-08-04.** Тот же симптом воспроизведён на **правой**
> панели тоже: `chronos-ipc select-tab:hyprland_binds` →
> лог `switched tab → opened at per-tab width tab="Hyprland binds"
> width=320.0`, но `hyprctl layers` показывает `side_panel_right`
> `w=40` — дважды подряд, с паузой между попытками. Значит баг не
> специфичен для `side_panel_left`/`expand_with_composer` — это
> системный паттерн рассинхрона `state.width` (внутренняя бухгалтерия)
> и реального `window.resize()` (layer-shell геометрия), который может
> задевать оба панеля. Проверить `render()`'s resize-guard в ОБОИХ
> `side_panel_left/mod.rs` и `side_panel_right/view.rs` — не чинить
> только левую половину.

**Роль:** FRONTEND/BACKEND (GPUI state).
**Источник:** `docs/orchestration/tasks/report-log/
T230-t210-t211-live-resmoke-and-select-tab-ipc-report.md` (находка §4) —
**подтверждено независимо архитектором дважды живьём** 2026-08-04, один
раз в полной тишине (без параллельных сессий), с `hyprctl layers`
геометрией до/после.

**Приоритет:** P1 — блокирует T233 (пересъёмка T223, левая панель) и
T241 (compose-and-send) косвенно, раз панель может не раскрыться.

## Наблюдение (воспроизведено дважды, независимо от T230-отчёта)

Иногда (не всегда) `chronos-ipc expand-left` после закрытой панели
оставляет `side_panel_left` на `x=0,w=40,h=1404` (rail-only) вместо
докнутой ширины — лог при этом показывает `IPC expand-left received` →
`side_panel_left: opened (pinned)` → `ACP client connected`, никакой
ошибки. **Иногда** (тот же код, тот же процесс, другой момент в истории
open/close циклов) команда честно раскрывает панель на докнутую ширину
— архитектор наблюдал оба исхода в этой сессии на одном и том же живом
процессе.

## Гипотеза корня (не подтверждена статическим чтением до конца —
проверить перед починкой)

`crates/app/src/side_panel_left/state.rs:142-153`:

```rust
pub fn ensure_chat_width(&mut self) {
    let need = self.sidebar_width() + super::sessions_list::SIDEBAR_HANDLE_WIDTH + 120.0;
    let target = self.remembered_chat_width.unwrap_or(Self::DEFAULT_CHAT_WIDTH)
        .max(need).min(self.max_width);
    if self.width < target {
        self.width = target;
    }
    self.remembered_chat_width = Some(self.width);
}
```

Гард `if self.width < target` — no-op, если `self.width` уже >= target
на момент вызова. `crates/app/src/side_panel_left/mod.rs`'s
`open_window()` (строки ~213 в актуальной версии, свериться) содержит
ранний return: если `SidePanelLeftState_.handle` уже `Some`, функция
просто ставит `pinned=true` и возвращает — **не создаёт новую сущность,
не сбрасывает `state.width`**. Если по какой-то причине `handle`
остаётся `Some` после незавершённого/некорректного предыдущего
close-цикла (или renderer ещё не подтвердил закрытие), следующий
`expand_with_composer` работает на **старой** сущности с уже
"застрявшим" в памяти `state.width`, а `render()`'s
`last_resized_width != panel_width` guard может совпасть по стечению
обстоятельств → `window.resize()` не вызывается → окно физически
остаётся 40px, хотя внутреннее состояние "думает" что оно широкое.

**Это гипотеза, не диагноз.** Первый шаг задачи — подтвердить или
опровергнуть её трейсингом (`tracing::debug!` в `ensure_chat_width` и
в `render()`'s resize-guard, залогировать `self.width`/`target`/
`last_resized_width`/`panel_width` на каждый вызов), не чинить вслепую.

## Что нужно

1. Добавить временный (или постоянный на `debug!` уровне) трейс в
   `ensure_chat_width()` и в resize-guard `render()` — воспроизвести
   баг живьём (рецепт ниже) с трейсингом включённым, прочитать лог,
   подтвердить гипотезу или найти реальную причину.
2. **Рецепт воспроизведения (подтверждён дважды):** открыть/закрыть
   левую панель через `chronos-ipc toggle-side-panel-left` несколько
   раз подряд (3-5 циклов), затем `chronos-ipc expand-left` — по опыту
   архитектора баг чаще проявляется после нескольких циклов, не на
   самом первом открытии свежего процесса.
3. Починить по факту найденной причины — если гипотеза верна, вероятно
   нужно либо: (a) `open_window`'s ранний return тоже гарантировать
   `ensure_chat_width()`/`dock_chat` применение (не только "уже
   открыт → просто pin"), либо (b) `close(cx)` синхроннее чистить
   `handle`/`state` перед тем как считается "закрыто".

## Верификация

```bash
cargo build --release -p chronos
cargo test --release -p chronos --lib -- side_panel_left
```

Live — обязателен, воспроизвести рецепт выше **10 раз подряд**
(не 1-2 — баг интермиттентный), убедиться что все 10 раз панель
раскрывается на докнутую ширину, не только иногда.

## Отчёт

`docs/orchestration/tasks/report/T242-expand-left-width-desync-report.md`
— приложить трейс-лог, подтверждающий найденную причину, и результат
10-кратного повтора после фикса.
