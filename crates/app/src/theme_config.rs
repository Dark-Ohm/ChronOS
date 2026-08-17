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
use gpui_component::theme::ThemeMode;
use inotify::{EventMask, Inotify, WatchMask};
use serde::{Deserialize, Serialize};

const DEBOUNCE_MS: u64 = 300;
const CONFIG_BASENAME: &str = "theme.toml";

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ThemeConfig {
    /// Имя схемы из `builtin_schemes()`. None/empty → falls through to default.
    pub scheme: Option<String>,
    /// T266: requested surface alpha in `0.0..=1.0` (None = opaque). The
    /// effective value is clamped up to the active scheme's readability
    /// floor by `apply_surface_config`.
    #[serde(default)]
    pub surface_alpha: Option<f32>,
    /// T266: compositor blur for the shell surfaces (Hyprland module).
    #[serde(default)]
    pub blur_enabled: bool,
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
            tracing::warn!("theme: read {} failed: {e}, using defaults", path.display());
            ThemeConfig::default()
        }
    }
}

/// Overlay the T266 surface settings onto a resolved scheme. Runs on EVERY
/// scheme-selection path (env / file / default) so alpha and blur survive
/// regardless of how the scheme was chosen.
///
/// Requested alpha is clamped into `0.0..=1.0`, then raised to the scheme's
/// measured readability floor (`min_alpha`) — the slider's low end maps to
/// the floor, never below it.
pub fn apply_surface_config(mut theme: Theme, cfg: &ThemeConfig) -> Theme {
    let requested = cfg.surface_alpha.unwrap_or(1.0).clamp(0.0, 1.0);
    theme.surface.alpha = requested.max(theme.surface.min_alpha);
    theme.surface.blur_enabled = cfg.blur_enabled;
    theme
}

/// Pure resolution: env (highest) → config `scheme` → `Theme::default`,
/// then `apply_surface_config` overlays surface settings exactly once.
///
/// Reuses `Theme::select_scheme`, which already logs `tracing::warn!` on
/// unknown scheme names and returns `Theme::default` (per task brief).
pub fn resolve_theme(env_value: Option<String>, cfg: &ThemeConfig) -> Theme {
    let scheme = if let Some(raw) = env_value {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            Theme::select_scheme(Some(trimmed.to_string()))
        } else {
            resolve_scheme_from_cfg(cfg)
        }
    } else {
        resolve_scheme_from_cfg(cfg)
    };
    apply_surface_config(scheme, cfg)
}

