# T305 — отчёт: control-center popup host

**Роль: FRONTEND.** Статус: **готово к приёмке** — реализовано, собрано,
протестировано, живой смок на release-бинаре проведён.

## Коротко

8 settings-табов правого рейла + новый Media-таб переехали в единый
anchored-popup (control center): клик по settings-иконке рейла открывает
layer-shell popup у этой иконки, внутри — таб-бар, переключающий тот же
`TabContent` через `TabContent::create(tab, cx)` (T304). Power-футер и
`power_row.rs` удалены. **Вход в popup — по решению владельца (ответ на
мой вопрос): иконки остаются на рейле как входы, рейл не создаёт для них
контентную панель — пересмотр решения №1 брифа.**

## Файлы

**Новые:**
- `crates/app/src/side_panel_right/control_center.rs` — хост + view:
  - `ControlCenterState` global (handle/view/active_tab/window-closed sub);
  - `is_popup_tab()` (9 табов: System, Media, Updates, Notifications,
    Display, EditorSettings, HyprlandBinds, AcpSettings, LauncherSettings);
  - `POPUP_TABS` — порядок таб-бара;
  - `open`/`close`/`close_this`/`toggle`/`active_tab`/`init`;
  - окно: LayerShell `Overlay`, TOP|RIGHT, никогда exclusive,
    `KeyboardInteractivity::None`, `exclusive_zone: Some(px(-1.))` —
    **обязательный opt-out** (см. «Кровный факт» ниже), margin из живых
    координат иконки;
  - view: таб-бар (9 иконок), body `overflow_y_scroll` (footer-clip trap),
    view-driven enter `arm_enter_progress` + `apply_enter_from_right`
    (слайд из-за края рейла — канон motion.rs:11-14);
  - кэш `HashMap<PanelTab, TabContent>` живёт во view — сущности табов
    создаются только тут и умирают с popup (решение №2 брифа);
  - ширина окна = `preferred_content_width()` таба (clamp 320..440),
    `window.resize` при переключении таба.
- `crates/app/src/side_panel_right/tab/media_tab.rs` — `MediaTab`: тонкая
  обёртка над `render_mpris_card` + подписка на `MprisState` (паттерн
  `tab/system.rs:49-50`); бэкенд `services::mpris` не тронут.

**Изменённые:**
- `tabs.rs` — `PanelTab::Media` вариант (id/parse_id/label/icon
  `icons/play.svg`/width 400). **Не в `ALL`** (popup-only; `ALL` = 21,
  тест `all_has_twenty_tabs_in_fixed_order` не переписывался — см. «Что НЕ
  сделано»).
- `tab/mod.rs` — `TabContent::Media` вариант + рукав `create` +
  `placeholder_description` + `mod media_tab;`.
- `rail.rs` — живой захват bounds иконки через `canvas` + `Rc<Cell<...>>`
  (bar-widget паттерн); `on_select` сигнатура теперь несёт
  `Bounds<Pixels>`.
- `rail_view.rs` — роутинг кликов: popup-таб → `control_center::toggle`
  (same→close, другой→remap), work-таб → `control_center::close` +
  `view.on_tab_select`; active-подсветка рейла при открытом popup — его
  active_tab.
- `view.rs` — снят весь power-футер: поля `power_arm`/`net_state`/
  `net_dl_history`/`net_ul_history`, `sample_network`, `on_power_click`,
  импорты `crate::power`/`net_stats`/`format_net_pair`; рукав
  `TabContent::System` без footer-хвоста.
- `mod.rs` — `pub(crate) mod control_center;` + `control_center::init` в
  `init()`; **хук `control_center::close(cx)` в `close()` и `close_this()`**
  (оба пути un-map рейла, mod.rs:523/593) — до early-return, чтобы
  осиротевший popup с уже закрытой панелью тоже умер.
- `tab/system.rs` — протухший док-комментарий про удалённый футер
  переписан (был ложью после удаления power_row).
- **Удалён** `side_panel_right/power_row.rs` (`git rm`).

## Кровный факт (найден живьём): `exclusive_zone: Some(px(-1.))`

Первый живой прогон: popup уезжал на ~48px влево от рейла и на 30px вниз.
Причина — ровно та ловушка, что задокументирована на
`content_window_options` (mod.rs): рейл резервирует 40px exclusive zone на
правом крае, композитор НЕ по запрошенному margin, а ДОПОЛНИТЕЛЬНО
сдвигает popup на эту резервацию (и бар сверху). Исправлено
`exclusive_zone: Some(px(-1.))` (wlr-layer-shell opt-out). После фикса
геометрия точная на обоих осях (замеры ниже).

## Верификация

### Сборка и юниты

