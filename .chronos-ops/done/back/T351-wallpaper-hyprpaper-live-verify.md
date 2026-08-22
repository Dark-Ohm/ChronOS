---
ticket: T351
role: back
status: active
tags: [chronos-ops, back, active]
---

# T351 — BACKEND: hyprpaper — живой Set, не только argv-юнит

**Роль:** BACKEND. **P2.** Дочерний T349 (принят 2026-08-22, канон —
`.chronos-ops/checkpoint/ARCHITECTURE.md` §19). Отчёт T349 честно раскрыл:
hyprpaper собран (`crates/services/src/wallpaper/backends.rs`
`hyprpaper_argv`/`apply_hyprpaper`) и юнит-протестирован
(`hyprpaper_argv_matches_reference`), но живьём НЕ гонялся.
**Зона:** только живая проверка + фикс багов, если найдутся — код
`backends.rs` (функции `apply_hyprpaper`/`hyprpaper_argv`/`monitor_names`)
уже написан в T349, не переписывать с нуля.
**Не параллелить с T352/T353** — один файл (`backends.rs`), делать
последовательно.

## Зачем

`hyprpaper` — единственный движок в пятёрке, который живёт как
**персистентный systemd-демон** (не restart-based, как остальные три) и
управляется через `hyprctl` (IPC самого Hyprland), а не напрямую свой
CLI/сокет. Это самый вероятный источник расхождения между тем, что
написано в `apply_hyprpaper`, и тем, что реально происходит: демон может
не подняться через `systemctl --user start hyprpaper` на этой машине
(fallback на голый спавн уже есть в waytrogen-источнике, но **в ChronOS
не портирован** — T349 сделал только IPC-Set, не bootstrap демона; см.
`.chronos-ops/reports-log/recon/T348-wallpaper-backend-control-surfaces-report.md`
раздел «hyprpaper», абзац «Демон»).

## Что сделать

1. Проверить, поднимается ли hyprpaper демон на этой машине:
   `systemctl --user start hyprpaper` (или просто `hyprpaper &` если
   юнита нет — сверить оба пути).
2. **Решено архитектором (2026-08-22): вариант 1, lazy bootstrap.**
   `apply_hyprpaper` сейчас НЕ бутстрапит демон — только шлёт `hyprctl
   hyprpaper wallpaper`. Добавить bootstrap на Set: `pidof hyprpaper` →
   `systemctl --user start hyprpaper` → bare-spawn fallback (голый
   `hyprpaper &`, если юнита systemd нет) → bounded readiness poll →
   затем IPC-Set. Зеркалит awww `ensure_daemon_forced` и сам
   waytrogen-источник (`changers/hyprpaper.rs`, см. T348-отчёт раздел
   «hyprpaper», абзац «Демон» — pgrep/systemctl/bare-spawn уже описаны
   там с путём:строкой, портировать оттуда, не изобретать). Zero-config
   UX — асимметрия «awww поднимается сам, hyprpaper нет» была
   недосмотром T349, не сознательным решением.
3. Живой Set на одном мониторе + на «All» (цикл по `hyprctl monitors`,
   `apply_hyprpaper`'s `monitor_names()`) — картинка, не видео (hyprpaper
   не умеет video, `Backend::Hyprpaper.supports_video() == false`).
4. Доказательства: `hyprctl layers` (или `hyprctl clients`, смотря как
   hyprpaper регистрирует слой) + `grim` кадр с реально другой картинкой
   на фоне (не чёрный/не старый awww-кадр), лог с `INFO`/`WARN` вокруг
   Set.
5. `cargo test -p chronos-services wallpaper` зелёный после любых правок.

## Готово когда

Живой hyprpaper-Set подтверждён кадром + логом на одном мониторе и на
«All»; демон либо сам поднимается, либо честно ошибается с понятным
сообщением (архитектор решил который); тесты зелёные.

**Отчёт:** `.chronos-ops/reports-fresh/T351-wallpaper-hyprpaper-live-verify-report.md`