/// Scheme only (no surface overlay) from the config file field.
fn resolve_scheme_from_cfg(cfg: &ThemeConfig) -> Theme {
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
/// Keep gpui-component's own `Theme` mode in lockstep with the active
/// `chronos_ui::Theme` (T205). gpui-component renders the editor gutter /
/// input internals from ITS global theme — if it stayed on the Light default
/// (`gpui_component::init`), CodeEditor gutter numbers (`muted_foreground`)
/// and internal fills would be light even in dark shell.
///
/// T263: mode alone was not enough — component popups (tray/dock
/// `PopupMenu`) rendered the STOCK palette next to ChronOS popups (live
/// catch by the architect, 2026-08-12). The popup-relevant tokens are now
/// mapped from the shell theme below. Still not a 1:1 map — only the
/// tokens component widgets actually read on our surfaces.
/// Safe before `Theme` global exists (`Theme::change` set-globals
/// defensively).
///
/// `pub` because `main.rs` re-syncs AFTER `gpui_component::init` — that init
/// overwrites the mode with Light default, so theme_config::init (earlier)
/// alone would leave a dark shell with a light gutter until first hot-reload.
pub fn sync_gpui_component_theme(cx: &mut App) {
    let shell = *Theme::global(cx);
    let dark = !shell.is_light;

    let mode = if dark { ThemeMode::Dark } else { ThemeMode::Light };
    gpui_component::theme::Theme::change(mode, None, cx);

    // Current-line band: stock dark `#171717` is nearly invisible on ChronOS
    // buffer (`surfaces::editor` ≈ bg.primary). Map from shell tokens so the
    // caret line is always readable (dogfood D1). Requires line_number mode
    // (code_editor) — paint path only draws the band when gutter is on.
    let active_line = shell.interactive.hover.opacity(if dark { 0.5 } else { 0.4 });
    let active_num = if dark {
        shell.accent.primary
    } else {
        shell.text.primary
    };
    let gpui_theme = gpui_component::theme::Theme::global_mut(cx);
    // T263: component popups (`PopupMenu` in tray/dock context menus) render
    // from THESE tokens, not ours — the stock palette made the tray menu a
    // flat near-black rectangle next to ChronOS-token popups. Component
    // `accent` is the MenuItem/ListItem HOVER background, so it maps to our
    // hover wash, not to the saturated `accent.primary`. Must re-apply after
    // `Theme::change` (it reloads stock colors), same as the font lock.
    // T266: tray/dock menus are rendered by gpui-component `PopupMenu`, not
    // by their host view roots — this popover token IS their menu plate.
    // Apply the effective surface alpha so the menus follow the shell's
    // transparency axis like every other surface.
    gpui_theme.popover = shell.surface_color(shell.bg.elevated);
    gpui_theme.popover_foreground = shell.text.primary;
    gpui_theme.accent = shell.interactive.hover;
    gpui_theme.accent_foreground = shell.text.primary;
    gpui_theme.border = shell.border.subtle;
    gpui_theme.muted_foreground = shell.text.muted;
    gpui_theme.selection = shell.bg.selection;
    // Font lock: gpui-component defaults mono to "DejaVu Sans Mono" on Linux
    // (Menlo/Consolas elsewhere). ChronOS canon is JetBrains Mono everywhere —
    // editor gutter/body use mono_font_family; UI chrome uses font_family.
    // Must re-apply after Theme::change (loads stock theme fonts).
    let mono: gpui::SharedString = shell.font_mono.into();
    gpui_theme.font_family = mono.clone();
    gpui_theme.mono_font_family = mono;
    let highlight = std::sync::Arc::make_mut(&mut gpui_theme.highlight_theme);
    highlight.style.editor_active_line = Some(active_line);
    highlight.style.editor_active_line_number = Some(active_num);
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
    sync_gpui_component_theme(cx);
    cx.refresh_windows();
}

/// Toggle Default (dark) ↔ Light, persist `theme.toml`, refresh windows.
///
/// If `CHRONOS_THEME` is set it still wins on next cold `apply`/reload —
/// toggle applies immediately and writes the file for normal resolution.
///
/// T266: the toggle must NOT reset surface settings — the next scheme is
/// overlaid with the current config's alpha/blur (regression gate: an
/// existing translucent setup survives a theme switch).
pub fn toggle(cx: &mut App) {
    let cfg = load_config();
    let next_name = if Theme::global(cx).is_light {
        "Default"
    } else {
        "Light"
    };
    if let Err(e) = persist_scheme(next_name) {
        tracing::warn!("theme: failed to persist scheme={next_name}: {e}");
    }
    let scheme = Theme::select_scheme(Some(next_name.to_string()));
    let theme = apply_surface_config(scheme, &cfg);
    tracing::info!(
        scheme = next_name,
        is_light = theme.is_light,
        surface_alpha = theme.surface.alpha,
        "theme: toggled"
    );
    cx.set_global(theme);
    sync_gpui_component_theme(cx);
    cx.refresh_windows();
}

pub(crate) fn persist_scheme(name: &str) -> std::io::Result<()> {
    write_config_key("scheme", toml::Value::String(name.to_string()))
}

/// Persist only the T266 `surface_alpha` key (RMW — unknown keys and the
/// `scheme` field survive). Returns the effective alpha the settings page
/// should display after clamping.
pub fn persist_surface_alpha(alpha: f32) -> Result<f32, String> {
    write_config_key(
        "surface_alpha",
        toml::Value::Float(f64::from(alpha.clamp(0.0, 1.0))),
    )
    .map_err(|e| e.to_string())?;
    Ok(alpha.clamp(0.0, 1.0))
}

/// Persist only the T266 `blur_enabled` key (RMW).
pub fn persist_blur_enabled(enabled: bool) -> Result<(), String> {
    write_config_key("blur_enabled", toml::Value::Boolean(enabled)).map_err(|e| e.to_string())
}

/// RMW single-key write: load the current document, set exactly one key,
/// write back — every other key (scheme, surface_alpha, blur_enabled,
/// unknown future keys) survives byte-for-byte. Whole-struct serialization
/// would wipe the sibling T266 keys on a scheme toggle.
fn write_config_key(key: &str, value: toml::Value) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: toml::Value = content.parse().map_err(|e: toml::de::Error| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;
    let table = doc
        .as_table_mut()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "not a TOML table"))?;
    table.insert(key.to_string(), value);
    let body = toml::to_string_pretty(&doc)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(&path, body)?;
    Ok(())
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
                tracing::error!("theme: failed to watch {}: {e}", watch_target.display());
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
            ..Default::default()
        };
        let t = resolve_theme(Some("Default".to_string()), &cfg);
        assert_eq!(t, Theme::default());
        assert_ne!(t, light_theme());
    }

    #[test]
    fn empty_config_preserves_opaque_blurless_default() {
        let cfg: ThemeConfig = toml::from_str("").unwrap();
        let theme = resolve_theme(None, &cfg);
        assert_eq!(theme.surface.alpha, 1.0);
        assert!(!theme.surface.blur_enabled);
    }

    #[test]
    fn env_scheme_still_applies_file_surface_settings() {
        let cfg = ThemeConfig {
            surface_alpha: Some(0.72),
            blur_enabled: true,
            ..Default::default()
        };
        let theme = resolve_theme(Some("Default".into()), &cfg);
        assert_eq!(theme.surface.alpha, 0.72_f32.max(theme.surface.min_alpha));
        assert!(theme.surface.blur_enabled);
    }

    #[test]
    fn surface_alpha_clamped_to_scheme_floor() {
        let cfg = ThemeConfig {
            surface_alpha: Some(0.05),
            ..Default::default()
        };
        let theme = resolve_theme(None, &cfg);
        // Requested 0.05 is below the floor — effective alpha must not dip
        // under it (slider low end = floor, never below).
        assert!(theme.surface.alpha >= theme.surface.min_alpha);
    }

    #[test]
    fn persist_scheme_preserves_surface_keys() {
        // Round-trip through a temp doc: a scheme write must not wipe
        // surface_alpha/blur_enabled or unknown keys. The real helper writes
        // to the user config dir; this test exercises the RMW merge via the
        // pure key-write against a parsed doc.
        let mut doc: toml::Value = toml::from_str(
            "scheme = \"Default\"\nsurface_alpha = 0.7\nblur_enabled = true\nunknown = 42\n",
        )
        .unwrap();
        let table = doc.as_table_mut().unwrap();
        table.insert("scheme".into(), toml::Value::String("Light".into()));
        let out: ThemeConfig = toml::from_str(&toml::to_string(&doc).unwrap()).unwrap();
        assert_eq!(out.scheme.as_deref(), Some("Light"));
        assert_eq!(out.surface_alpha, Some(0.7));
        assert!(out.blur_enabled);
    }

    #[test]
    fn resolve_env_case_insensitive_wins_over_config() {
        let cfg = ThemeConfig {
            scheme: Some("Default".to_string()),
            ..Default::default()
        };
        let t = resolve_theme(Some("LiGhT".to_string()), &cfg);
        assert_eq!(t, light_theme());
    }

    #[test]
    fn resolve_config_when_env_unset() {
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
            ..Default::default()
        };
        let t = resolve_theme(None, &cfg);
        assert_eq!(t, light_theme());
    }

    #[test]
    fn resolve_config_when_env_empty() {
        // Empty env string must NOT win — falls through to config.
        let cfg = ThemeConfig {
            scheme: Some("Light".to_string()),
            ..Default::default()
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
            ..Default::default()
        };
        let t = resolve_theme(Some("nonsense-scheme".to_string()), &cfg);
        assert_eq!(t, Theme::default());
        assert_ne!(t, light_theme());
    }

    #[test]
    fn resolve_config_garbage_falls_to_default() {
        let cfg = ThemeConfig {
            scheme: Some("nonsense-scheme".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_theme(None, &cfg), Theme::default());
    }

    #[test]
    fn resolve_config_empty_scheme_falls_to_default() {
        let cfg = ThemeConfig {
            scheme: Some("   ".to_string()),
            ..Default::default()
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

    /// T263: the component theme must carry our popup tokens, not the stock
    /// palette — otherwise tray/dock `PopupMenu` renders foreign colors next
    /// to ChronOS popups. The mapping is scheme-agnostic (it reads whatever
    /// the shell global holds), so assert both schemes.
    fn assert_component_theme_mapped(cx: &gpui::App, shell: &Theme) {
        let gt = gpui_component::theme::Theme::global(cx);
        assert_eq!(gt.popover, shell.bg.elevated, "popover");
        assert_eq!(
            gt.popover_foreground, shell.text.primary,
            "popover_foreground"
        );
        assert_eq!(gt.accent, shell.interactive.hover, "accent (row hover)");
        assert_eq!(gt.accent_foreground, shell.text.primary, "accent_foreground");
        assert_eq!(gt.border, shell.border.subtle, "border");
        assert_eq!(gt.muted_foreground, shell.text.muted, "muted_foreground");
        assert_eq!(gt.selection, shell.bg.selection, "selection");
    }

    #[gpui::test]
    fn sync_maps_shell_tokens_into_component_theme_dark(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            // `Theme::change` reads `ThemeRegistry` when the component Theme
            // global is absent — init first (same pattern as dock context-menu
            // tests).
            gpui_component::init(cx);
            cx.set_global(Theme::default());
            sync_gpui_component_theme(cx);
            let shell = *Theme::global(cx);
            assert_component_theme_mapped(cx, &shell);
        });
    }

    #[gpui::test]
    fn sync_maps_shell_tokens_into_component_theme_light(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            cx.set_global(light_theme());
            sync_gpui_component_theme(cx);
            let shell = *Theme::global(cx);
            assert_component_theme_mapped(cx, &shell);
        });
    }
}
