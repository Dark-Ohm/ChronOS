<!-- T063 — migrated 2026-07-22 from orchestration/report-log/grok-report-animation.md — see orchestration/tasks/MIGRATION.md -->

# Session: разведка `gpui-animation` — компилируется ли против нашего форка — 2026-07-20

**Задание:** Grok №18 (бриф `orchestration/agents/GROK.md`).
**Ответ одной строкой: НЕТ out-of-the-box (8 ошибок, 3 API-дельты). ДА после 3 хирургических правок их кода (~15 строк) + boot через `gpui_platform::application()` для примеров. Живой hover-переход плавный, mid-кадр подтверждён grim.**

Клон: `/home/neo/scratch/gpui-animation-recon` (НЕ ChronOS, НЕ Source; upstream `ad77bea` / crates.io `0.2.60`, `--depth 1` с `https://github.com/chi11321/gpui-animation`). Тулчейн: `rustc/cargo 1.97.1`.

## Сделано (факт, не намерение)

- Клон в `/home/neo/scratch/gpui-animation-recon`, `Cargo.toml`: `gpui = { path = ".../Source/gpui" }` (было `"0.2.2"`).
- `cargo check -p gpui-animation` **без правок исходников** → 8 ошибок, `EXIT≠0` (см. таблицу).
- Диагностический патч **только в клоне** (3 дельты API) → lib `Finished`, `EXIT:0`.
- Их штатные examples (`hover_color`/`hover_position`/`button`/`translate`) падают на `Application::new()` — метода нет в нашем форке (есть `with_platform` / `gpui_platform::application()`). Это boot-API, не animation-API.
- Мини-пример `examples/mini_hover.rs` (boot через `gpui_platform::application()`, `.with_transition` + `.transition_on_hover(300ms, Linear, …)`) → `cargo check/build --example mini_hover` зелёные.
- **Живой смок:** процесс окно (class/title пустые, pid совпал), grim idle/mid/hover/away:
  - idle: доминирует `(30,30,46)` = `#1e1e2e` (наш bg) + box `(49,50,68)` = `#313244`;
  - mid (~150ms): появляется `(58,87,194)` — промежуточный синий, не snap;
  - hover: `(137,180,250)` = `#89b4fa` (целевой accent);
  - away: pixel_diff с idle = **0**.
  Скриншоты: `/tmp/gpui-anim-smoke/{idle,mid,hover,away}4.png`.
- `cargo tree -p gpui-animation`: ровно один `gpui v0.2.2 (…/Source/gpui)`. `dashmap`/`parking_lot`/`smol` — без gpui.
- `cx.spawn(Self::animation_tick).detach()` (`src/transition.rs:92`) **типизируется** против нашего `App::spawn` / `AsyncApp` без правок. Условие эскалации брифа (несовместимость spawn) **НЕ наступило**.

### Таблица результатов

| Цель | Собрался | Примечание |
|---|---|---|
| `gpui-animation` lib, только path-патч | ❌ нет | 8 ошибок, 3 API-дельты (ниже) |
| `gpui-animation` lib + 3 хир. патча | ✅ да | ~0.4s инкрементально после gpui |
| example `hover_color` (их) | ❌ нет | `Application::new` отсутствует |
| example `hover_position` (их) | ❌ нет | то же |
| example `button` (их) | ❌ нет | то же |
| example `translate` (их) | ❌ нет | то же |
| example `mini_hover` (наш recon) | ✅ typecheck + build | boot `gpui_platform` |
| runtime hover (mini_hover) | ✅ плавно | mid-кадр + rollback, grim |

## Три API-дельты (форк vs crates.io `gpui 0.2.2`)

Все три — расхождение **нашего форка** с crates.io 0.2.2, на который писали `gpui-animation 0.2.60`. Не баг spawn, не `smol` как третий рантайм.

### 1. `AsyncApp::update` → `R`, не `Result<R>`

- crates.io: `async_context.rs:142` → `pub fn update<R>(…) -> Result<R>` (weak upgrade, fallible).
- наш форк: `Source/gpui/src/app/async_context.rs:163` → `pub fn update<R>(…) -> R` (через `self.app()`, non-fallible).
- их код: `cx.update(|cx| cx.refresh_windows()).ok();` (`transition.rs:207`, `:243`) — `.ok()` на unit `()`.
- Патч-клон: убрать `.ok()` (2 места).

### 2. `BoxShadow.inset: bool` добавлен в форке

- crates.io: 4 поля (color/offset/blur/spread) — `style.rs:308-317`.
- наш: + `pub inset: bool` — `Source/gpui/src/style.rs:345-355`.
- их `impl Interpolatable for BoxShadow` (`interpolate.rs:384`) не заполняет `inset`.
- Патч-клон: `inset: if t < 0.5 { self.inset } else { other.inset }`.

### 3. `Style.text` — `#[refineable]` в форке, нет у crates.io

- crates.io Style: `pub text: TextStyleRefinement` **без** `#[refineable]` → в `StyleRefinement` поле `text: Option<TextStyleRefinement>`.
- наш: `#[refineable] pub text: TextStyleRefinement` (`style.rs:291-292`) → в refinement `text: TextStyleRefinement` напрямую.
- их `fast_interpolate` match'ит `Option` (`interpolate.rs:446-456`) → type error.
- Патч-клон: прямой `self.text.fast_interpolate(&other.text, t, &mut out.text)`.

### 4. (только examples) `Application::new()` убран

- crates.io: `Application::new()` поднимает `current_platform(false)`.
- наш: только `with_platform` / `new_inaccessible`; ChronOS и форк-examples bootят через `gpui_platform::application()` (`Source/gpui_platform/src/gpui_platform.rs:13-15`).
- На lib-совместимость не влияет. Для smoke — mini-пример с `gpui_platform`.

