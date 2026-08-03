# T212 report — settings surface honesty (ACP/Editor/theme partial)

**Отчёт:** 2026-08-03. **Зона:** `tab/acp_settings.rs`, `tab/preview.rs` + drive-by
errata `assets.rs` (см. §5). **Источник:** T209 S6/S7/E9/B2.

## 1. ACP Reload visible + works (Must 1)

`#acp-reload` уже существовал — T209 читал его как "missing" из-за клиппинга:
sibling-блок с путём `~/.config/chronos/agents.toml` был `flex_1` **без**
`min_w(px(0.))`, поэтому длинный путь рос без ограничения и выталкивал
Reload за пределы 320 px viewport. Правка: путь `flex_1().min_w(px(0.))` +
`whitespace_nowrap().overflow_hidden().text_ellipsis()`, Reload —
`flex_none()` (никогда не сжимается, никогда не выталкивается).

`known_agents()` уже перечитывает `agents.toml` при каждом вызове reload
(проверено грепом — не новый код, существующий путь).

## 2. Missing file Editor (Must 2)

`read_for_preview` уже возвращал честную ошибку `"Cannot read '<path>': <os
error>"` через `State::Error` → `render_error` (путь+причина, не пустой
экран). Добавлено: `Err`-ветка `on_target_changed` была **немой** —
`Ok`-ветка логирует, `Err` нет. Провал открытия не оставлял следа в логе,
неотличимо от непройденного клика при разборе живого прогона. Добавлен
`tracing::warn!(path, error, "…load failed")`.

**Гипотеза T209 sub-FAIL S6** (blank surface при отсутствующем файле) — не
воспроизвелась статикой: код уже шёл в `State::Error`, не в пустой рендер.
Вероятная причина расхождения — стейл-бинарь на момент смока (T209 лог
09:xx, а этот путь ошибок не менялся с T179). Не опровергнуто и не
подтверждено живьём в эту сессию — см. §6.

## 3. agents.toml edit path (Must 3)

Решение: **View-only**, не Edit. `.toml` — `Text`-kind, а T194c скоупит
Preview/Edit dual-toggle на Markdown; Edit-intent на `.toml` и так тихо
даунгрейдится до View в `apply_intent`. Раньше кнопка называлась "Edit" и
обещала редактирование, которого не будет — переименована в "Open"/"View",
подписи в Actions-блоке и заголовке правлены под честную формулировку
("View agents.toml, then edit it externally, then Reload").

Отдельно задокументирован (комментарием в коде, не фикс) T194c-контракт:
повторный клик на тот же путь не перечитывает диск даже при бампе
`generation` — существующий тест `same_path_intent_switch_does_not_reload`
пинит это поведение. Не трогал — общий контракт с Files/Follow, решение
крупнее одной вкладки.

## 4. Light theme buffer/rail (Must 4, E9)

**Rail — подтверждено светлым живьём.** `toggle-theme` IPC → `theme.toml`
`scheme = "Light"`, скриншот `crates/app/src/side_panel_right/rail.rs`
рендера (`grim -g "2520,30 40x1410"`): бледно-лавандовый фон, иконки
тёмно-синие, читаемо. Код (`surfaces::chrome`, rail SVG `text_color`) уже
branch'ит на `theme.is_light` — E9 в части rail не воспроизвелась.

