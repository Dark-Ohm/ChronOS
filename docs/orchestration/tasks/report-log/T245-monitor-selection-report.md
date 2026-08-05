# T245 — шелл непредсказуемо переезжает на HDMI-A-1 вместо DP-1: отчёт

**Дата:** 2026-08-05
**Статус:** закрыт — корень найден живой диагностикой, фикс закоммичен
(`monitor : … (T245)`), живая верификация 5/5.
**Роль:** FRONTEND (Rust, GPUI) + живая диагностика на этой машине.

## Корень — НЕ uuid, а auto-designate-крысиный капкан

**Гипотеза тикета (б) опровергнута:** `d.uuid()` у форка **стабилен**.
`Source/gpui_linux/src/linux/wayland/display.rs:31`:
```rust
fn uuid(&self) -> anyhow::Result<Uuid> {
    let name = self.name.as_ref().context("…")?;
    Ok(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()))
}
```
uuid = UUIDv5(NAMESPACE_DNS, **`wl_output.name`**) — чистая функция имени
коннектора (`DP-1`/`HDMI-A-1` у Hyprland), не session-order/connector-index.
Эмпирическое подтверждение:
- `uuid5(NS_DNS, "DP-1")     = 09e7b298-aad0-546d-a4de-adcb9106fd7d` —
  ровно тот uuid, что в шапке `monitor.rs` как канонический пример;
- `uuid5(NS_DNS, "HDMI-A-1") = 56f01978-2d1e-5e26-bbe4-cc5fd992f8af` —
  ровно тот uuid, что лежал в `~/.config/chronos/monitor.toml` в момент
  инцидента.

**Настоящий корень — auto-designate в `pult_display()`.** При временном
отсутствии DP-1 из `cx.displays()` (DPMS-сон Samsung ночью / поздний
`wl_output Done` после 100ms-колла `bar::init`) фолбэк largest-by-area
выбирал единственный доступный HDMI-A-1, и старый код переписывал
`monitor.toml` на его uuid. После этого uuid-матч детерминированно сажал
шелл на HDMI **каждую** загрузку. След в лайве: `monitor.toml` переписан
в **02:52:34** (ночь инцидента), бар живьём сидел на HDMI-A-1
(`hyprctl layers`: `namespace: bar`, x=2560, w=1920 — геометрия Dell'а;
на DP-1 бара нет).

## Фикс (`crates/app/src/monitor.rs`)

- Новая чистая функция `should_auto_designate(existing, winner)`:
  запись конфига **только на true first run** (конфига ещё нет).
  Существующий uuid — источник истины и никогда не перезаписывается.
- Временное отсутствие настроенного дисплея теперь: WARN
  `configured uuid … not found among N displays, using fallback` +
  работа на фолбэке, **без** перезаписи конфига.
- Постоянная диагностика резолва: `info!` на каждое разрешение
  (`id/uuid/area/via configured-uuid|fallback-config-mismatch|fallback-no-config`),
  `debug!` по каждому дисплею — для будущих разборов «шелл не туда сел».
- Модульный док обновлён: зафиксирован факт стабильности uuid, механизм
  капкана и семантика «конфиг авторитативен» (смена монитора — правкой
  `monitor.toml` или удалением файла для пере-дизайнации).
- Юнит-тест `should_auto_designate_only_on_first_run` (12/12 зелёные).

**Ремонт конфига:** `~/.config/chronos/monitor.toml` (испорченный багом)
возвращён на `09e7b298-…` (uuid DP-1) — это и было изначальное значение
до ночного переписывания.

## Верификация (живая, release-бинар)

- `chronos-stop`+start ×5: бар на **DP-1 5/5** (`hyprctl layers -j`),
  резолв каждый раз `via configured-uuid (2 live displays)`, ноль строк
  `auto-designating`, `monitor.toml` не тронут (mtime = момент ремонта).
- Анти-капкан: конфиг с мусорным uuid `00000000-…` → WARN
  `not found among 2 displays, using fallback`, резолв
  `via largest-by-area` → DP-1 (больший), конфиг **не переписан**
  (старый код переписал бы его). После теста конфиг восстановлен на DP-1.
- `cargo test --release -p chronos --lib -- monitor`: 12/12 ok.
- `cargo build --release -p chronos`: чисто (только pre-existing warnings).

## Принятый остаточный риск (задокументирован в коде)

First-run на свежей установке при спящем/незаэньюмеренном большом
мониторе в момент первого 100ms-колла всё ещё может задизайнить меньший —
и теперь это перманентно (конфиг авторитативен). Решение отклонённой
альтернативы «сменить ключ uuid→connector name»: uuid уже есть чистая
функция connector name, проблема была не в ключе.

## Коммит

`monitor : pult display auto-designate only on first run — config authoritative (T245)`
+ `orchestration : T245 closed — report + DECISIONS.log entry`.
