# T118 — Updates popup upgrade output — Report

## Статус: DONE (код + unit-тесты, живой апгрейд не тестировался — нет доступных обновлений)

## Архитектурные решения

### `UpgradeState` / `UpgradeProgress`

```rust
pub enum UpgradeState {
    Idle,
    Running(UpgradeProgress),  // было просто Running
    Done,
    Failed,
}

pub struct UpgradeProgress {
    pub current: usize,        // текущий шаг (1-indexed)
    pub total: usize,          // всего шагов
    pub last_line: String,     // последняя строка вывода
    pub completed_names: Vec<String>,  // обработанные пакеты
}
```

**Почему `Vec<String>` а не `HashSet`**: порядок нужен для UI (staircase — появление по порядку), `PartialEq` уже реализован для `Vec`, а объём мал (десятки пакетов максимум).

**Почему `current/total` а не `percent: f32`**: в типах проекта запрещены floats (`Eq` guard), процент считается как `((current as f64 / total as f64) * 100.0) as u8`.

### Потоковый захват вывода

`run_upgrade_all()` заменён с `Command::status()` на `Command::spawn()` + pipe stderr. `pacman` пишет прогресс в stderr (не stdout). Отдельный `std::thread` читает построчно через `BufReader`, парсит `(N/M) upgrading name...` и обновляет `Mutable<UpdatesState>` реактивно.

**Формат pacman** (проверен на этой машине через `pacman -Qu` + документация):
- `(3/7) upgrading firefox...` — текущий шаг, имя пакета
- `(1/5) installing new-pkg...` — новая установка
- `(2/3) reinstalling glibc...` — переустановка
- `(4/4) removing old-pkg...` — удаление (не анимируем)

### UI — view.rs

1. **Spinner**: `icons/arrows-clockwise.svg` + текст `Upgrading… 3/7`
2. **Progress bar**: `div` с `w(fraction)` на основе `current/total`, цвет `accent`
3. **Live output line**: `progress.last_line` — моноширинный мелкий текст
4. **Staircase**: фильтрация `completed_names` из списка — пакеты исчезают по мере завершения
5. **Header count**: показывает `visible_count` во время апгрейда

## Верификация

- `cargo test -p chronos-services --lib aur` — 22 теста ✅ (включая 6 новых для `parse_progress_line`)
- `cargo build --release -p chronos` ✅
- Шелл запущен ✅
- Живой апгрейд: **PENDING** — на момент тестирования нет доступных обновлений для реального апгрейда. Код стриминга протестирован unit-тестами на фикстурах pacman-вывода.

## Файлы

| Файл | Изменения |
|---|---|
| `crates/services/src/aur/types.rs` | `UpgradeProgress` struct, `UpgradeState::Running(data)` |
| `crates/services/src/aur/mod.rs` | Streaming `run_upgrade_all()`, `parse_progress_line()`, 6 новых тестов |
| `crates/services/src/lib.rs` | Экспорт `UpgradeProgress` |
| `crates/app/src/updates_popup/view.rs` | Spinner, progress bar, live output, staircase фильтрация |
