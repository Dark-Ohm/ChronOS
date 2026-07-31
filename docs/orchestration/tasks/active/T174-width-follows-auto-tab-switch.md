# T174 — ширина не следует за вкладкой при автоматической смене на System

**Статус:** active. **Роль:** FRONTEND. Общие правила —
`docs/orchestration/agents/RULES.md`.

Идёт одна. Задача маленькая и точечная — это хвост слайса 3, найденный
QA-смоком T173.

**Зона (твоя):**
- `crates/app/src/side_panel_right/view.rs`

**НЕ трогать:** `tabs.rs` (`preferred_content_width` верна, менять значения
не надо), `mod.rs` (`ensure_content_width` уже принимает `target`),
`tab/**`, `rail.rs`, `dock/**`, `scene.rs`, `assets/**`.

**Отчёт:** `docs/orchestration/tasks/report/T174-width-follows-auto-tab-switch-report.md`.

---

## Дефект — измерен живьём, не гипотеза

Нашёл QA в T173. Воспроизведение:

1. `set-workspace-mode:developer`, панель открыта;
2. перейти на **Editor** — слой `side_panel_right` становится `w=560`
   (`preferred_content_width(Editor) == 560`);
3. `set-workspace-mode:gamer` — Editor из набора Gamer уходит.

Ожидание: активной становится System, `preferred_content_width(System) == 400`,
значит слой должен стать `w=400`.

Факт из отчёта T173:

```
P1C Developer Editor: DP-1 2000 560 1410
P1D Gamer fallback:   DP-1 2000 560 1410
```

Ширина **залипла на 560**. Панель при этом не закрывается и фокус уходит на
System верно — ломается ровно ширина.

## Причина — подтверждена по коду архитектором

`crates/app/src/side_panel_right/view.rs:263-272`:

```rust
if !rail_tabs.contains(&self.active_tab) {
    tracing::info!(
        was = self.active_tab.label(),
        "side_panel_right: active tab not in mode set → System"
    );
    self.active_tab = PanelTab::System;
    self.ensure_tab_view(PanelTab::System, cx);
}
```

Здесь меняется `active_tab` и создаётся вьюха — но **ширина не
применяется**. Вся работа с шириной, добавленная в T171, живёт в
`on_tab_select`, а этот путь через неё не проходит: вкладку меняет не
пользователь, а сам `render` при смене режима.

То есть T171 покрыла путь «пользователь кликнул по вкладке» и не покрыла
путь «вкладка ушла сама». Это дырка в моей приёмке T171, не твоя вина —
принимай как факт и чини.

## Что делаем

Свести оба пути к одной механике: смена активной вкладки — **откуда бы она
ни пришла** — применяет ширину этой вкладки.

Форму выбираешь ты, но требования:

- fallback на System применяет `active_tab_width(System)` через
  `ensure_content_width`, как это делает `on_tab_select`;
- **guard на `dock_content == false` сохраняется** — в rail-only ширина не
  применяется, панель остаётся `RAIL_ONLY_WIDTH`. Тот же guard, что в
  `on_tab_select` (`content_open`);
- ручной ресайз не теряется: если у System в `tab_resize_memory` лежит своя
  ширина, применяется она, а не `preferred` — за это и отвечает
  `active_tab_width`;
- **лишнего `window.resize()` быть не должно.** В коде есть
  `last_resized_width` и `last_exclusive_zone` ровно для этого; третий
  счётчик не заводить.

**Ловушка, о которой надо думать отдельно:** правка идёт **внутри
`render()`**. Мутация глобала `SidePanelRightState` из рендера уже
происходит (`ensure_tab_view` рядом), но добавь её так, чтобы не получить
бесконечную перерисовку: применил ширину → следующий кадр видит, что
вкладка уже в наборе, и ничего не делает. Если увидишь, что рендер начинает
крутиться — **не подпирай флагом, опиши в отчёте**, значит место для правки
выбрано неверно и надо переносить логику в обработчик смены режима.

## Тесты

Обязателен тест именно на **этот** путь — его сегодня нет, поэтому дефект и
доехал до живого прогона:

- активна вкладка, которой нет в наборе режима → после разрешения набора
  активной стала System **и ширина равна `preferred_content_width(System)`**;
- то же, но у System есть запись в `tab_resize_memory` → применяется она;
- `dock_content == false` → ширина осталась `RAIL_ONLY_WIDTH`.

`#[gpui::test]` с `TestAppContext` в форке есть (`Source/gpui/src/test.rs`),
им уже написаны тесты T168 и T171 — бери их за образец. Тест, дублирующий
логику продукта внутри себя, тестом не считается.

## Верификация

```
cargo test -p chronos
cargo clippy -p chronos --all-targets
cargo build --release -p chronos
```

**Живой прогон обязателен** — дефект найден живьём, значит и закрываться
должен живьём. Точный сценарий из T173:

```python
import socket
def ipc(m):
    s = socket.socket(socket.AF_UNIX); s.connect("/run/user/1000/chronos.sock")
    s.sendall(m.encode()); s.close()

ipc("set-workspace-mode:developer")
ipc("toggle-side-panel-right")
```

Развернуть контент (клик по вкладке рейла или захват хэндла на левом краю),
перейти на **Editor**, замерить, переключить в Gamer, замерить снова:

```bash
hyprctl layers -j | python3 -c "
import json,sys
for out,v in json.load(sys.stdin).items():
    for lvl,l2 in v['levels'].items():
        for l in l2:
            if l.get('namespace')=='side_panel_right': print(out, l['x'], l['w'])"
```

**Ожидание:** `w=560` на Editor → `w=400` после переключения в Gamer.
Приложи обе строки целиком и кадры до/после — на кадре после переключения
не должно быть сквозной полосы слева.

Рейл — крайние 54 px справа (`x = 2506…2560`), иконки от `y ≈ 57` с шагом
≈ 40. `ydotool` берёт **половинные** координаты (подтверждено семь раз),
сокет `YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket`.

Лог грепать **целиком**: `grep -n "panicked at" лог`, не по своей зоне.

Пультовый вывод бывает занят фуллскрин-игрой — занят, значит «не
проверено» с причиной, в игру не лезь.

## Коммит

Ветка от актуального `master`. Сообщение: `side_panel_right : ширина следует
за вкладкой и при автоматической смене на System (T174)`. Без AI-трейлеров,
`git diff --staged` глазами, поимённый `git add`. **Коммитишь ты.**
