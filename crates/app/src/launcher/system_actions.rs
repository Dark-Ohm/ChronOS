//! System actions in the launcher header (T265-F): Lock / Log out / Sleep /
//! Hibernate / Restart / Shutdown.
//!
//! Order comes from `[system_actions] order = [...]` in `launcher.toml`
//! (unknown ids → default + warn). The arm/confirm state machine is shared
//! with the right panel via `crate::power`. Avatar/name come from
//! `passwd` GECOS + `~/.face` / AccountsService; a missing icon falls back to
//! an initial, never a broken image.

use std::path::PathBuf;

use crate::launcher::launcher_config::LauncherConfig;
use crate::power::PowerAction;

/// Built-in header order (T265-F).
pub const DEFAULT_ACTIONS: [PowerAction; 6] = [
    PowerAction::Lock,
    PowerAction::LogOut,
    PowerAction::Sleep,
    PowerAction::Hibernate,
    PowerAction::Restart,
    PowerAction::Shutdown,
];

/// Parse a `[system_actions]` id. Accepts the canonical ids plus a couple of
/// friendly aliases; `None` on garbage.
pub fn parse_action(raw: &str) -> Option<PowerAction> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "lock" => Some(PowerAction::Lock),
        "logout" | "log_out" | "log-out" => Some(PowerAction::LogOut),
        "sleep" | "suspend" => Some(PowerAction::Sleep),
        "hibernate" => Some(PowerAction::Hibernate),
        "restart" | "reboot" => Some(PowerAction::Restart),
        "shutdown" | "poweroff" | "power-off" => Some(PowerAction::Shutdown),
        _ => None,
    }
}

/// Resolve the header order: the config list if present and non-empty, else
/// the default six. Unknown ids are warned and skipped; an all-garbage list
/// falls back to the default (T265-F: "мусор в toml → дефолт + warn").
pub fn resolve_actions(config: &LauncherConfig) -> Vec<PowerAction> {
    let order = &config.system_actions.order;
    if order.is_empty() {
        return DEFAULT_ACTIONS.to_vec();
    }
    let mut actions: Vec<PowerAction> = Vec::new();
    for raw in order {
        match parse_action(raw) {
            Some(action) => actions.push(action),
            None => tracing::warn!("launcher: unknown system_actions id '{raw}' — skipped"),
        }
    }
    if actions.is_empty() {
        tracing::warn!("launcher: system_actions had no valid ids — using default order");
        DEFAULT_ACTIONS.to_vec()
    } else {
        actions
    }
}

/// True when the kernel lists `mode` in `/sys/power/state` (pure helper for
/// tests; the `*_available` fns below read the real file).
pub fn supports_mode(state: &str, mode: &str) -> bool {
    state.split_whitespace().any(|token| token == mode)
}

fn power_state() -> String {
    std::fs::read_to_string("/sys/power/state").unwrap_or_default()
}

/// Hibernate (sleep-to-disk) support — kernel advertises `disk` in state.
pub fn hibernate_available() -> bool {
    supports_mode(&power_state(), "disk")
}

/// Suspend (sleep-to-RAM) support — kernel advertises `mem` in state.
pub fn suspend_available() -> bool {
    supports_mode(&power_state(), "mem")
}

/// Whether an action's backend is available on this system (T246 — a missing
/// backend renders the tile disabled, not a no-op).
pub fn available(action: PowerAction) -> bool {
    match action {
        PowerAction::Hibernate => hibernate_available(),
        PowerAction::Sleep => suspend_available(),
        _ => true,
    }
}

/// Canonical `launcher.toml` id for an action (inverse of `parse_action`).
pub fn action_id(action: PowerAction) -> &'static str {
    match action {
        PowerAction::Lock => "lock",
        PowerAction::LogOut => "logout",
        PowerAction::Sleep => "sleep",
        PowerAction::Hibernate => "hibernate",
        PowerAction::Restart => "restart",
        PowerAction::Shutdown => "shutdown",
    }
}

/// Human reason shown when a tile is disabled (tooltip/label).
pub fn disabled_reason(action: PowerAction) -> Option<&'static str> {
    match action {
        PowerAction::Hibernate if !hibernate_available() => Some("no hibernate support"),
        PowerAction::Sleep if !suspend_available() => Some("no suspend support"),
        _ => None,
    }
}

