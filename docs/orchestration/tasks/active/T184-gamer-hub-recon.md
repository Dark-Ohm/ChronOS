# T184 — разведка: источники игр и per-game scene для слайса 5

**Статус:** active. **Роль:** RECON.
**Правила:** `docs/orchestration/agents/RULES.md` (прочитать целиком).
**План (утверждён вариант A):** `docs/superpowers/plans/2026-08-02-gamer-hub-slice-5.md`.
**Спека:** `docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md` §4.2, §5, §13, §14 п.5.

**Ты ничего не пишешь в продуктовый код.** Ни строки в `crates/`, ни
правок `scenes.toml` пользователя. Продукт — отчёт с цифрами и `file:line`,
по которому архитектор пишет брифы T185–T188 без гадания.

**Отчёт:** `docs/orchestration/tasks/report/T184-gamer-hub-recon-report.md`.

После отчёта — **стоп.** Не двигать в `done/`, не в `report-log/`, не
«принята».

---

## Зачем

Слайс 5: Gamer at-rest hub + per-game scene model. Game Deck / telemetry /
gamepad — **слайс 6, не сюда**.

Без фактов на **этой машине** Library станет либо пустой декорацией, либо
фейком (обложки/playtime без backend). Разведка фиксирует: откуда брать
список игр, как launch, что уже умеет `scene` / `GamingModeState` / applications.

---

## Что уже решено планом (не переизобретать)

- Источник: XDG `.desktop` (Categories=Game) + эвристика id (`steam_app_*`,
  `heroic_*`, `lutris_*`, …) + pin/recent локально. **Не** Steam Web API.
- `scenes.toml` v1 расширяется optional-полями (`kind`, `app`,
  `apply_gaming_profile`, …) — ломки version нет.
- `Gamer ≠ GamingModeState`: вход в Gamer mode **не** включает compositor
  gaming profile.
- `restore_for_mode` остаётся read-only на диске; писать `[last]` будет
  будущий `activate` (T185), не ты.

---

## Что читаешь (зона)

Только чтение:

| путь | зачем |
|---|---|
| `crates/services/src/applications/` | `AppEntry`, parse `.desktop`, чего нет (Categories?) |
| `crates/app/src/scene.rs` | model, restore, нет `activate`? |
| `crates/app/src/system_popup/gaming_mode.rs` | apply/revert pub/private, hyprctl path |
| `crates/app/src/workspace_mode.rs` | smart-prompt, prefs per app id |
| `crates/app/src/dock/config.rs` | Gamer defaults, resolve_pinned |
| `crates/app/src/side_panel_right/tabs.rs` | `for_mode(Gamer)`, parse_id |
| `crates/app/src/launcher/` или кто launch'ает apps | как стартуют desktop Exec |
| живые файлы на диске (см. команды ниже) | цифры, не память |

Разрешено читать: `/usr/share/applications`, `~/.local/share/applications`,
flatpak exports, Steam shortcuts если есть. **Не** править.

---

## Вопросы (ответить все)

### 1. `AppEntry` и Categories

- Парсит ли сейчас `Categories=`? (`types.rs` — file:line)
- Что нужно добавить минимально, чтобы фильтр `Game` работал?
- Сигнатуры публичных типов — как есть.

### 2. Цифры на этой машине (команды + вывод в отчёт)

Сними **реальные** числа (подставь пути, если отличаются):

```bash
# сколько .desktop всего (user+system, грубо)
find /usr/share/applications ~/.local/share/applications -name '*.desktop' 2>/dev/null | wc -l

# с Categories содержащим Game
rg -l '^Categories=.*Game' /usr/share/applications ~/.local/share/applications 2>/dev/null | wc -l

# steam_app / heroic / lutris ids
find /usr/share/applications ~/.local/share/applications -name 'steam_app_*.desktop' 2>/dev/null | wc -l
ls /usr/share/applications/steam*.desktop ~/.local/share/applications/steam*.desktop 2>/dev/null
ls ~/.local/share/applications/ | rg -i 'heroic|lutris|steam' | head -40

# flatpak games exports (если есть)
ls ~/.local/share/flatpak/exports/share/applications 2>/dev/null | head -20
ls /var/lib/flatpak/exports/share/applications 2>/dev/null | rg -i game | head -20
```

В отчёте: **N games by Categories**, **N by steam_app_***, **примеры 5–10 id+Name**,
пустая ли Library без эвристики.

### 3. Launch path

- Как ChronOS сейчас запускает приложение из dock/launcher?
  (файл:строка, `Command`, `gio`, `gtk-launch`, …)
- Сработает ли типичный Steam Exec (`steam steam://rungameid/…` или
  `steam_app_730` desktop) этим же путём? **Проверь 1–2 реальных
  .desktop глазами** (Exec= строка), не запускай игру обязательно —
  достаточно показать Exec и сказать «тот же launcher path / нужен другой».

### 4. Scene model gaps

По `scene.rs`:

- какие поля у `Scene` сейчас;
- есть ли `activate` / запись `[last]` / seed hub — да/нет + line;
- что из плана (`kind`, `app`, `apply_gaming_profile`) ляжет как
  `#[serde(default)]` без миграции;
- риск: `extra` flatten vs новые named fields — коллизии?

### 5. GamingModeState

- `apply` / `revert` — pub или private? кто зовёт сегодня?
- Можно ли звать из `scene::activate` без правки popup UI?
- Что считается success/fail (log lines)?
- Напоминание в отчёте одной строкой: Gamer mode switch **не должен**
  звать apply (спека §5) — подтверди, что `workspace_mode::set` сейчас
  **не** зовёт gaming_mode (grep).

### 6. Gamer rail сегодня

- Точный список `for_mode(Gamer)` (enum variants).
- Сколько settings в tail; shared-order invariant с Developer.
- Куда логично вставить Library/Scenes/Captures (между System и settings)
  — согласуется ли с планом §2.5?

### 7. Pin/recent storage

Рекомендация **одним абзацем**: отдельный `~/.config/chronos/games.toml`
vs секция в `scenes.toml` — плюс/минус на фактах (кто уже пишет scenes,
риск затереть). Архитектор выберет; тебе нужен аргументированный совет.

### 8. Что НЕ трогать / out of scope

Явно перечисли: Game Deck, gamepad, Steam API, artwork CDN, playtime —
**нет в дереве / нет backend**. Если что-то из «нет» **есть** — file:line.

---

## Формат отчёта

```markdown
# T184 report

## 1 AppEntry / Categories
## 2 Machine counts (commands + numbers)
## 3 Launch path
## 4 Scene gaps
## 5 GamingModeState
## 6 Gamer rail
## 7 Pin/recent recommendation
## 8 Out of scope confirmed
## 9 Risks for T185–T188 (короткий список)
## Что НЕ сделано
```

Каждое утверждение о коде — **`path:line`**. Цифры — вывод команды, не
«примерно». Номера строк **с диска сейчас**, не из памяти (урок T159).

---

## Коммит

Кода нет. Если коммитишь только отчёт — `recon : T184 источники игр для
слайса 5`. `git add` поимённо. HANDOFF / MIGRATION / роли — **не трогать**.

---

## Верификация (для тебя)

- [ ] все 8 разделов заполнены
- [ ] хотя бы один блок machine counts с реальным `wc -l`
- [ ] grep: `workspace_mode` ↛ `gaming_mode` (или наоборот — с line)
- [ ] отчёт в `report/T184-…`, задание осталось в `active/`