```
$ cargo check -p chronos        → 0 errors; новых warnings нет
  (bin: 79 warnings — как до T305; lib: 131, все предсуществующие,
   в т.ч. render_group в rail.rs — мёртв ещё в HEAD, не мой)
$ cargo test -p chronos --lib   → 597 passed; 0 failed
$ cargo test -p chronos --bins  → 789 passed; 0 failed
$ cargo test -p chronos --lib side_panel_right → 199 passed
```

### Живой смок (release, DP-1 2560×1440, `hyprctl layers -j` + grim + ydotool)

Порядок и доказательства (лог `/tmp/chronos-smoke.log`, кадры `/tmp/*.png`):

1. **Открытие**: клик по иконке Updates (первая в рейле, кастомный
   `panels.toml`) → лог `control_center: popup opened tab="Updates"`,
   слой `control_center` в `hyprctl layers -j`.
2. **Геометрия**: popup x=2088, правый край 2508 = rail left 2512 − 4px
   (margin_right = wrap_inset 4 + RAIL 40 + GAP 8 = 52); y=38 = верх
   иконки (bar 30 + rail-local 8). Кадр `/tmp/smoke-2-popup.png`,
   карточка замерена: 417×557px.
3. **Remap на другую иконку**: клик по Notifications (2-я) →
   `popup opened tab="Notifications"`, y 38→70 = верх 2-й иконки.
   Тот же popup, x без изменений (правый край зафиксирован).
4. **Таб-бар самого popup**: клик по 2-й иконке таб-бара (Media) →
   `switched tab tab="Media"`, окно ресайзнулось (x 2108, ширина 400 =
   Media preferred). Кадр `/tmp/popup-media-crop.png`.
5. **Toggle**: повторный клик по той же иконке рейла → popup закрыт
   (слой исчез, нового open в логе нет).
6. **Un-map рейла (путь `close`)**: popup открыт → `chronos-ipc
   toggle-side-panel-right` → лог `rail closed` + `content closed`,
   слой control_center исчез. **Popup не пережил закрытие рейла.**
7. **Work-таб закрывает popup**: popup открыт на System → клик по Files
   (3-я иконка) → popup закрыт, панель: `switched tab → tab="Files"
   width=440.0` (обычное поведение рейла не сломано).
8. **System в popup**: клик по иконке System (нижняя группа, y=1350) →
   `popup opened tab="System"`, y=1336, ширина 400; кадр
   `/tmp/popup-system-crop.png` — дашборд рендерится (карточки
   (69,71,90), текст, акцент #007ACC), без panic — тяжёлый таб (5
   подписок сервисов) живёт в popup.
9. Ссылка на геометрию: кадры `/tmp/popup-notifications-crop.png`,
   `/tmp/smoke-3.png`, `/tmp/open.png`, `/tmp/closed.png`.

Среда после смока возвращена: `pkill -x chronos` (до сессии shell не
работал), курсор оставлен на (100,700), обои/мониторы не трогались.

## Что НЕ сделано

- **Тест `all_has_twenty_tabs_in_fixed_order` не переписан** — требование
  брифа вытекало из «settings уходят из `ALL`» (решение №1), которое
  владелец пересмотрел в ответ на мой вопрос: иконки остаются на рейле,
  `for_mode`/`ALL` не меняются. Состав рейла прежний, тест по-прежнему
  точен. Если решение №1 вернут — переписать под 13 табов.
- **`close_this` живьём не проверен**: это `#[allow(dead_code)]` запасной
  путь (кликов «наружу» у popup нет) — хук в обоих путях стоит
  (мод.rs:523/593), но второй путь сейчас недостижим. Проверен только
  `close`.
- **Персист active_tab**: грепом по `active_tab`/`PanelTab` в конфиге —
  активный таб НЕ персистится на диск (сессионный, `view.rs:62`,
  комментарий «session-only»). Фолбэк `resolve_active_tab` (view.rs) →
  `PanelTab::System` остаётся валидным: System-иконка на рейле есть,
  `PanelTab::default()` = System резолвится. Миграция не нужна —
  невалидного persisted-значения не существует. Холодный старт: панель
  открывается на System — валидной вкладке.
- **Enter-анимация кадром не снята**: стиллы брались через 1.2-1.5с после
  клика (анимация 260мс уже завершена). Механизм — канон motion.rs
  (view-driven, не `with_animation`); визуальная проверка слайда — на
  владельце/архитекторе.
- **Медиа-бэкенд не трогался** (`services::mpris` — вне зоны).
- **Коммит не делал** — в брифе раздела «Коммит» нет; изменения лежат в
  рабочем дереве (6 изменённых + 2 новых + 1 удалённый).
- В 16:12-13 в логе случилась серия «popup opened/switched tab» во время
  моей калибровки курсора — приписываю живым кликам пользователя
  (позже доказано: чистые move события кликов не порождают — парковка
  курсора дала ноль событий; все пути перепроверены контролируемым
  прогоном выше).
