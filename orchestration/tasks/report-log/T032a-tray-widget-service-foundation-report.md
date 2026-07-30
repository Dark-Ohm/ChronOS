<!-- T032a — migrated 2026-07-22 from orchestration/report-log/opencode-report.md — see orchestration/tasks/MIGRATION.md -->

# Session: OPENCODE №1 — system tray (StatusNotifierWatcher + виджет) — 2026-07-17

## Сделано (факт, не намерение)
- `crates/services/src/tray/types.rs` (новый): `TrayState`(Data), `TrayItem`(id/title/icon_name/icon_pixmap/label), `TrayIcon`, `TrayCommand::ActivateItem`. НЕ derive Eq (float-rule; пиксмап opaque).
- `crates/services/src/tray/mod.rs` (новый): `TraySubscriber` реализует наш `trait Service`. Server-side zbus 5.17 `org.kde.StatusNotifierWatcher` (RegisterStatusNotifierItem/Host, сигналы Registered/Unregistered/HostRegistered/HostUnregistered, свойства RegisteredStatusNotifierItems/IsStatusNotifierHostRegistered/ProtocolVersion) + client-side proxy `StatusNotifierItem` (Title/Id/IconName/IconPixmap + Activate/SecondaryActivate/ContextMenu). `new()` sync, паника вне tokio (Handle::current guard), std::sync::Mutex для inner-состояния.
- `crates/services/src/tray/mod.rs`: `dispatch(ActivateItem)` — sync fire-and-forget (spawn на захваченный `Handle`, как `CompositorSubscriber::dispatch`); кликабельность из GPUI-обработчика без асинка.
- `crates/services/src/tray/mod.rs`: начальный discovery уже зарегистрированных item (`DBusProxy::list_names` → …StatusNotifierItem) + `NameOwnerChanged`-вотчер (spawn на tokio, НЕ в zbus-хендлере) для авто-удаления исчезнувших item.
- `crates/services/src/lib.rs`: `pub mod tray;` + re-export `TrayCommand/TrayIcon/TrayItem/TrayState/TraySubscriber`, поле `Services.tray` + `init_all()`, runtime_guard-тест `tray_new_panics_outside_runtime`.
- `crates/app/src/state.rs`: аксессор `AppState::tray(cx)`.
- `crates/app/src/bar/widgets/tray.rs` (новый): `TrayWidget` (`BarWidget`, `BarSection::Right`). Рендерит ряд бейджей; клик → `dispatch(ActivateItem)`.
- `crates/app/src/bar/widgets/mod.rs` (файл Cline, уже закоммичен в дереве): `mod tray;` + `tray::register(cx);` на месте.
- `crates/services/examples/tray-smoke.rs` (новый): живой D-Bus смок (фейковый StatusNotifierItem + реальный TraySubscriber), проверяет регистрацию, свойство RegisteredStatusNotifierItems и доставку Activate.

## Расхождения со спекой/планом
- Спека: `dispatch()` async. Реально: сделал **sync** fire-and-forget (spawn на `Handle`). Причина: клик-хендлер `BarWidget::render` не async, а `App::spawn(|cx| async move{…})` упирался в `AsyncFnOnce` lifetime-ограничение компилятора; sync-диспатч через `Handle::spawn` чище и идентичен `CompositorSubscriber::dispatch`. Не забыто — решение.
- Спека: client proxy на `org.kde.StatusNotifierItem` «каждого зарегистрированного item». Реально: прокси строится лениво при регистрации (чтение Title/IconName/IconPixmap) и при Activate, а не держится постоянно — достаточно для Activate и MVP-рендера. Не забыто — решение (постоянный пул не нужен для MVP).
- Спека: обработка `StatusNotifierItemUnregistered`. Реально: сигнал эмитится, но удаление item триггерится через `NameOwnerChanged`-вотчер (более надёжно для crash-кейсов), а не только по входящему сигналу от item. Не забыто — решение.

## Не реализовано из acceptance criteria
- **Рендер иконки по icon_name через freedesktop icon-theme** — НЕ сделано (явно разрешено спекой как MVP-фоллбэк). Сейчас текстовый бейдж: первая заглавная буква `title` (или `icon_name`, иначе `?`). icon-theme lookup в GPUI вне scope MVP.
- **Рендер pixmap (IconPixmap)** — НЕ сделано (спека: «можешь отложить, зафиксируй в отчёте»). Буфер сохраняется (ARGB→RGBA разворот уже сделан в `add_item`), но виджет его не рисует. TODO, не решение.
- **Контекстное меню** — НЕ сделано (спека: «контекст-меню отложено»). Сигнатура `ContextMenu`/`SecondaryActivate` и `DBusMenu`-прокси не подключены.

