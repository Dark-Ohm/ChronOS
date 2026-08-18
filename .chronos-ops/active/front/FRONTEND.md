# FRONTEND — точка входа роли (кухня `.chronos-ops`)

**Роль:** UI, взаимодействие, тема — ChronOS. Не пишет сервисы/IPC/
packaging (это BACKEND).

**Общие правила:** `.chronos-ops/RULES.md` — прочитать перед стартом.

**Активное:**
- **T307** — `T307-wrap-geometry-hot-reload-stale.md`. Начинай с этого.
  P3, находка T303: `apply_wrap` early-return при уже открытой матте —
  `wrap.thickness`/`inner_radius` в `frame.toml` не подхватывается без
  рестарта шелла.

**Очередь FRONTEND пуста после T307** — сверяться с `checkpoint/HANDOFF.md`
за новыми находками владельца.

**Закрыто 2026-08-18:** T301 — `Select`-попап (composer): `max_w(px(N))`
вместо `w_full().min_w(px(0.))` в `ModelSelectItem`/`ModeSelectItem::render`,
`96f713a`. Корень (T298-хвост): `w_full()` = процент без definite-родителя
в MinContent/MaxContent-проходе флекса → `truncate_width = None` → текст
резался родительским `overflow_hidden` без `…`. Пиксельный `max_w` даёт
Taffy definite `AvailableSpace` → эллипсис реально вычисляется
(`Source/gpui/src/elements/text.rs:670-690`). **Живой грим снят архитектором**
(исполнитель честно не смог — протухший fd `ydotoold`, задокументировано в
отчёте): `Nous Portal · nvidia/nemotron-3…` и три другие длинные строки
кончаются реальным `…`, короткие (`qwen/qwen3.8-max`) влезают целиком.

**Закрыто 2026-08-18:** T302 — бага нет, rail-only при призыве = принятый
дизайн T220 (см. `checkpoint/HANDOFF.md`, «Кровные факты»). Не заводить
заново по тому же симптому: пустая контентная зона сразу после призыва —
ожидаемая картина, чат раскрывает клик по иконке рейла или `expand-left`.

**Закрыто 2026-08-18:** T305 — settings-табы правого рейла + Media в
единый anchored control-center popup, `f326fc7`. Кровный факт:
`exclusive_zone: Some(px(-1.))` — обязательный opt-out от резервации
рейла, иначе композитор сдвигает popup на 40px+бар. Решение №1 брифа
пересмотрено владельцем: иконки остаются на рейле, `ALL`/`for_mode` не
меняются (21 таб, Media popup-only, не в `ALL`).

**Закрыто 2026-08-18:** T303 — `wrap.thickness` разведён от
`bottom_strip.height`, единый санитизированный источник инсета, debug-лог
выпилен, `c6df21a`. Кровный факт: matte-геометрия — `LEFT|BOTTOM`-якорь с
**отрицательным** `margin` (`bottom=-inset`, `left=-inset`) компенсирует
резервации ровно; `LEFT|RIGHT` центрирует переполнение при открытой
панели (оставлен только `LEFT`); слой `Top`, не `Overlay` (тот глушит
клики рейла/панелей пустым input-region); `border_t_0()` — верх рисует
бар. Находка не в зоне тикета вынесена в **T307**.