## Расхождения со спекой/планом

1. **Бриф: «компилируется ли» (как gpui-form у Cline — без правок их кода)** → **нет**. Cline №1 получил «да без правок кода»; здесь нужны 3 правки исходников библиотеки. Манифест-only path-подмены недостаточно.
2. **Бриф: эскалация только на `cx.spawn` signature** → spawn **совместим**. Реальные блокеры — `AsyncApp::update` fallibility, `BoxShadow.inset`, refineable `text`.
3. **Бриф: «не патчь их код» при spawn-fail** → spawn не fail'нул; диагностический патч в клоне сделан осознанно, чтобы отделить «3 известных дельты» от «ещё неизвестный хвост». После патча lib чистая; новых ошибок нет.
4. Их 4 example-main не переписывались — вместо них `mini_hover` (тот же API surface).
5. `cargo test` крейта не гонялся (вне брифа; у них unit-тестов почти нет, examples = smoke).

## Не реализовано из acceptance criteria

- Интеграция в ChronOS (hover бара и т.п.) — **вне зоны**, только разведка.
- Патч upstream / PR в chi11321 — не делался.
- Их 4 штатных example main против нашего boot API — не починены (достаточно mini_hover).
- Сравнение с нашим `EasingCurve` (`gpui/src/easing.rs`) на уровне математики — не делалось; библиотека приносит **свой** `Transition` trait + Linear/и т.д., не зовёт наш `EasingCurve` напрямую (подтверждено: `rg EasingCurve` в клоне = 0 hits). То есть «слой поверх EasingCurve» из roadmap — продуктовая метафора, не literal dependency.

## Проверено фактом, не на словах

- `cargo check -p gpui-animation` (только path) → 8 errors:
  - `transition.rs:207/243` `E0599 no method named ok for ()`
  - `interpolate.rs:384` `E0063 missing field inset in BoxShadow`
  - `interpolate.rs:447-452` `E0308 expected TextStyleRefinement, found Option<_>` (+ `as_mut` на non-Option)
- После 3 патчей: `Finished dev profile … in 0.40s`, `EXIT:0`.
- `cargo check --example mini_hover` → `Finished … in 28.16s` (полная линковка gpui_linux/platform), `EXIT:0`.
- `cargo build --example mini_hover` → `Finished … in 1m 00s`, `EXIT:0`.
- `cargo tree -p gpui-animation --depth 1` → `gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)` + dashmap 6.1.0 + parking_lot 0.12.5 + smol 2.0.2.
- Живой: pid window, grim:
  - idle top color `(30,30,46)` ×1.47M px, box `(49,50,68)` ×13.8k;
  - mid top secondary `(58,87,194)` ×21.9k (интерполяция);
  - hover secondary `(137,180,250)` ×31.8k;
  - away ≡ idle (diff 0).
- Source `git status --short` — пусто. ChronOS tracked — без наших правок (только pre-existing untracked `skills/*`/`_ds/`).
- `cx.spawn(...).detach()` — строка `transition.rs:92`, signature `App::spawn` (`app.rs:1810`) принимает `AsyncFnOnce(&mut AsyncApp) -> R` — их `animation_tick` подходит.

## Новые риски / известные баги

- **Версии `0.2.x` совпадают, ABI/API — нет.** crates.io `gpui = "0.2.2"` ≠ наш path-форк с тем же semver. Любая внешняя gpui-lib потребует такой же check, не «версия совпала → ок». Severity: высокий для разведок, средний для продукта (патчи малы).
- **Fork-only патч vs upstream.** Без форка `gpui-animation` (или `[patch]` + vendored tree) ChronOS не сможет зависеть от crates.io-версии as-is. Severity: средний — 3 правки локальны и стабильны.
- **Свои curves, не Kael `EasingCurve`.** Если цель — единый easing-словарь с форком, придётся писать `impl Transition for OurCurve` или bridge. Severity: низкий (их trait тривиален: `calculate(t: f32) -> f32`).
- **README «early development, API subject to change» + 0.2.60** — минорные релизы частые; pin по git rev обязателен. Severity: средний.
- **`smol` в deps** — только `Timer::after` + channel wakeup внутри уже-нашей GPUI-таски (как оценил Архитектор). Третьего event-loop не поднимает (подтверждено кодом `transition.rs:246-249`). Severity: низкий.
- Runtime-смок — одно обычное xdg-окно, не layer-shell. Для bar-hover (layer-shell) семантика та же на уровне element/style, но first live bar integration всё равно обязателен. Severity: средний (честная граница).

## Статус ARCHITECTURE.md / DECISIONS.log

Не обновлены: разведка, решение «брать/не брать» — за Архитектором.
Если «брать» — кандидат в DECISIONS/roadmap:
- `gpui-animation 0.2.60` (ad77bea) **не** компилируется as-is против gpui-ce chronos edition;
- 3 дельты (AsyncApp::update non-Result, BoxShadow.inset, refineable text) + boot через gpui_platform;
- после патча — typecheck + live hover smooth confirmed;
- executor-модель (`cx.spawn.detach` + smol timer future) совместима с Runtime split.

## Хвосты для Архитектора

- Клон: `/home/neo/scratch/gpui-animation-recon` (с `target/`, патчи в `src/{transition,interpolate}.rs` + `examples/mini_hover.rs`). Можно снести после приёмки.
- Smoke PNG: `/tmp/gpui-anim-smoke/*4.png` (idle/mid/hover/away).
- ChronOS и `Source/` — ни байта не менялись; коммит в ChronOS **не нужен** (по брифу).
- Продуктовый следующий шаг, если «берём»: vendored/path crate с 3 патчами + зависимость в `crates/app` на hover правого кластера (Mimo №11) — отдельный бриф, не эта сессия.
