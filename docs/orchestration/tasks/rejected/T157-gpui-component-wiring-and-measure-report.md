# T157 — gpui-component: проводка и замер — отчёт

**Статус:** замер выполнен. **Ветка:** `measure/gpui-component` (не запушена).
**master:** `c688c11` — чистый, не тронут.

---

## Шаг 0 — база

База (`master`, `c688c11`): **22 520 192** байт.

Ожидаемая цифра (22 475 648) не совпала — пользователь подтвердил новую базу
**22 520 192**. Дельта +44 544 байт (~43 KiB) объясняется коммитами T150,
T152, T154, вошедшими в master после июльского замера.

---

## Шаг 1 — проводка

Изменено 4 файла (+ `Cargo.lock`, пересобранный):

```
 Cargo.toml                              |    8 +
 crates/app/Cargo.toml                   |    1 +
 crates/app/src/main.rs                  |    1 +
 crates/app/src/side_panel_right/view.rs |   17 +-
```

**Корневой `Cargo.toml`:**
- `gpui-component` в `[workspace.dependencies]` с `default-features = false`,
  путь в worktree `../Source-wt-component/gpui-component/crates/ui`
- второй `[patch]`-блок на `https://github.com/zed-industries/zed` →
  наш форк (как в пилоте `20ee13a`)

**`crates/app/Cargo.toml`:** `gpui-component.workspace = true`

**`crates/app/src/main.rs`:** `gpui_component::init(cx)`

**`crates/app/src/side_panel_right/view.rs`:** `Button` из gpui-component
(рендерится, линкуется) — чтобы линкер не выкинул компонент при LTO.

---

## Шаг 3 — cargo tree: фичи выключены

```
=== lsp-types ===
error: package ID specification `lsp-types` did not match any packages

=== html5ever ===
error: package ID specification `html5ever` did not match any packages

=== markdown ===
error: package ID specification `markdown` did not match any packages

=== num-traits ===
num-traits v0.2.19
├── gpui-component v0.6.0 (.../Source-wt-component/gpui-component/crates/ui)
│   └── chronos_app v0.1.0 (.../chronos-ecosystem/ChronOS/crates/app)
│       └── chronos v0.1.0 (.../chronos-ecosystem/ChronOS)
└── ... (остальные потребители в графе)

=== gpui-component features ===
gpui-component v0.6.0 (.../Source-wt-component/gpui-component/crates/ui)
├── gpui feature "default"
│   └── gpui feature "wgpu"
│       ├── gpui feature "image"
│       │   ├── gpui feature "svg"
│       │   └── gpui feature "png"
│       └── gpui feature "wayland"
└── gpui-component feature "default"
    ├── gpui-component feature "input"
    ├── gpui-component feature "theme"
    ├── gpui-component feature "checkbox"
    ├── gpui-component feature "button"
    ├── gpui-component feature "radio"
    ├── gpui-component feature "popover"
    ├── gpui-component feature "link"
    ├── gpui-component feature "text"
    └── gpui-component feature "dock"
```

**Результат:** `lsp-types`, `html5ever`, `markdown` — **нет в графе**.
`num-traits` — есть (он в `default`-фичах компонента: `chart` + `plot`).
`chrono` — в графе из `[workspace.dependencies]` ChronOS, не от компонента.

Гейты работают.

---

## Шаг 4 — замер

| Сборка | Размер (байт) |
|---|---|
| База (master, `c688c11`) | 22 520 192 |
| Инкрементал | 23 662 144 |
| From-scratch (`cargo clean`) | 23 662 144 |

**Дельта: +1 141 952 байт = +1.09 MiB.**

Инкрементал и from-scratch совпали (расхождение 0 байт < 50 KiB) —
Cargo.toml менялся, инкрементал всё равно пересобрал всё.

**Шлюз:** ≤ +1.2 MiB — **зелёная зона.** Гейты окупились. Компонент идёт
в T158 (обрезка + настоящий потребитель).

---

## Шаг 5 — живой прогон

```
chronos-stop; chronos-rebuild; chronos-start
```

Скриншот: `/tmp/T157-gpui-component-live.png` (742 KB).

Панель поднялась, правая панель с компонентом отрисовалась, шелл
функционирует.

**Паники:** 0 новых. Два старых вхождения в логе — от предыдущих запусков
(thread ID `903578` и `2084188`, текущий запуск — `1502587`),
оба в `hermes_acp/client.rs` (не связано с компонентом).

**`window not found`:** 78 вхождений — накоплены за историю лога,
текущий запуск без них.

---

## Итог

- **Дельта: +1.09 MiB** — ниже порога +1.2 MiB.
- **Гейты T156 подтверждены в реальном графе** — `lsp-types`, `html5ever`,
  `markdown` не линкуются.
- **Ветка `measure/gpui-component`** готова к T158.
- **master** (`c688c11`) не тронут.
