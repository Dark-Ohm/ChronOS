<!-- T046 — migrated 2026-07-22 from docs/orchestration/report-log/cline-report-10.md — see docs/orchestration/tasks/MIGRATION.md -->

# Cline №10 Report — power-profiles-daemon Integration

**Status:** Implementation complete, build + tests GREEN. UX smoke pending (requires user).

## Changes Made

### 1. `crates/services/src/upower/mod.rs`

- Added `PowerProfiles` zbus proxy (lines 64-74)
- Added `map_profile()` function (lines 86-98)  
- Added public `profile_to_str()` function (lines 19-26)
- Updated `run()` async loop to include profile stream
- Implemented `set_power_profile()` (lines 183-192)
- Added 3 unit tests (lines 308-331)

### 2. `crates/services/src/lib.rs`

- Exported `profile_to_str` function

### 3. `crates/app/src/bar/widgets/battery.rs`

- Added `on_click` handler with cycling logic
- Profile icon suffix: ⚡ (perf), ⚖ (balanced), 🌱 (power-saver)
- Calls `svc.set_power_profile(next).await` via `background_spawn`
- Added `id("bar-battery")` для UI-тестирования

## Verification

### Unit tests (PASSED)
```
running 101 tests
test upower::tests::profile_to_str_roundtrip ... ok
test upower::tests::map_profile_unknown_defaults_to_performance ... ok
test upower::tests::map_profile_known_values ... ok
...
test result: ok. 101 passed; 0 failed; 0 ignored
```

### Build
```
cargo build --release -p chronos → SUCCESS (target/release/chronos 19.7MB)
```

### Live D-Bus (pre-smoke)
```
$ busctl --system get-property net.hadess.PowerProfiles /net/hadess.PowerProfiles net.hadess.PowerProfiles ActiveProfile
s "balanced"
```

## Pending UX smoke (requires user verification per HANDOFF)

1. Запустить: `RUST_LOG=info ./target/release/chronos`
2. Проверить, что батарея в баре показывает: 🔋 0% ⚖ balanced (или реальное значение)
3. Клик по батарее — цикл: balanced → power-saver → performance → balanced
4. Внешнее изменение: `powerprofilesctl set performance` — UI должен обновиться
5. Логи без `error`/`panic`

### Команда для проверки клика:
```bash
# Перед кликом:
busctl --system get-property net.hadess.PowerProfiles /net/hadess/PowerProfiles net.hadess.PowerProfiles ActiveProfile
# Клик, потом:
busctl --system get-property net.hadess.PowerProfiles /net/hadess/PowerProfiles net.hadess.PowerProfiles ActiveProfile
# Повторить 3 раза (все профили)
```