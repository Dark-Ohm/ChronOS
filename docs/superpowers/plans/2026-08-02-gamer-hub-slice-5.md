# Gamer at-rest hub + per-game scene model (слайс 5) — Implementation Plan

Реализует §14 п.5 спеки
`docs/superpowers/specs/2026-07-30-adaptive-developer-gamer-shell-ide-design.md`
(«Gamer at-rest hub shell and per-game scene model»).

**Опирается на закрытое:**
- слайс 1 — `workspace_mode` Developer/Gamer, IPC, smart-prompt contract;
- слайс 2 — `scene.rs` + `scenes.toml` v1, композиция рейла/дока по режиму;
- слайс 3 — контракт вкладки, 14-tab catalog, ширина по вкладке;
- слайс 4 — Developer workbench (Files/Terminal/Build/Preview) — **не трогаем**;
- `GamingModeState` (`system_popup/gaming_mode.rs`) — apply/revert через `hyprctl eval` + power profile, **отдельный слой от workspace mode** (§5).

**Объём: вариант A — hub + scenes + Captures empty + T189 profile wire. УТВЕРЖДЁН пользователем 2026-08-02.**

---

## 1. Что говорит спека

§4.2 At-rest game hub — Gamer workspace **может** показывать:

- recent and pinned games;
- session history / playtime;
- artwork / live media;
- achievements / social — **when real integrations exist**;
- captures / replay;
- hardware and controller state;
- **per-game scene configuration**.

A game scene can remember: monitor, Hyprland workspace, audio output,
microphone, performance profile, recording defaults, companion apps.
Resolution/refresh — capability-gated follow-up.

§5 Transition contract (обязательно соблюдать):

- mode switch **may** restore last scene, change rail/dock;
- mode switch **must not** terminate apps, discard panel state,
  change audio/performance/recording/display **without explicit scene settings**;
- `WorkspaceMode::Gamer` ≠ `GamingModeState` — вход в Gamer **не** включает
  compositor gaming profile; UI не рисует профиль «активным» только потому,
  что выбран Gamer.

§14 п.6 (Game Deck + telemetry + gamepad) — **следующий слайс, не этот**.

§13 — честные `unavailable` / `permission required`, без «coming soon».

---

## 2. Главное решение слайса

### 2.1 Две поверхности Gamer

| поверхность | слайс | что |
|---|---|---|
| **At-rest hub** | **5 (этот)** | библиотека игр, список/активация сцен, per-game конфиг |
| **In-game Game Deck** | 6 | overlay поверх игры, FPS/capture/audio, gamepad |

В слайсе 5 **нет** overlay-панели поверх fullscreen-игры. Есть правый рейл
в Gamer mode (как сейчас System+settings) + контент hub-вкладок.

### 2.2 Что уже есть vs чего нет

**Есть (проверено в дереве 2026-08-02):**

| кусок | где |
|---|---|
| `Scene` / `ScenesConfig` / `restore_for_mode` (read-only disk) | `crates/app/src/scene.rs` |
| Gamer rail = System + 6 settings (7 вкладок) | `tabs.rs::for_mode` |
| Gamer dock default `steam/discord/firefox/kitty` | `dock/config.rs` |
| `GamingModeState::apply/revert` | `system_popup/gaming_mode.rs` |
| `.desktop` скан без Categories | `crates/services/src/applications/` |
| smart-prompt prefs per app id | `workspace_mode.rs` tests (`steam_app_570`) |

**Нет:**

- API **активации** сцены пользователем (`activate` + запись `[last]`);
- per-game полей в `Scene` (app, companions, apply_gaming_profile, …);
- вкладок Library / Scenes / Captures;
- UI hub;
- фильтра «это игра» в `AppEntry` (нет `categories`);
- Game Deck / telemetry / gamepad (слайс 6).

### 2.3 Источник списка игр (решение)

**Не** тащить Steam Web API / ProtonDB / achievements в слайс 5.

Минимум, достаточный для hub (**уточнено T184, 2026-08-02**):

1. **XDG `.desktop`** с `Categories` содержащим `Game` (после расширения парсера);
2. **эвристика Exec** (не filename): `steam steam://rungameid/<id>`,
   плюс id-префиксы `steam_app_*` / `heroic_*` / `lutris_*` если появятся;
   на этой машине `steam_app_*.desktop` = **0**, реальные ярлыки —
   `Counter-Strike 2.desktop` и т.п. с `rungameid` в Exec;
