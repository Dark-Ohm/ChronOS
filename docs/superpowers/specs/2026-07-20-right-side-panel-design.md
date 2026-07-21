# Правая боковая панель — Design Spec

**Date:** 2026-07-20
**Status:** DRAFT (brainstorming complete, ждёт ревью)
**Scope:** новый модуль `crates/app/src/side_panel_right/` +
новые сервисы `crates/services/src/system_resources/`, `crates/services/src/power/`

---

## 1. Цель

Первая из двух боковых панелей ChronOS (см. `roadmap.md` §«Боковые панели»).
Правая панель — расширенная поверхность виджетов, слишком тяжёлых для бара:
полноценный MPRIS-плеер, живой мониторинг CPU/RAM/GPU, сетевой трафик,
и системные power-действия. Левая agent-панель (чат) — отдельная спека,
не в этом документе.

**Успех v1:**
- Hover у правого края монитора показывает панель (peek), уход курсора
  без пина — закрывает.
- Хоткей/клик по MPRIS-виджету в баре — пинит панель открытой до
  повторного тоггла/Esc.
- MPRIS-блок реально управляет активным плеером (play/pause/next/prev/mute).
- CPU/RAM/GPU и сетевой трафик показывают живые данные, читаемые
  на глаз, без «эффекта LED-гирлянды на морозе» (см. отклонённые
  визуальные итерации в `.superpowers/brainstorm/` этой сессии).