/// Current login name (`$USER`, then `$LOGNAME`, then a fallback).
pub fn user_name() -> String {
    for key in ["USER", "LOGNAME"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    "user".to_string()
}

/// GECOS full name from `/etc/passwd` (the part before the first `,`).
pub fn user_full_name() -> Option<String> {
    let user = user_name();
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let mut cols = line.split(':');
        if cols.next() == Some(user.as_str()) {
            // columns: password, uid, gid, gecos, home, shell.
            let _password = cols.next();
            let _uid = cols.next();
            let _gid = cols.next();
            let gecos = cols.next().unwrap_or("");
            let full = gecos.split(',').next().unwrap_or("").trim();
            if !full.is_empty() {
                return Some(full.to_string());
            }
        }
    }
    None
}

/// Avatar file: `$HOME/.face` first, then the AccountsService icon — an
/// absolute path only when the file actually exists (never a broken image).
pub fn face_path() -> Option<PathBuf> {
    let user = user_name();
    let home = dirs::home_dir()?;
    let icons_dir = PathBuf::from("/var/lib/AccountsService/icons");
    [
        home.join(".face"),
        icons_dir.join(&user),
        icons_dir.join(format!("{user}.face")),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// First letter of the user's name (GECOS, else login), uppercased — the
/// fallback avatar glyph.
pub fn user_initial() -> String {
    let name = user_full_name().unwrap_or_else(user_name);
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::launcher_config::SystemActionsConfig;

    #[test]
    fn default_order_is_the_spec_six() {
        let actions = DEFAULT_ACTIONS;
        let names: Vec<&str> = actions.iter().map(|a| a.label()).collect();
        assert_eq!(
            names,
            ["Lock", "Log out", "Sleep", "Hibernate", "Restart", "Shutdown"]
        );
    }

    #[test]
    fn parse_action_accepts_ids_and_aliases() {
        assert_eq!(parse_action("lock"), Some(PowerAction::Lock));
        assert_eq!(parse_action("logout"), Some(PowerAction::LogOut));
        assert_eq!(parse_action("reboot"), Some(PowerAction::Restart));
        assert_eq!(parse_action("restart"), Some(PowerAction::Restart));
        assert_eq!(parse_action("  SHUTDOWN "), Some(PowerAction::Shutdown));
        assert_eq!(parse_action("hibernate"), Some(PowerAction::Hibernate));
        assert_eq!(parse_action("garbage"), None);
    }

    #[test]
    fn empty_config_resolves_to_default() {
        let actions = resolve_actions(&LauncherConfig::default());
        assert_eq!(actions, DEFAULT_ACTIONS.to_vec());
    }

    #[test]
    fn valid_order_is_respected() {
        let cfg = LauncherConfig {
            system_actions: SystemActionsConfig {
                order: vec!["shutdown".into(), "lock".into()],
            },
            ..LauncherConfig::default()
        };
        assert_eq!(
            resolve_actions(&cfg),
            vec![PowerAction::Shutdown, PowerAction::Lock]
        );
    }

    #[test]
    fn garbage_falls_back_to_default() {
        let cfg = LauncherConfig {
            system_actions: SystemActionsConfig {
                order: vec!["nonsense".into(), "also-bad".into()],
            },
            ..LauncherConfig::default()
        };
        assert_eq!(resolve_actions(&cfg), DEFAULT_ACTIONS.to_vec());
    }

    #[test]
    fn mixed_keeps_valid_and_skips_garbage() {
        let cfg = LauncherConfig {
            system_actions: SystemActionsConfig {
                order: vec!["lock".into(), "junk".into(), "sleep".into()],
            },
            ..LauncherConfig::default()
        };
        assert_eq!(
            resolve_actions(&cfg),
            vec![PowerAction::Lock, PowerAction::Sleep]
        );
    }

    #[test]
    fn supports_mode_matches_tokens() {
        assert!(supports_mode("freeze mem disk", "disk"));
        assert!(supports_mode("freeze mem disk", "mem"));
        assert!(!supports_mode("freeze mem", "disk"));
        assert!(!supports_mode("", "disk"));
    }
}