**Editor buffer — не переподтверждено живьём в эту сессию**, но
источниковый код уже покрывает случай явно: `render_editor_input_body`
(preview.rs) навешивает `.bg(surfaces::editor(theme))` на обёртку **и** на
сам `Input` (комментарий в коде: "gpui-component default (Light) fill was
the white projector on dark shell — Styled() applies after Input's own
appearance, so these win"), `surfaces::editor()` branch'ит на `is_light`.
Это код T205 (`8b36055`), уже был в бинаре на момент T209 смока (коммит
02.08 21:01, бинарь собран 03.08 09:01) — то есть T209 тестировал ЭТОТ ЖЕ
код и всё равно увидел тёмный буфер. Расхождение не объяснено: либо
скриншот T209 снят до полного применения темы (гонка hot-reload), либо
есть путь, не покрытый этим оверрайдом (например `Input`'s внутренний
gutter/scrollbar/selection через `gpui_component::Theme`, не
`chronos_ui::Theme` — не проверено).

Не чинил вслепую: **никакого дополнительного изменения в этом файле не
внесено** для этого пункта — не было воспроизводимого дефекта, который
можно было бы исправить, только неподтверждённое противоречие. Требует
живого перепрогона с рук — см. §6.

## 5. Drive-by errata (вне зоны T212): `follow.svg` не в `assets.rs`

Обнаружено живьём, не в задании: работающий процесс писал в лог
`ERROR gpui::asset_cache: Failed to load asset: asset error: Embedded
resource not found: icons/follow.svg` при каждом открытии `side_panel_left`.
T211 (`ee35b2b`, "Follow affordance") добавил
`img("icons/follow.svg")` в `panel.rs` и сам файл
`assets/icons/follow.svg` на диск, но **не добавил строку в
`assets.rs`'s `icons!` embed-list** — тот самый паттерн, задокументированный
в HANDOFF как правило T169 ("новая иконка живёт в двух местах"). Значит
Follow-иконка у пользователя сейчас не грузится вообще (пустой квадрат),
несмотря на "ACCEPTED static" в T211.

**Починено:** одна строка, `"follow.svg"` вставлена в алфавитном месте
между `"folder.svg"` и `"hexagon-core.svg"` (`assets.rs`). Не в зоне T212,
но тривиально и однозначно сломано — оставить как есть означало бы
заведомо разбитую иконку до следующей случайной находки.

## 6. Optional P2 (`margin.x` no-op) — не сделано

Вне Must-списка, не тронуто из-за бюджета сессии.

## Verification

- `cargo test --lib -p chronos` — **239/239 зелёных** (после всех правок
  включая `assets.rs` errata).
- `cargo build --release -p chronos` — **успешно** (~3m 17s), без новых
  предупреждений в изменённых файлах (все warnings в diagnostics —
  pre-existing, в vendored `Source/gpui*` крейтах, не в `crates/app`).
- IPC `toggle-theme` живьём на уже запущенном процессе (pid 2615759, бинарь
  собран **до** этой сессии, без текущих правок) — `theme.toml` переключился
  на Light, процесс не упал (T211's crash-fix держится).
- Rail screenshot в Light — подтверждён глазами (§4).
- Editor-буфер в Light — **NOT VERIFIED** живьём в эту сессию: попытки
  открыть вкладку Editor через `ydotool` упёрлись в нестабильное поведение
  клика по рейл-кнопке/handle на границе rail-only (40px) состояния —
  один и тот же клик то триггерил `dock toggle`, то `handle grab expanded
  rail → content width=40.0` (лог `chronos::side_panel_right::view`), то
  вообще не регистрировался. Это похоже на уже известный T210-остаток
  ("drag holds peek"), не на что-то новое в T212 — не чинил, не в зоне.
  Честно оставляю как NOT VERIFIED, а не подделываю PASS.

## Residuals для следующего живого прогона (руки, не мои)

1. Editor buffer в Light theme — переподтвердить визуально (см. §4, §6).
2. Follow-иконка — переподтвердить, что после **пересборки** (уже сделана
   `cargo build --release`, но текущий запущенный `pid 2615759` — старый
   бинарь, **требует рестарта шелла** чтобы подхватить фикс из §5).
3. T210 remainder (rail-only click flakiness) — не в зоне, но столкнулся
   с ним живьём при попытке проверить п.1; фиксирую как ещё одно
   независимое подтверждение, что резмок T210 всё ещё нужен.

**Коммит:** `settings : honesty reload missing-file light rail (T212)`.

## Architect accept note (2026-08-03)

**Verdict:** ACCEPTED WITH RESIDUAL (static + partial live).

- Reload clip fix + follow.svg embed: real, good.
- agents.toml **"View / edit externally"**: honest **until T213**. After T213
  (edit all text), ACP Open should use `PreviewIntent::Edit` again — T212
  copy is temporary product state, not final.
- Light editor buffer: still **LIVE N/V** (report honest).
- margin.x P2: deferred.