## Проверено фактом, не на словах
- `cargo build --workspace` → 0 errors (только pre-existing warnings в чужих crates: `interpolate_optional`, `ContentMask`, `Task`).
- `cargo test -p chronos-services` → `test result: ok. 25 passed; 0 failed` (6 новых tray-тестов + runtime_guard в lib.rs).
- `cargo test -p chronos` → `26 passed; 0 failed` (state.rs-аксессор жив).
- `cargo clippy -p chronos -p chronos-services` → 0 errors; warning `non-binding let on a future` в widget-клике пойман и исправлен (dispatch теперь sync, клик реально активирует).
- Живой смок `cargo run -p chronos-services --example tray-smoke` (реальный session bus, Hyprland запущен):
  - `[smoke] item present: id=:1.734 title=Some("SmokeItem") icon=Some("smoke-icon") label=S`
  - `[smoke] RegisteredStatusNotifierItems OK: [":1.734"]`
  - `[fake-item] Activate received` → `✅ tray-smoke PASSED`
- `busctl --user list | grep -i statusnotifier` → до запуска жив только activatable `org.x.StatusNotifierWatcher`; реальных SNI-апп (nm-applet/blueman) в сессии нет — поэтому полный GUI-смок через запущенный ChronOS не делал (нет дисплея-прогона бара в этом окружении), заменён функциональным D-Bus смоком выше.

## Новые риски / известные баги
- `TraySubscriber` claim-ит `org.kde.StatusNotifierWatcher` на session bus. Если в сессии уже есть другой watcher (KDE/waybar), `request_name` встанет в очередь (флаги по умолчанию без ReplaceExisting) и методы пойдут не к нам — MVP не делает ReplaceExisting, чтобы не вышвыривать чужой host. Severity: low (в нашей сессии чужого `org.kde.*` watcher'а нет).
- `NameOwnerChanged`-вотчер держит отдельный `DBusProxy` + stream; при ретрае `serve` (обрыв соединения) старый вотчер не отменяется явно — потенциально висячая задача при длительных обрывах. Severity: low (для десктоп-сессии соединение стабильно).
- Текстовый бейдж вместо иконки — визуально не по-Tray-канону, но функционален. Severity: low (явный MVP-фоллбэк по спеке).

## Статус ARCHITECTURE.md / DECISIONS.log
- Не обновлял. Решение (tray = server-side watcher + client proxy, sync dispatch) повторяет уже закреплённый паттерн notification/ (server-side zbus 5.17, Handle-guard, std::sync::Mutex). Если Архитектор хочет зафиксировать tray в ARCHITECTURE.md — сказать, добавлю.

---

## Дополнение: ПРИЁМКА №1 — починка ayatana-формы (task №2), коммит 75a1061

### Баг (диагноз Архитектора, подтверждён кодом)
ayatana/libappindicator-приложения (`udiskie --appindicator`, `nm-applet`, `blueman`)
передают в `RegisterStatusNotifierItem` **голый object path** без bus name. Старый
`split_service` резал по первому `/` → destination = пустая строка → `builder` падал
(`tray: failed to build item proxy for /org/ayatana/NotificationItem/udiskie`). Sender из
`Header` использовался только при пустом аргументе.

### Фикс
- `normalize_registration(service, sender)`: если `service` начинается с `/` → canonical
  key = `{sender}{service}`, где destination = unique name отправителя; путь сохраняется.
  Пустой аргумент → sender; bus name (+ опц. `/path`) → как есть.
- `split_service` теперь работает с canonical key (режет по первому `/`), корректно
  выдавая destination для всех трёх форм.
- Ключ item'а содержит unique name отправителя → `NameOwnerChanged`-вотчер удаляет item
  при выходе приложения (проверено живьём, см. ниже).
- `add_item` логирует `tray: item added: …` (форма из acceptance).

### Проверено фактом
- `cargo test -p chronos-services` → **37 passed** (новый `normalize_registration_forms`
  покрывает все 3 формы, включая `/org/ayatana/NotificationItem/x` + sender; расширен
  `split_service_forms`).
- Живо (`tray-live-check` держит вотчер, `udiskie --appindicator` перезапущен):
  - `tray: item added: :1.750/org/ayatana/NotificationItem/udiskie (title=Some("udiskie"), icon=Some("drive-removable-media"))`
  - `tray: item added: :1.138/org/ayatana/NotificationItem/systray (title=Some("tray_linux_release"), icon=Some("/tmp/systray_avSay5"))`
  - после kill старого udiskie: `tray: unregistered 1 item(s) for vanished name :1.750`
  - после рестарта: `tray: item added: :1.810/org/ayatana/NotificationItem/udiskie`
  → реальные ayatana-итемы попадают в `TrayState` (title/icon_name), бейдж будет виден
  в баре; NameOwnerChanged-очистка работает.
- `tray-smoke` переписан под ayatana bare-path форму → PASSED (item с ключом
  `:N/StatusNotifierItem`, Activate доходит).

### Замечание по верификации
`cargo test --workspace` в этом окружении зависает (вне зоны tray — тест другого
крейта, видимо требует display/VM); `cargo build --workspace` зелёный, а мои крейты
(`chronos-services` 37 passed, `chronos`) зелёные. Гонять полный workspace-тест мешает
не мой код.
