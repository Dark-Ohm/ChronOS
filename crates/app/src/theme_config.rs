//! Theme config (`~/.config/chronos/theme.toml`) + hot-reload watcher.
//!
//! Resolution order (per task brief #2 of 2026-07-20):
//!   1. `CHRONOS_THEME` env — highest priority (удобно для смоков); empty/whitespace
//!      → falls through to config.
//!   2. `theme.toml` field `scheme = "<имя из builtin_schemes>"` — case-insensitive
//!      match (делегирует в `Theme::select_scheme`).
//!   3. `Theme::default()` (тёмная Mocha-подобная).
//!
//! Hot-reload: правка/создание/удаление `theme.toml` → тема применяется БЕЗ
//! рестарта шелла. Глобал `Theme` переустанавливается, все окна рисуются заново
//! через `cx.refresh_windows()`. Таймер/дебаунс — на GPUI executor
//! (`cx.spawn` + `tokio::time`), НЕ на tokio-спавне (DECISIONS «Runtime split»);
//! блокирующий inotify-читак — отдельный std-тред (паттерн luau/watcher.rs).
//!
//! Файл НЕ перезаписывается молча при отсутствии/битом — только warn и дефолт.

use std::path::PathBuf;
use std::time::Duration;

use chronos_ui::Theme;
use gpui::{App, BorrowAppContext};
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};

const DEBOUNCE_MS: u64 = 300;
const CONFIG_BASENAME: &str = "theme.toml";

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeConfig {
    /// Имя схемы из `builtin_schemes()`. None/empty → falls through to default.
    pub scheme: Option<String>,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("chronos")
        .join(CONFIG_BASENAME)
}

fn parent_dir() -> PathBuf {
    let p = config_path();
    p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
}

/// Load theme config from disk. Missing/bad → `ThemeConfig::default()` + warn.
/// Never silently writes the file (per task brief).
pub fn load_config() -> ThemeConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<ThemeConfig>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(
                    "theme: failed to parse {}: {e}, using defaults",
                    path.display()
                );
                ThemeConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("theme: {} not found, using defaults", path.display());
            ThemeConfig::default()
        }
        Err(e) => {
            tracing::warn!(
                "theme: read {} failed: {e}, using defaults",
                path.display()
            );
            ThemeConfig::default()
        }
    }
}

/// Pure resolution: env (highest) → config `scheme` → `Theme::default`.
///
/// Reuses `Theme::select_scheme`, which already logs `tracing::warn!` on
/// unknown scheme names and returns `Theme::default` (per task brief).
pub fn resolve_theme(env_value: Option<String>, cfg: &ThemeConfig) -> Theme {
    if let Some(raw) = env_value {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Theme::select_scheme(Some(trimmed.to_string()));
        }
    }
    if let Some(ref name) = cfg.scheme {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return Theme::select_scheme(Some(trimmed.to_string()));
        }
    }
    Theme::default()
}

/// Resolve using live env+config — used by init & every reload.
pub fn resolve_active_theme() -> Theme {
    resolve_theme(std::env::var("CHRONOS_THEME").ok(), &load_config())
}

/// Set active `Theme` global + schedule all windows to repaint via
/// `cx.refresh_windows()`. Idempotent — safe to call on every reload.
pub fn apply(cx: &mut App) {
    let theme = resolve_active_theme();
    // `Theme::set` = `*global_mut = …` и паникует, если глобал ещё не
    // создан. На cold-start `Theme::init` больше не зовётся (superseded
    // этим модулем) — первый apply должен `set_global`, не mutate.
    // Повторные hot-reload тоже ок: set_global просто заменяет.
    cx.set_global(theme);
    cx.refresh_windows();
}

/// Initialize theme from env+config and spawn hot-reload watcher.
/// Supersedes `chronos_ui::Theme::init` for the app entry: same role +
/// file config + hot-reload (ChronOS architecture §9).
pub fn init(cx: &mut App) {
    let path = config_path();
    let cfg = load_config();
    let env = std::env::var("CHRONOS_THEME").ok();
    let theme = resolve_theme(env.clone(), &cfg);
    tracing::info!(
        "theme: env={:?}, file={}, bg.primary l={:.2}",
        env,
        path.display(),
        theme.bg.primary.l
    );
    cx.set_global(theme);
    cx.refresh_windows();
    spawn_watcher(cx);
}