- Power row вызывает реальные log out / restart / shutdown. Switch
  user — видим, но disabled (нет login manager'а, см. §3.4).

**Вне зоны v1** (осознанно отложено, roadmap):
- Видео-превью (`gpui-video-player`) — отдельная веха.
- Кнопка «развернуть плеер» (шеврон) — в v1 это **визуальный стаб без
  действия** (см. §6); полноценный expanded-player — будущая веха.
- Левая agent-панель — отдельная спека.

---

## 2. Архитектура

### 2.1 GPUI-окно — layer-shell overlay, не exclusive zone

Панель **не** держит экран постоянно занятым (в отличие от бара) —
это lazy `Layer::Overlay`, по образцу launcher/notifications
(`ARCHITECTURE.md §4`):

```rust
WindowOptions {
    kind: WindowKind::LayerShell(LayerShellOptions {
        namespace: "side_panel_right".to_string(),
        layer: Layer::Overlay,
        anchor: Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
        exclusive_zone: Some(px(0.)),   // не двигает рабочую область
        margin: (0, 0, 0, 0),
        keyboard_interactivity: KeyboardInteractivity::None, // до пина
        ..Default::default()
    }),
    ..
}
```

Ширина панели — `300px` (зафиксировано визуальным брейнштормом), высота —
во весь монитор (top..bottom anchor).

### 2.2 Состояния: hover-peek vs pinned

Два независимых триггера открытия (подтверждено пользователем: «1+3»):

1. **Hover-peek** — курсор у правого края экрана (зона ~4px хит-тест,
   отдельная невидимая полоса-детектор или Hyprland `layoutmsg`/pointer
   events — техническое решение хит-теста уточняется в плане) открывает
   панель в **непинованном** состоянии. Уход курсора с панели и из зоны
   триггера — закрывает с небольшим debounce (не мгновенно, чтобы не
   мигало при проходе мимо).
2. **Pin** — хоткей (по аналогии с launcher-тоглом) или клик по
   MPRIS-виджету бара открывает/держит панель **пинованной**:
   не закрывается по уходу курсора, закрывается по повторному тоглу
   или Esc.

Состояние `Peek | Pinned` — простое перечисление в state панели,
пин виден в UI как приглушённая точка-индикатор в шапке секции (см.
мокап `panel-full.html`).

**Известный риск** (внести в план как явный пункт, не решать здесь):
хит-тест «курсор у края монитора» на layer-shell поверхности Hyprland
не тривиален — surface для детектора должен либо быть отдельным
тонким invisible layer-shell окном на правом краю, либо use compositor
event (`hyprctl`/hyprland-rs pointer position polling). Нужна разведка
перед реализацией, не блокирует спеку, блокирует план.

### 2.3 Reentrant-close и dismiss

Наследует стандартные попап-конвенции `ARCHITECTURE.md §4.1`:
explicit dismiss only (Esc/re-toggle/click-away при pinned), close_this
guard для reentrant `remove_window()`.

---

## 3. Компоненты (сверху вниз в панели)

### 3.1 MPRIS-карточка

- Обложка `64×64px` (было 44 — увеличено по фидбеку «картинку побольше»).
- Название/исполнитель, прогресс-бар с таймкодом (текущее/общее).
- Transport — три круглых icon-button (prev/pause-play/next), pause
  — «primary» (крупнее, акцентный цвет), НЕ эмодзи-строка.
- Mute-кнопка справа от transport — мутит **источник** (сам плеер/браузер,
  тот процесс что играет), НЕ master sink. MPRIS-протокол per-player mute
  не поддерживает, поэтому это делается через PipeWire на уровне
  application-стрима: `wpctl set-mute <stream-id> toggle`, где
  `<stream-id>` — id PipeWire-стрима приложения (`pw-dump` секция
  streams/sink-inputs, не sinks/sources). Существующий
  `crates/services/src/audio/` **на master** парсит sink/source
  (`AudioDevice`/`EndpointState`); per-app streams — **Task 6**
  (в working tree есть WIP `AudioStream`/`ToggleStreamMute` —
  **не принят, не коммитить вслепую**). Сопоставление MPRIS→stream —
  эвристика по `application.name`/процессу; 1:1 нет → no-op + лог,
  не паника.
- Шеврон-стрелка в углу карточки — «open full player». **В v1 это
  визуальный элемент без обработчика** (задел под будущий expanded-player).
- Данные и команды play/pause/next/prev — существующий
  `crates/services/src/mpris/{mod,types}.rs`
  (`MprisCommand::PlayPause/Next/Previous`), без изменений контракта.

### 3.2 System-секция (CPU/RAM/GPU)

- Три строки «spectrum» — лейбл слева (`CPU`/`RAM`/`GPU`, JetBrains
  Mono, uppercase), 14 тонких вертикальных баров высотой `52px`
  (история последних N сэмплов), число справа в фиксированной колонке
  (`67%` и т.п., JetBrains Mono).
- Палитра — холодная монохромная (правка «не гей-парад»): CPU
  `#5fd3e8` (ярче всех — акцент шелла), RAM `#4fa3c9`, GPU `#33638a`.
  Разделение по яркости, не по оттенку.
- Данные — новый сервис `crates/services/src/system_resources/`:
  - CPU/RAM через `sysinfo` (polling-тик, интервал — по аналогии с
    существующими poll-сервисами типа upower).
  - GPU через `nvml-wrapper` (NVML уже установлен в системе,
    `libnvidia-ml.so.610.43.03` подтверждён). **Nvidia-only** — на
    системах без NVML секция GPU скрывается (нет крейта деградации до
    "N/A", известное ограничение, зафиксировать в плане, не решать сейчас).
- Ring-buffer сэмплов на виджет — тот же принцип, что чинили в
  `network.rs` 2026-07-20: время-гейт + кэш последнего значения,
  инъекция `Instant`/`Duration` параметром, НЕ зависящий от частоты
  вызова `render()` (см. `skills/chronos-shell/SKILL.md` правило 0).

### 3.3 Network-секция

- Тот же spectrum-идиом, две строки `↓ dn` / `↑ up`.
- Палитра — тоже холодная: down `#7cc4e8`, up `#3d6d94`.
- Данные — переиспользуть существующий network-сервис
  (`crates/app/src/bar/widgets/network.rs` + сервисный слой) — панель
  не дублирует сбор трафика, подписывается на тот же источник, что и
  бар. Бар-виджет (лампочка+спидометр) остаётся как есть — панель это
  расширение, не замена (замена — будущая веха, roadmap).

### 3.4 Power row

**Контекст, меняющий объём:** в системе нет login/display manager'а
(нет greetd/SDDM/lightdm) — сессия стартует напрямую (TTY autologin →
exec Hyprland или аналог). Свой login manager — будущий, отдельный
проект, не существует сегодня. Это значит:

- **Switch user** — некуда переключать: нет greeter'а, которому можно
  отдать VT. **В v1 — кнопка присутствует визуально, но disabled**
  (приглушена, курсор `not-allowed`, тултип «требует login manager»).
  Не реализуем до появления собственного login manager'а — не гадать
  про `loginctl`/`dm-tool`, эта земля пуста.
- **Log out** — реализуемо: `hyprctl dispatch exit` завершает сессию
  compositor'а, TTY-autologin решает что происходит дальше (либо назад
  в shell-логин на TTY, либо respawn — зависит от инит-скрипта сессии,
  не от ChronOS).
- Restart → `systemctl reboot`.
- Shutdown → `systemctl poweroff`.

Три кнопки в ряд (Switch user disabled / Log out / Restart / Power) —
иконки-обводки, без заливки, тихие — `color: #7d80a6`, hover `#c8cae0`;
Power при hover краснеет `#f38ba8` — единственная «опасная» подсветка
в активных кнопках. Disabled-состояние Switch user — отдельный
визуальный токен (ещё не мокапилось, нужна итерация при реализации).

Новый сервис `crates/services/src/power/` — тонкая обёртка над
`hyprctl dispatch exit` / `systemctl reboot` / `systemctl poweroff`.
Switch user в сервис не входит вообще (нечего оборачивать).

