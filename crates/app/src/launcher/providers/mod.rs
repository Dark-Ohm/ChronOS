//! Prefix providers for the launcher's single search field (T265-E).
//!
//! A typed prefix switches the field's provider without a second `Input`:
//! `>` shell, `/`/`~` paths, `=` calculator, `?` help, `i:` system info. No
//! prefix is app search (T265-A/B). The dispatcher below is the *one* small
//! `match` so `view.rs` does not grow its own provider switch.

pub mod calc;
pub mod files;
pub mod help;
pub mod shell;
pub mod sysinfo;

/// Which provider owns the field for the current text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provider {
    /// No prefix — normal application search (T265-A/B).
    Apps,
    /// `>` — run a command in `$SHELL`.
    Shell,
    /// `/` or `~` — path browsing.
    Files,
    /// `=` — calculator.
    Calc,
    /// `?` — help for the modes.
    Help,
    /// `i:` — read-only system info.
    SysInfo,
}

impl Provider {
    /// Short header-chip label for the active mode.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Apps => "APPS",
            Provider::Shell => "SHELL",
            Provider::Files => "FILES",
            Provider::Calc => "CALC",
            Provider::Help => "HELP",
            Provider::SysInfo => "SYS",
        }
    }
}

/// What Enter / click does with a provider result row.
#[derive(Clone, Debug, PartialEq)]
pub enum ProviderAction {
    /// Run a command detached in `$SHELL -lc`.
    RunCommand(String),
    /// Open a path with `xdg-open`.
    OpenPath(String),
    /// Copy a string to the clipboard.
    Copy(String),
    /// Read-only row (help, sysinfo, calc error) — Enter is a no-op.
    None,
}

/// One row in a provider's result list (rendered instead of the app grid).
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderResult {
    /// Stable id for selection/dedup.
    pub id: String,
    /// Primary line.
    pub label: String,
    /// Secondary line (shown faint, right-aligned).
    pub detail: Option<String>,
    /// Leading glyph.
    pub glyph: char,
    /// What Enter / click does.
    pub action: ProviderAction,
}

/// Split the raw field text into `(provider, payload)`. Leading whitespace is
/// ignored; the payload is the text after the prefix (trimmed where the prefix
/// char is a separator, kept verbatim for `/`/`~` paths).
///
/// Anything that is not a recognized prefix is plain app search — an unknown
/// prefix is *not* silently swallowed (T265-E).
pub fn parse_prefix(raw: &str) -> (Provider, String) {
    let trimmed = raw.trim_start();
    if trimmed.is_empty() {
        return (Provider::Apps, String::new());
    }
    let first = trimmed.chars().next().expect("non-empty");
    match first {
        '>' => (Provider::Shell, trimmed[1..].trim_start().to_string()),
        '=' => (Provider::Calc, trimmed[1..].trim_start().to_string()),
        '?' => (Provider::Help, String::new()),
        '/' | '~' => (Provider::Files, trimmed.to_string()),
        _ if trimmed.starts_with("i:") => {
            (Provider::SysInfo, trimmed[2..].trim_start().to_string())
        }
        _ => (Provider::Apps, trimmed.to_string()),
    }
}

/// Compute a provider's result rows for `payload`. `Provider::Apps` has no
/// rows — the app search path fills the grid itself.
pub fn results(provider: Provider, payload: &str) -> Vec<ProviderResult> {
    match provider {
        Provider::Apps => Vec::new(),
        Provider::Shell => shell::results(payload),
        Provider::Files => files::results(payload),
        Provider::Calc => calc::results(payload),
        Provider::Help => help::results(),
        Provider::SysInfo => sysinfo::results(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_plain_are_apps() {
        assert_eq!(parse_prefix(""), (Provider::Apps, String::new()));
        assert_eq!(
            parse_prefix("  firefox"),
            (Provider::Apps, "firefox".to_string())
        );
        assert_eq!(
            parse_prefix("firefox"),
            (Provider::Apps, "firefox".to_string())
        );
    }

    #[test]
    fn shell_prefix_tolerates_optional_space() {
        assert_eq!(
            parse_prefix(">echo hi"),
            (Provider::Shell, "echo hi".to_string())
        );
        assert_eq!(
            parse_prefix("> echo hi"),
            (Provider::Shell, "echo hi".to_string())
        );
    }

    #[test]
    fn calc_prefix_strips_leading_space() {
        assert_eq!(parse_prefix("=2+2"), (Provider::Calc, "2+2".to_string()));
        assert_eq!(parse_prefix("= 2+2"), (Provider::Calc, "2+2".to_string()));
    }

    #[test]
    fn help_is_always_available() {
        assert_eq!(parse_prefix("?"), (Provider::Help, String::new()));
        assert_eq!(parse_prefix("?whatever"), (Provider::Help, String::new()));
    }

    #[test]
    fn paths_keep_their_text() {
        assert_eq!(
            parse_prefix("/home/neo/Do"),
            (Provider::Files, "/home/neo/Do".to_string())
        );
        assert_eq!(
            parse_prefix("~/Downloads"),
            (Provider::Files, "~/Downloads".to_string())
        );
    }

    #[test]
    fn sysinfo_prefix_matches_lowercase_i_colon() {
        assert_eq!(parse_prefix("i:"), (Provider::SysInfo, String::new()));
        assert_eq!(
            parse_prefix("i: foo"),
            (Provider::SysInfo, "foo".to_string())
        );
        // No colon -> plain app search.
        assert_eq!(
            parse_prefix("ide"),
            (Provider::Apps, "ide".to_string())
        );
    }
}
