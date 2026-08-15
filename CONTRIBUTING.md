# Contributing

**[Dark-Ohm](https://github.com/Dark-Ohm)** · [site](https://dark-ohm.github.io/ChronOS/) · [GPUI fork](https://github.com/Dark-Ohm/Chronos-GPUI)

Не «AI-проект». Мой шелл. Агенты — чернорабочие под приёмкой; в коммитах их нет.

## Docs

`docs/HANDOFF.md` → `docs/ARCHITECTURE.md` → `docs/DECISIONS.log`. Чат и бриф агента проигрывают.

## Done

Release + живой Wayland + `grim`. Зелёный `cargo test` сам по себе — ничего.

```sh
cargo build --release -p chronos
pkill -x chronos          # не -f
RUST_LOG=info ./target/release/chronos
```

CLI: `./scripts/install-dev-cli.sh` → `chronos-rebuild && chronos-stop && chronos-start`  
Гайд: [`docs/dev-cli.md`](docs/dev-cli.md)

## Code

- `let _ = fallible` — запрещено (`?` / `.log_err()` / match).
- Workspace lints: `[lints] workspace = true` на новых крейтах.
- Коммент = *why*, не пересказ строки.
- Bleeding-edge deps. Чужие пины не тащить.

## Git

`area : what` · named `git add` · `git diff --staged` · без AI-trailer · `reference/` gpui-shell не коммитить.

## Skills

Proof-ссылки (`file:line`) в скиллах — гейт, не пожелание. `./skills/check-proofs.sh`
гоняется в CI (job `skill-proofs`, push/PR) и pre-commit хуком локально; битый реф =
фейл. Ссылки на внешние деревья (Zed upstream, Hermes checkout, philip, fable-примеры,
плейсхолдеры writing-plans) — в allowlist скрипта, отчитываются как `EXT`, прогон не валят.

Pre-commit хук (проверяет staged `SKILL.md` / `*.eval.md` / `references/*.md`), активация один раз на клон:

```sh
git config core.hooksPath scripts/git-hooks
```

Отключить: `git config --unset core.hooksPath`. Проверить вручную весь vault:
`./skills/check-proofs.sh` (exit 0 = чисто). В CI форк `Chronos-GPUI` клонируется
в sibling `../Source` (best-effort): при успехе fork-ссылки проверяются строго,
при провале — деградируют в informational. Корни, которых нет на свежем раннере,
тоже informational: `reference/` (gitignored снапшоты). Кит
`gpui-component` живёт в `../Source/gpui-component/` (тот же форк, что
`gpui`) — ссылки `Source/gpui-component/…` строгие, если `../Source`
склонирован. Коммитящиеся корни (`crates/…`, `docs/…`, `skills/…`)
строгие всегда.

## Plugins

`crates/plugins/<name>/{manifest.toml,init.luau}` · id = путь каталога.

## PR

Не переизобретай то, что уже в `docs/DECISIONS.log`. Вопрос лучше, чем 400 LOC мимо архитектуры.

Apache-2.0 — `LICENSE` / `NOTICE`.