3. **исключить** client `steam.desktop` (Categories=Game, но не игра);
4. **pinned / recent** — отдельный `~/.config/chronos/games.toml`
   (решение T184, не секция scenes.toml).

Playtime / artwork / achievements — `unavailable` с причиной «no integration»,
пока нет реального backend. Не рисовать фейковые часы и обложки.

**Разведка T184** обязана подтвердить цифры на этой машине
(сколько `.desktop` с Game, сколько steam_app_*, жив ли Steam flatpak id)
до того, как FRONTEND рисует Library.

### 2.4 Формат сцены — расширение v1, не ломка

`version` остаётся **1**. Новые поля — optional с `#[serde(default)]`.
`extra` flatten уже ловит неизвестное (слайс 2). Битый/старый файл не
затираем (урок T164).

```toml
version = 1

[last]
developer = "chronos"
gamer = "hub"

[[scene]]
id = "hub"
name = "Game Hub"
mode = "gamer"
# kind отсутствует или "hub" — at-rest библиотека, без app launch target
kind = "hub"
rail_tabs = ["system", "library", "scenes", "captures",
             "acp_settings", "mcp_settings", "lsp_settings",
             "api_providers", "editor_settings", "hyprland_binds"]
active_tab = "library"
dock = ["steam", "discord", "firefox", "kitty"]

[[scene]]
id = "game-steam-730"
name = "Counter-Strike 2"
mode = "gamer"
kind = "game"
# desktop id / steam_app id — то, что launch и smart-prompt уже понимают
app = "steam_app_730"
# companions = оверрайд дока для этой сцены (можно оставить поле dock)
dock = ["steam", "discord", "mumble"]
# false по умолчанию: Gamer ≠ GamingModeState (§5)
apply_gaming_profile = false
# capability-gated: храним, не применяем, пока нет сервиса
# audio_sink = ""
# microphone = ""
# hyprland_workspace = ""
display = ""   # UUID вывода, как в слайсе 2; пусто = pult default
```

**Правила:**

- `kind = "hub" | "game"` (default `""` → трактовать как hub, если `app` пуст,
  иначе game);
- `app` пуст у hub; у game обязателен для launch, иначе launch = unavailable;
- `apply_gaming_profile = true` — **единственный** путь, где сцена зовёт
  `GamingModeState::apply`; revert при уходе на hub / другую сцену без флага;
- запись `[last].gamer` — **только** из user-path `scene::activate`, не из
  `restore_for_mode` (он остаётся read-only на диске).

### 2.5 Рейл Gamer после слайса

`PanelTab::ALL` растёт. Developer work-tools **не** получают новые gamer-only
вкладки в `for_mode(Developer)`.

Предлагаемый Gamer set (порядок, §5 shared relative order для System+settings):

```
System,
Library,      // NEW — GameLibrary
Scenes,       // NEW — list/activate/create per-game scenes
Captures,     // NEW — EmptyTab / unavailable (нет capture backend)
+ settings group (6) unchanged
```

Итого Gamer ≈ **10** вкладок (было 7). Developer остаётся **14 + 0 gamer-only**
(Library/Scenes/Captures **не** в Developer `for_mode` — они mode-specific,
как work tools Developer не в Gamer).

**Важно для `ALL`:** catalog включает все варианты enum (иконки/labels/тесты).
`for_mode` режет наборы. Shared order: `System` first; settings tail identical
in both modes (тест `developer_settings_group_matches_gamer_settings_group_order`
должен выжить).

Ширины (ориентир, исполнитель может обосновать замер):

| tab | preferred_content_width |
|---|---|
| Library | 480 |
| Scenes | 400 |
| Captures | 320 (empty) |

---

## 3. Границы слайса

### В слайсе

1. **RECON** источников списка игр + фактических id на машине.
2. **Расширение `Scene`** + `scene::activate` + персист `[last]` + builtin hub
   scene seed (если файла нет / нет gamer-сцен).
3. **Вкладки** Library / Scenes / Captures + иконки в `assets.rs` (урок T169).
4. **Library UI** — список из реального сканa, pin/recent локально, launch
   через существующий desktop Exec (как dock/launcher).
5. **Scenes UI** — список gamer-сцен, activate, create-from-app (минимально:
   id/name/app/dock defaults), delete с confirmation (§4.2 destructive).
