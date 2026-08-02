# ChronOS — product cut (канон)

**Обновлено:** 2026-08-02. **Статус:** утверждено пользователем в диалоге
(не спека Shell-IDE §14 целиком).

> Красивый быстрый Hyprland DE-шелл + постоянный агент слева + тонкий
> workbench справа + системные настройки. Не IDE. Не Game OS.
> Подписка Chronos agent / BYO API·ACP. Со временем — кандидат DE в
> CachyOS installer. Поставка **с dotfiles** Hyprland.

При расхождении с `docs/superpowers/specs/2026-07-30-adaptive-…` и планами
слайсов 5–8 **побеждает этот файл** для product scope. Спека остаётся
историей/идеями, не backlog’ом.

---

## 1. Пользователь и обещание

- Поставил → красиво → агент слева всегда отвечает / правит конфиг / помогает
  с файлами / research / vault (tools, не раздувание бинаря).
- Справа видно файлы и то, что агент трогает; можно **поправить руками**.
- Системные настройки — как у нормальной ОС (звук, дисплей, сеть, питание,
  шелл, бинды).
- Легко поставить / снести / закастомить. Масса, не «платформа для агентов».

---

## 2. В бинаре `chronos` (тонкий DE)

### Chrome
Bar, dock, notifications, OSD, launcher (лёгкий), theme, outputs/pult,
workspace density modes (Developer/Gamer = **плотность chrome**, не два OS).

### Левая панель — Agent (ядро продукта)
- ACP multi-turn, model, tools, errors human-readable.
- Chronos subscription route **или** BYO API / ACP endpoint.
- Кнопка **«Следить за агентом»** (follow): правая панель синхронизируется
  с тем, что агент делает (открытый файл / diff / activity) — UX как везде,
  не R&D.
- ACP settings: **добавление / удаление ACP-агентов** (не « entие LSP»).

### Правая панель — workbench (минимум)

| Вкладка | Роль |
|---|---|
| **Files** | дерево; клик → открыть в Editor |
| **Editor** | бывший Preview **+ правка**: смотреть и **редактировать** то, что
  агент изменил / что открыл пользователь. Не полноценный IDE-editor. |
| **System** | настройки шелла + OS (бывший Editor settings → **System settings**). |
| **Library** (Gamer) | список игр + launch + pin — **оставить**, dogfood приятный. |
| **Captures** (optional later) | не «продукт capture pipeline» — **лист папки**
  скриншотов (`~/Pictures` / hyprshot dir). |
| **Hyprland binds** | нужен, **после** работы над shippable hyprland dotfiles. |
| **ACP agents** | add/remove/configure ACP endpoints (может жить в System или
  отдельной вкладкой settings-группы — решить при UI). |

Live «чем агент занят» в правой панели при follow — **да**, lightweight
(activity/stream), не отдельный IDE.

### Поставка
- Бинарь + **dotfiles Hyprland** (минимальный рабочий профиль).
- Install/uninstall clean (AUR → later CachyOS DE list).
- Кастом: тема + layout, мало конфиг-файлов, документированных.

---

## 3. Вне бинаря (отдельное ПО / agent tools)

| Не вшивать | Куда |
|---|---|
| Full FM (Chronos-FM) | отдельное app; шелл launch/focus |
| Full terminal IDE | kitty/foot / desktop_terminal surface optional |
| Build/test orchestration | IDE / CLI |
| Game Deck / telemetry / scenes manager | **out** |
| PDF/Obsidian deep/research engines | agent tools + внешние apps |
| LSP / MCP panels | **не сейчас**; MCP позже если agent backend |
| Hindsight stack | infra, не UI шелла |

---

## 4. Явный park / kill (срез 2026-08-02)

| Было в слайсе 5 / рейле | Решение |
|---|---|
| **Scenes** (per-game scene UI, T189) | **KILL product path** — «сцены нахуй не нужны». Код `scene.rs`/seed может жить
  dormant; UI/T189 не делать. |
| **Captures** как product | **не проблема** — later: list screenshot folder, не backend записи |
| LSP / MCP settings tabs | **не нужны** сейчас — убрать из default rail / hide |
| Editor (empty IDE tab) | **убрать**; Preview → **Editor** (view+edit) |
| Editor settings | → **System settings** |
| **Terminal** (T192 cut) | **вернуть later** — вырезали зря; отдельный T, не блокер Editor |
| Gamer Library | **KEEP** |
| apply_gaming_profile / T190 | optional advanced; не blocker DE |
| T191 gamer QA slice | low priority vs agent+chrome |

---

## 5. Ближайшие фазы (приземлённо)

1. **Chrome daily + hyprland dotfiles ship set** (binds после dotfiles).
2. **Agent left: reliability + follow button + right live activity.**
3. **Files + Editor (preview+edit)**; убрать мёртвый Editor tab / rename Preview.
4. **System settings** consolidation; ACP agents CRUD.
5. **Rail cleanup:** default icons only for real tabs; experimental hidden.
6. **Packaging** install/uninstall; dogfood → AUR → CachyOS conversation.

---

## 6. Анти-цели (чтобы снова не расползтись)

- Не второй Zed / VS Code.
- Не Steam Big Picture / Game Deck в ядре.
- Не 14 empty rail icons «как в спеке».
- Не тащить в бинарь то, что агент может вызвать tool’ом.
- Не оркестрация ради оркестрации — product first.

---

Архитектор + пользователь. 2026-08-02.
