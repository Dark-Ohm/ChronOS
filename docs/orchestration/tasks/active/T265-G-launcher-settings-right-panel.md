# T265-G — настройки лаунчера в правой панели

**Статус:** ГОТОВА К ВЫДАЧЕ — T265-F в git (`ba810d8`). Live grim F ещё открыт, на код G не блокер.
Все ключи `launcher.toml` уже живут в волнах A–F.
**Приоритет:** P2.
**Родитель:** `T265-launcher-full-functionality.md`.
**Роль:** FRONTEND. Правая панель + persist.

## Задача

Отдельная страница настроек лаунчера в правой панели, не модалка, не
вторая «tune»-кнопка в футере OSD (её сняли в T275; вернуть можно
сюда — клик открывает эту вкладку через IPC `select-tab`).

Добавить `PanelTab::LauncherSettings` в
`crates/app/src/side_panel_right/tabs.rs` (`ALL`, `id`, rail label,
placeholder). `TabContent` в `tab/mod.rs` — живая вьюха, не `EmptyTab`.
Образец страницы: `tab/bar_settings.rs` / `tab/acp_settings.rs`.

Группы (ориентир AppGrid, ключи уже из A–F):

1. Внешний вид — компактный режим по умолчанию, подписи, плотность сетки.
2. Сетка — columns/rows, размеры иконки.
3. Поиск — включать скрытые в выдачу, инлайн-completion on/off.
4. Категории — какие показывать, hide empty (уже дефолт B).
5. Избранное — sort alpha, hide labels.
6. Системные действия — состав и порядок плиток F.
7. Скрытые приложения — список + поиск + Unhide (данные T265-D `[hidden]`).

Писать **только** `~/.config/chronos/launcher.toml` RMW `toml::Value`.
Не `bar.toml`, не `apply_patch` бара, не `frame.toml`.

Контролы: существующий `slider_control` из `bar_settings.rs` для чисел;
`Select` / toggle из кита. Свой слайдер не писать. Живой preview: OSD
подхватывает через watcher (300 ms, как frame/bar) — завести
`launcher_config` watcher, если в C его ещё нет.

Вернуть кнопку tune в футер OSD **только** вместе с этим бэкендом
(планка T246). Без вкладки кнопку не рисовать.

## Нельзя

- 50 мёртвых ключей «как у AppGrid» без читателя в коде.
- Трогать Appearance / Frame (T284) и `surface_alpha` (T266).
- `Source/gpui/`, `Cargo.lock`.
- Переносить модель лаунчера во вторую копию.

## Зона

- `crates/app/src/side_panel_right/tabs.rs`
- `crates/app/src/side_panel_right/tab/mod.rs`
- `crates/app/src/side_panel_right/tab/launcher_settings.rs` (новый)
- rail icon/label, если каталог настроек это требует
- `crates/app/src/launcher/**` — watcher + hot-apply

Не `side_panel_left`.

## Верификация

```
cargo test -p chronos --lib side_panel_right
cargo test -p chronos --lib launcher
```

Юниты: sanitize мусора в toml; hidden unhide вычёркивает id.

Live: вкладка открывается; ползунок columns сразу меняет сетку открытого
OSD; Unhide возвращает приложение; tune в футере ведёт сюда. Grim
вкладки + OSD рядом.

## Коммит

`feat(launcher): settings page in right panel (T265-G)`