6. **Опциональный apply `GamingModeState`** только если
   `apply_gaming_profile = true` на активируемой сцене; UI честно показывает
   состояние **из `GamingModeState::is_active`**, не из `WorkspaceMode`.
7. **QA-смок** слайса.

### Вне слайса (осознанно)

- Game Deck overlay, FPS/temps/VRAM, capture/stream, audio mixer — **слайс 6**.
- Gamepad service / controller focus — **слайс 6**.
- Steam API, playtime, achievements, CDN artwork — пока нет backend.
- Resolution / refresh control — нет safe display service.
- Вариант C внешних окон (`[scene.windows]`) — отдельный план.
- Editor / Build / Files в Gamer — не возвращаем; Gamer не второй IDE.
- Автопереключение mode при fullscreen game — smart-prompt уже есть;
  auto-`set` **запрещён** (слайс 1).

---

## 4. Global Constraints

Действуют поверх `docs/orchestration/agents/RULES.md`.

1. **Режим не переключается сам.** Новых вызовов `workspace_mode::set/toggle`
   из не-user путей — ноль. Активация **сцены** ≠ смена **режима**.
2. **Gamer ≠ GamingModeState.** UI / тосты / иконки не врут.
3. **`restore_for_mode` остаётся read-only на диске.** Пишет только
   `activate` (user path).
4. **Битый `scenes.toml` не затираем** (T164).
5. **Честность §13.** Нет обложек/часов/достижений без данных.
6. **Палитра — токены темы.** Хардкод цветов — нет.
7. **Ошибки не глушим.** `let _ = fallible()` вне закона.
8. **`assets.rs` explicit list** — новые SVG не «лежат рядом», а вшиты
   (T169).
9. **Зоны задач не пересекаются** по файлам в одной волне.
10. **Developer workbench (T176–T180) — регрессия недопустима.**

---

## 5. Порядок и волны (обновлено 2026-08-02 после T184)

```
волна 0:  T184 RECON — ПРИНЯТА
              ↓
волна 1 (ПАРАЛЛЕЛЬНО, зоны не пересекаются):
          T185 BACKEND  scene.rs — fields + activate + hub seed
          T186 FRONTEND tabs/assets/icons — Library/Scenes/Captures enum
          T187 BACKEND  applications + games.toml — categories/is_game/pins
              ↓
волна 2 (последовательно, tab/mod.rs):
          T188 FRONTEND Library UI
          T189 FRONTEND Scenes UI
              ↓
волна 3:  T190 BACKEND  apply_gaming_profile wire
              ↓
волна 4:  T191 QA       живой смок
```

**Почему T187 apps отдельно:** T184 показал Categories + games.toml как
самостоятельный backend; параллельно scene/rail без драки за файлы.

**tab/mod.rs:** T186 stubs Placeholder; T188 затем T189 — по одному arm.

---

## 6. Задачи (T-ID)

| ID | Роль | Что | Зависит | Где |
|---|---|---|---|---|
| T184 | RECON | Источники игр | — | **done** |
| T185 | BACKEND | scene fields + activate + hub seed | T184 | **active** ||
| T186 | FRONTEND | rail Library/Scenes/Captures + icons | T184 | **active** ||
| T187 | BACKEND | categories + is_game + games.toml | T184 | **active** ||
| T188 | FRONTEND | Library UI | T186, T187 | pause/BLOCKED |
| T189 | FRONTEND | Scenes UI | T185, T186, T188 | pause/BLOCKED |
| T190 | BACKEND | apply_gaming_profile wire | T185 | pause/BLOCKED |
| T191 | QA | смок слайса 5 | всё | pause/BLOCKED |

Брифы волны 1 — `tasks/active/T185|T186|T187-*.md`.

---

## 7. Файловая карта (ожидаемая)

