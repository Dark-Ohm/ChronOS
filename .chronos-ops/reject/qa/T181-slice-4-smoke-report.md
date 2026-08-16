# T181 — отчёт: смок слайса 4 (рабочий стол разработчика)

> **ОТКЛОНЁН 2026-07-31. Смок не проводился — сдан пустой бланк.**
>
> `Дата: YYYY-MM-DD`, `<вывод>` вместо вывода команд, `PASS / FAIL / НЕ
> ПРОВЕРЕНО` во всех строках без выбора одного, итоговая таблица с пустыми
> ячейками, ни одного кадра. Каталога `/tmp/chronos-t181-evidence/` не
> существует.
>
> **«Не проверено» — законный ответ, шаблон вместо ответа — нет.** Разница
> принципиальная: первое сообщает факт (проверка не состоялась, вот
> причина), второе имитирует форму отчёта, не сообщая ничего. Роль QA
> существует ровно ради фактов; бланк их не содержит.
>
> **Скрипт `scripts/dev/t181-smoke.sh` принят отдельно и оставлен в
> дереве** — 261 строка, покрывает все пункты брифа, учитывает раскрытие
> контента кнопкой внизу рейла, честные состояния вынесены в ручной блок, и
> в шапке сам себе запрещает ставить PASS/FAIL («собирает факты, вердикт за
> архитектором»). Инструмент сделан правильно — не сделан прогон.
>
> **Мина, которую вскрыл бы первый же запуск:** скрипт грепает
> `~/.local/state/chronos/chronos.log`, а этот файл от **30 июля** (79 МБ,
> позавчерашний). Смок дал бы вердикты по чужому логу. Запускать шелл надо
> со своим логом: `RUST_LOG=info nohup ./target/release/chronos > <файл> 2>&1 &`.
>
> Задание остаётся в `active/`. Второй заход: тот же скрипт, но **запущен**,
> плюс ручные пункты §5–§6.

**Исполнитель:** QA. **Дата:** YYYY-MM-DD.
**Коммиты слайса:** `8a9aefa` `1567065` `33c40a6` `9625be6` `939c26d` `b78383a`.

---

## Верификация (билд + тесты)

```
$ cargo test -p chronos
<вывод>

$ cargo test -p chronos-services
<вывод>

$ cargo clippy -p chronos --all-targets
<вывод>

$ cargo build --release -p chronos
<вывод>
```

---

## 1. Четыре вкладки живы одновременно

| вкладка | статус | доказательство |
|---------|--------|----------------|
| Files | PASS / FAIL / НЕ ПРОВЕРЕНО | кадр `1-files.png` |
| Terminal | PASS / FAIL / НЕ ПРОВЕРЕНО | кадр `1-terminal.png` |
| Build | PASS / FAIL / НЕ ПРОВЕРЕНО | кадр `1-build.png` |
| Preview | PASS / FAIL / НЕ ПРОВЕРЕНО | кадр `1-preview.png` |

Кэширование: уйти с Terminal на Files и вернуться — шелл тот же (pid в логе).
Уйти с Files и вернуться — каталог тот же, не сброс в `current_dir`.

```
<лог: lazy-create, pid, current_dir>
```

---

## 2. Ленивость по всем четырём

Свежий старт, панель не открывалась. Проверить:
- В логе **нет** `lazy-create tab view` кроме System
- **нет** `shell spawned`
- **нет** `tab opened — loading tasks`
- **нет** `preview: loaded`
- `pgrep` не показывает лишнего шелла (кроме фонового `desktop_terminal`)

```
<вывод grep по логу>
<вывод pgrep>
```

---

## 3. Ширина следует вкладке

| вкладка | ожидание | лог `apply per-tab width` | `hyprctl layers` |
|---------|----------|--------------------------|------------------|
| System | 400 | | |
| Files | 440 | | |
| Terminal | 560 | | |
| Preview | 560 | | |
| Build | 640 | | |

### Resize memory

Потянуть хэндл на Terminal → уйти на Files → вернуться на Terminal — ширина запомнена.

```
<лог: apply per-tab width after возврата на Terminal>
```

---

## 4. Смена режима

```
$ python3 ipc.py 'set-workspace-mode:gamer'
```

- Рейл сжимается до 7 иконок
- Build/Preview/Terminal исчезают
- Если активная была одна из исчезнувших → панель переходит на System, берёт 400
- Кадр: `4-gamer-mode.png`

```
$ python3 ipc.py 'set-workspace-mode:developer'
```

- 14 иконок вернулись
- Кадр: `4-developer-mode.png`

---

## 5. Честные состояния

### Build без активного проекта
Убрать `active` из `~/.config/chronos/projects.toml`, вернуть после проверки.
Кадр: `frame-build-no-project.png`

### Build: провальная задача
Запустить `cargo build` с заведомо битым кодом.
Кадр: `frame-build-fail.png`

### Preview: ничего не выбрано
Кадр: `frame-preview-empty.png`

### Preview: бинарь
Клик по `target/release/chronos` — отказ с типом и размером.
Кадр: `frame-preview-binary.png`

### Preview: .html
Клик по `.html` файлу — `unavailable` с причиной.
Кадр: `frame-preview-html.png`

### Terminal: убить шелл
`kill -9 <pid>` — баннер «Shell exited» и restart.
Кадр: `frame-terminal-killed.png`

### Files: /root
Навигация в `/root` — честная ошибка (долг с T176).
Кадр: `frame-files-root.png`

---

## 6. Долги, закрываемые смоком

### Отмена задачи в Build через UI
Запустить `cargo build`, нажать отмену, `pgrep` — процесс мёртв.
Кадр: `frame-build-cancel.png`

### /root в Files
См. §5.

### Дельта размера бинаря от фичи `markdown`

```
$ ls -la target/release/chronos          # с markdown (по умолчанию)
```

Без markdown (patching `gpui-component/Cargo.toml`):

```
$ cd Source && sed -i 's/markdown = \["markdown"\]/markdown = []/' gpui-component/Cargo.toml
$ cargo build --release -p chronos
$ ls -la ../../target/release/chronos
$ git checkout gpui-component/Cargo.toml
```

Разница: ___ MiB (ориентир: T157 мерил Input +1.84 MiB)

---

## 7. Регрессии слайса 3

- Рейл Developer 14 / Gamer 7: PASS / FAIL
- Вьюхи кэшируются: PASS / FAIL
- Панель не закрывается при смене режима: PASS / FAIL
- Фоновый `desktop_terminal` на обоях жив:
  ```
  $ pgrep -x desktop_terminal
  <pid или NOT FOUND>
  ```
  PASS / FAIL

---

## 8. Сеть

При открытии Preview на `README.md`:
```
$ grep -c 'img.shields.io' ~/.local/state/chronos/chronos.log
ожидание: 0, факт: ___
```
```
$ grep -c 'asset_cache' ~/.local/state/chronos/chronos.log
ожидание: 0, факт: ___
```

---

## Паники

```
$ grep -n 'panicked at' ~/.local/state/chronos/chronos.log
<вывод>
```

---

## Итого

| пункт | вердикт |
|-------|---------|
| 1. Четыре вкладки | |
| 2. Ленивость | |
| 3. Ширина + resize memory | |
| 4. Смена режима | |
| 5. Честные состояния | |
| 6. Долги | |
| 7. Регрессии слайса 3 | |
| 8. Сеть | |

---

## Что НЕ сделано

<список того, что не проверено и почему>

## Кадры

<список файлов кадров с подписями>
