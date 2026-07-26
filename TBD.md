# TBD — хвосты и хотелки

> Живой список **некритичного**: polish, wishlist, отложенные идеи.
> Не замена `HANDOFF.md` (оперативка) и не `orchestration/tasks/` (T-ID).
> Когда пункт созрел → бриф T-ID или вычёркивание с датой/коммитом.
>
> **Обновлено:** 2026-07-26 (T129 active — motion; theme/panels baseline closed)

## Правила

- Сюда — только то, что **не блокирует daily driver** прямо сейчас.
- Критика / инциденты / «ломает светлую тему» — в HANDOFF или сразу T-ID.
- Формулировка: *что болит* + *где* (путь/поверхность), без «красиво бы».
- Закрыл → строка `~~…~~ (YYYY-MM-DD, commit)` или удали с пометкой в git log.

---

## Theme / chrome polish

- [ ] Right panel **art well** на Light: чистый чёрный дырой на pageBg; soft well или dark-only black.
- [ ] **Mpris disabled** (mute/prev/next): на light opacity+muted почти невидимы.
- [ ] **Rail inactive icons** на wallpaper (rail-only): muted тонет; чуть `text.secondary` / opacity.
- [ ] **Net spectrum bars** на light: серый слабее CPU/RAM; secondary или info-tint.
- [ ] **Permission card** elevation vs body — mockup gradient, сейчас плоско.
- [ ] Light-pass остальных chrome: **tray / project switcher / updates popup** (HANDOFF: «в светлой ещё не смотрели»).
- [ ] `surfaces::content` — light=dark=`bg.primary`, helper noop; схлопнуть или дать роль.
- [ ] Spectrum dark: три mockup `rgb()` (#89dceb / #89b4fa / #f9e2af) — оставить pixel-parity или токены с accent-table.

## Side panels

- [ ] **Resize / exclusive race**: `state.width` vs реальный layer `w=54` (last_resized без реального set_size).
- [ ] Hover-strip peek open/close мигает у края экрана.
- [ ] Left: **jank dropdown** agent switcher (долг после T108).
- [ ] Left: **ghost-trail** (форк, #8 / #8-bis).
- [ ] Left empty thread UX: «No messages yet» + огромная пустота vs плотная правая.
- [ ] T126/T127 live ACCEPT still open (код есть).
- [ ] T115 Files tab — **PAUSE** (бриф ужесточён).

## Wallpaper / waytrogen (T133 caveats)

- [ ] Live smoke: grim окна waytrogen launched из шелла.
- [ ] Resync при закрытии gallery без GUI Next.
- [ ] Next без GUI path (edge cases).

## Visual depth queue

- [x] ~~T129 — panel/popup enter-exit~~ → **active**  
      `orchestration/tasks/active/T129-panel-popup-enter-exit.md` (2026-07-26)
- [ ] T130 — toast enter/exit (после T129).
- [ ] T131 — fork: 3D scene primitive + example.
- [ ] T132 — один 3D demo surface в шелле.
- [ ] T128 elevation report prose / optional grim archive.

## Agent / ACP

- [ ] Live round-trip models после prompt.
- [ ] Второй ACP backend в реестре (сейчас только Hermes).
- [ ] Composer: gpui-component TextInput vs homemade (C-2 note).

## Updates popup

- [ ] T118 caveats: spinner static / staircase filter; live smoke long list scroll.
- [ ] T119 live smoke multi-select upgrade (PENDING).

## Infra / docs

- [ ] HANDOFF sync: theme wire left+right, surface roles (`5de7b31`, `091187c`, `8e8043e`).
- [ ] Daily smoke checklist: Super+Shift+T light+dark, both panels content open, grim.
- [ ] `unwrap`/`expect` cleanup (~163 warn) — по касанию, не разом.
- [ ] `let _ = fallible` hygiene — по касанию файлов.

## Wishlist (идеи, без срока)

- [ ] Active window title в right header (сейчас static `"kitty"`).
- [ ] Permission card → реальный backend (сейчас mock).
- [ ] Switch user (power row) — сейчас disabled.
- [ ] Per-tab content beyond System (Files/Editor/… — coming soon).
- [ ] Theme: accent table per scheme (сейчас accent общий #007acc).
- [ ] Optional: ChronOS light → soft-hint Zed theme (out of scope шелла).

---

## Закрыто недавно (для памяти)

- ~~Right panel hardcoded mocha / light не применялась~~ → Theme wire + surface roles (2026-07-25…26: `091187c`, `5de7b31`).
- ~~Left panel hardcoded mocha~~ → `8e8043e`.
- ~~Theme toggle Super+Shift+T + theme.toml~~ — в дереве.