| путь | ответственность |
|---|---|
| `crates/app/src/scene.rs` | model fields, activate, seed, last write |
| `crates/services/src/applications/types.rs` | optional `categories: Vec<String>` parse |
| `crates/services/src/applications/mod.rs` | filter helpers `is_game_entry` (pure) |
| `crates/app/src/side_panel_right/tabs.rs` | enum + for_mode + parse_id + widths |
| `crates/app/src/side_panel_right/tab/mod.rs` | TabContent arms |
| `crates/app/src/side_panel_right/tab/library.rs` | **create** Library UI |
| `crates/app/src/side_panel_right/tab/scenes.rs` | **create** Scenes UI |
| `crates/app/src/assets.rs` | include_bytes новых SVG |
| `crates/app/assets/icons/rail-library.svg` и др. | **create**, без `mix-blend-mode` |
| `crates/app/src/system_popup/gaming_mode.rs` | возможно pub apply/revert API для scene (сейчас private `apply`) — **минимальный** export, без UI popup changes если можно |
| `~/.config/chronos/scenes.toml` | user data |
| `~/.config/chronos/games.toml` | pin/recent (если не впихнуть в scenes) |

**Не трогать без нужды:** `tab/files|terminal|build|preview.rs`, Developer
`for_mode`, `workspace_mode::set` call sites.

---

## 8. Итоговая верификация слайса (T190 + архитектор)

Release-сборка, `RUST_LOG=info`, кадры `grim`, IPC
`set-workspace-mode:gamer` / `toggle-side-panel-right`. Панель — **два
приёма** (иконка → `⊞` внизу рейла).

| # | проверка | PASS criteria |
|---|---|---|
| P1 | Gamer rail | Library/Scenes/Captures видны; settings group на месте; Developer rail **без** этих трёх |
| P2 | Library | список не пуст **или** честный empty «no games detected» с причиной; launch реального .desktop |
| P3 | Scenes | builtin hub есть; activate game-сцены меняет `SceneState.active` + `[last].gamer` на диске |
| P4 | Create-from | create scene from library app → появляется в списке, activate работает |
| P5 | Gaming profile | сцена с `apply_gaming_profile=false` **не** включает GamingModeState; с `true` — включает; UI/log согласованы; revert при уходе |
| P6 | Mode switch | Developer⇄Gamer не убивает apps; panel state; last scene restore |
| P7 | Honesty | Captures = unavailable reason; нет фейкового playtime/art |
| P8 | Assets | все новые иконки **на кадре**, не пустые слоты (T169) |
| P9 | Panic | `grep panicked at` по свежему логу = 0 |
| P10 | Regress | Files/Terminal/Build/Preview в Developer живы (smoke touch) |

«Компилируется и тесты зелёные» **не** закрывает слайс.

---

## 9. Что считается провалом

- Gamer mode сам включает `GamingModeState` без scene flag / user action.
- `restore_for_mode` снова пишет диск и стирает сцены (T164).
- Library рисует Steam-обложки/часы без backend.
- Game Deck / overlay / gamepad «заодно» в этом слайсе.
- Новые вкладки в Developer rail «чтобы ALL совпал».
- Иконки в assets/ но не в `assets.rs` → пустые слоты.
- Самоприёмка / отчёт в `report-log` мимо inbox.
- Регрессия слайса 4 workbench.

---

## 10. Риски

1. **Мало Game .desktop на машине** — Library пустая. Mitigate: эвристика
   steam_app_* + честный empty + seed 1–2 manual pin в games.toml для смока.
2. **Launch Steam games** — Exec часто `steam steam://rungameid/…`; проверить
   T184, не выдумывать Proton.
3. **`apply`/`revert` private** — export без ломки popup; race с ручным toggle
   в System popup — last writer wins, log both.
4. **Рост ALL / тесты** — `all_has_fourteen_tabs` станет
   `all_has_seventeen_tabs` (14+3); не ослаблять coverage, расширять.
5. **Параллельные правки `tab/mod.rs`** — строгий порядок T186→T187→T188.
6. **QA-роль** после T181 — новый исполнитель, дисциплина RULES; фабрикаты =
   reject без «ещё заход ради захода».

---

## 11. Связь с HANDOFF / следующим

После утверждения плана:

1. HANDOFF: «слайс 5 plan approved, T184 next»;
2. бриф T184 в `tasks/active/`;
3. после T184 — брифы T185…

Слайс 6 (не планировать здесь детально): Game Deck overlay, telemetry
adapters, capture/audio, gamepad service — entry condition §4.2.

---

## 12. Утверждение

**Статус плана: УТВЕРЖДЁН 2026-08-02, вариант A** (полный: hub + scenes +
Captures empty + T189 profile wire). Пользователь: «А».

Первая задача в поле: **T184** RECON —
`docs/orchestration/tasks/active/T184-gamer-hub-recon.md`.

Архитектор. 2026-08-02.