/// inotify hot-reload: OS thread owns blocking `Inotify` read, GPUI task
/// runs the debounce timer + `apply` (per luau/watcher.rs pattern).
///
/// Watches the parent dir (not the file itself — inotify on a non-existing
/// file fails; watching the dir catches later CREATE). Filters events by
/// basename `theme.toml`.
pub fn spawn_watcher(cx: &mut App) {
    let parent = parent_dir();
    if !parent.is_dir() {
        tracing::debug!(
            "theme: parent dir {} missing, hot-reload disabled",
            parent.display()
        );
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let watch_target = parent.clone();

    std::thread::Builder::new()
        .name("theme-inotify".into())
        .spawn(move || {
            let mut inotify = match Inotify::init() {
                Ok(i) => i,
                Err(e) => {
                    tracing::error!("theme: inotify init failed: {e}");
                    return;
                }
            };

            // CLOSE_WRITE covers normal save; MOVED_TO covers atomic rename
            // (editor write-temp-then-rename); CREATE/DELETE catch file
            // appearance/disappearance; MODIFY catches partial writes.
            let mask = WatchMask::CLOSE_WRITE
                .union(WatchMask::MOVED_TO)
                .union(WatchMask::CREATE)
                .union(WatchMask::DELETE)
                .union(WatchMask::MODIFY);
            if let Err(e) = inotify.watches().add(&watch_target, mask) {
                tracing::error!(
                    "theme: failed to watch {}: {e}",
                    watch_target.display()
                );
                return;
            }

            let target = std::ffi::OsStr::new(CONFIG_BASENAME);
            let mut buf = [0u8; 4096];
            loop {
                let events = match inotify.read_events_blocking(&mut buf) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("theme: inotify read error: {e}");
                        break;
                    }
                };

                let mut changed = false;
                for ev in events {
                    if ev.mask.contains(EventMask::ISDIR) {
                        continue;
                    }
                    if let Some(name) = ev.name {
                        if name == target {
                            changed = true;
                        }
                    }
                }
                if changed && tx.send(()).is_err() {
                    break; // receiver dropped — GPUI app shutting down
                }
            }
        })
        .expect("theme: failed to spawn inotify thread");

    cx.spawn(async move |cx| {
        // Trailing debounce: reset on every batch, fire DEBOUNCE_MS after
        // the last event — coalesces editor save bursts (write-temp + rename).
        let mut deadline: Option<tokio::time::Instant> = None;
        loop {
            let timer = async {
                match deadline {
                    Some(d) => tokio::time::sleep_until(d).await,
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                _ = rx.recv() => {
                    deadline = Some(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                }
                _ = timer => {
                    deadline = None;
                    let _ = cx.update(|cx| {
                        apply(cx);
                        tracing::info!("theme: hot-reloaded from {}", config_path().display());
                    });
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_ui::parse_hex;

    fn light_theme() -> Theme {
        chronos_ui::builtin_schemes()
            .into_iter()
            .find(|s| s.name == "Light")
            .map(|s| s.theme)
            .unwrap()
    }

    #[test]
    fn resolve_env_wins_over_config() {
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
        };
        let t = resolve_theme(Some("Default".to_string()), &cfg);
        assert_eq!(t, Theme::default());
        assert_ne!(t, light_theme());
    }

    #[test]
    fn resolve_env_case_insensitive_wins_over_config() {
        let cfg = ThemeConfig {
            scheme: Some("Default".to_string()),
        };
        let t = resolve_theme(Some("LiGhT".to_string()), &cfg);
        assert_eq!(t, light_theme());
    }

    #[test]
    fn resolve_config_when_env_unset() {
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
        };
        let t = resolve_theme(None, &cfg);
        assert_eq!(t, light_theme());
    }

    #[test]
    fn resolve_config_when_env_empty() {
        // Empty env string must NOT win — falls through to config.
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
        };
        let t = resolve_theme(Some(String::new()), &cfg);
        assert_eq!(t, light_theme());
        let t = resolve_theme(Some("   ".to_string()), &cfg);
        assert_eq!(t, light_theme());
    }

    #[test]
    fn resolve_default_when_both_unset() {
        let cfg = ThemeConfig::default();
        assert_eq!(resolve_theme(None, &cfg), Theme::default());
        assert_eq!(resolve_theme(Some(String::new()), &cfg), Theme::default());
    }

    #[test]
    fn resolve_env_garbage_falls_to_default_not_config() {
        // env garbage — select_scheme warns + returns default (does NOT fall
        // through to config). This is the documented «env перебивает конфиг».
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
        };
        let t = resolve_theme(Some("nonsense-scheme".to_string()), &cfg);
        assert_eq!(t, Theme::default());
        assert_ne!(t, light_theme());
    }

    #[test]
    fn resolve_config_garbage_falls_to_default() {
        let cfg = ThemeConfig {
            scheme: Some("nonsense-scheme".to_string()),
        };
        assert_eq!(resolve_theme(None, &cfg), Theme::default());
    }

    #[test]
    fn resolve_config_empty_scheme_falls_to_default() {
        let cfg = ThemeConfig {
            scheme: Some("   ".to_string()),
        };
        assert_eq!(resolve_theme(None, &cfg), Theme::default());
    }

    #[test]
    fn parse_theme_toml_with_scheme_field() {
        let toml_str = r#"scheme = "Light""#;
        let cfg: ThemeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.scheme.as_deref(), Some("Light"));
    }

    #[test]
    fn parse_theme_toml_empty_file() {
        // Empty toml → all-None, no panic.
        let cfg: ThemeConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.scheme, None);
    }

    #[test]
    fn parse_theme_toml_ignores_unknown_keys() {
        // serde defaults to ignoring unknown fields — будущие опции (radius,
        // font_uid и т.п.) не сломают чтение `scheme`.
        let toml_str = r#"unknown_field = 42
scheme = "Light""#;
        let cfg: ThemeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.scheme.as_deref(), Some("Light"));
    }

    #[test]
    fn parse_theme_toml_invalid_does_not_panic() {
        let result: Result<ThemeConfig, _> = toml::from_str("not valid toml [[[");
        assert!(result.is_err());
    }

    /// Sanity: акцент не переопределяется в Light (кровный факт из №1).
    /// Сохраняем инвариат: `accent.primary` одинаковый в обеих схемах.
    #[test]
    fn accent_is_same_across_schemes() {
        let accent = parse_hex("007acc").unwrap();
        assert_eq!(Theme::default().accent.primary, accent);
        assert_eq!(light_theme().accent.primary, accent);
    }
}