**Все действующие кнопки (log out/restart/shutdown) — подтверждающий
диалог обязателен** (не в этой спеке, но зафиксировать: destructive-
action правило проекта распространяется и на UI, не только на
git/shell).

---

## 4. Данные и жизненный цикл

Панель — GPUI entity, подписывается на:
- `MprisSubscriber` (существует)
- `AudioSubscriber` — **расширяется** (Task 6) командой
  `ToggleStreamMute(u32)` + парсингом application-стримов из `pw-dump`
  (на master — только sink/source; WIP в tree — см. §3.1)
- новый `SystemResourcesSubscriber` (CPU/RAM/GPU)
- существующий network service (переиспользуется как есть)
- новый `PowerSubscriber` (dispatch-only, без состояния — команды
  fire-and-forget через `Command::new(...).spawn()`, ошибки — `log_err()`,
  не глушить `let _ =`)

Все новые сервисы — `[lints] workspace = true` в своих Cargo.toml
(обязательное правило проекта).

---

## 5. Визуальный дизайн

Валидировано визуальным брейнштормом этой сессии (мокапы в
`.superpowers/brainstorm/1665344-1784566844/content/panel-full.html`,
финальная итерация). Ключевые решения:

- Шрифты — `Inter` (UI-текст) + `JetBrains Mono` (числа/лейблы),
  соответствует `STYLE.md` (`font_ui`/`font_mono`).
  **Статус 2026-07-21:** оба поля в `Theme` (`font_mono` `3e04264`,
  `font_ui` `"Inter"` `18c88f0`). Потребители панели ещё пишутся
  (Task 9+); до подключения — GPUI default family.
- Панель: `300px` ширина, фон `#1a1a26` (темнее bg.secondary бара —
  панель на overlay-слое, не на bg.tertiary), бордер `#26273a`, отступ
  `20px`, gap между секциями `20px`.
- Никаких скруглённых прогресс-баров/циферблатов/LED-сегментов —
  все три направления явно отклонены пользователем в процессе
  (см. `.superpowers/brainstorm/.../meter-style.html`,
  `geometry-style.html`, `combined.html` — сохранить как историю
  решений, не удалять).
- Светлая тема (Light C) — **не покрыта этим брейнштормом**, нужна
  отдельная итерация мокапа перед реализацией светлого кадра панели
  (тот же процесс, что уже применялся к Updates/Volume/System попапам).

---

## 6. Что явно НЕ входит в v1

- Видео-превью (`gpui-video-player`).
- Функциональный expanded-player за шевроном (только визуальный стаб).
- Замена бар-виджетов (network/mpris в баре остаются).
- Светлая тема панели (мокап не пройден).
- Левая agent-панель.
- Конфигурируемость (какие метрики показывать, hover-таймауты и т.п.)
  — v1 хардкодит разумные дефолты, конфиг — если понадобится после
  живого использования.

---

## 7. Тестирование

- Unit: ring-buffer сэмплинга CPU/RAM/GPU/network — инъекция времени,
  проверка «не зависит от частоты вызова render» (паттерн
  `network.rs` тестов, 2026-07-20).
- Unit: `command_to_*_args`-стиль чистые функции для power-команд
  (аргументы `systemctl`/`loginctl`/`hyprctl`), без реального spawn
  в тестах.
- Live smoke (обязательно, per `verification-before-completion`
  skill и HANDOFF.md): release-сборка, реальный hover/pin на живом
  Hyprland, реальная GPU-нагрузка (`glxgears`/игра) для проверки, что
  spectrum-бар GPU реально шевелится, `RUST_LOG=info`, grim-скриншоты
  обоих состояний (peek/pinned).
- Power-действия — **живой смок ОБЯЗАН включать хотя бы restart** на
  тестовой машине с подтверждением пользователя перед вызовом (нельзя
  тестировать shutdown/restart без явного разрешения в моменте —
  destructive action).

---

## 8. Открытые вопросы для плана (не для этой спеки)

1. Механизм хит-теста hover-у-края (invisible layer-shell strip vs
   compositor polling) — требует технической разведки.
2. ~~Log out / switch user — механизм~~ — снято: нет login manager'а
   в системе (будет свой, отдельным проектом, не сейчас). Log out =
   `hyprctl dispatch exit`, Switch user = disabled-стаб в v1. Не
   переоткрывать этот вопрос без нового факта о login manager'е.
3. ~~`font_ui` в `Theme` — blast radius~~ — **закрыто** `18c88f0`:
   одно поле + `Default`; схемы/base16 через `Theme::default()`,
   второй литерал `Theme {…}` не потребовался.
4. Per-app stream mute — формат `pw-dump` / match-эвристика: Task 6.
   В working tree есть незакоммиченный WIP; **не считать done** без
   приёмки + коммита.
