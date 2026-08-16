//! Desktop entry data types.

use std::path::Path;

/// A `[Desktop Action <id>]` group from a .desktop file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DesktopAction {
    /// Action id from the section header, e.g. "NewWorkspace".
    pub id: String,
    /// Localized display name (`Name=`).
    pub name: String,
    /// `Exec=` with XDG field codes already stripped.
    pub exec: String,
}

/// A parsed XDG .desktop file (Type=Application only).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppEntry {
    /// Filename without `.desktop` extension (e.g. "firefox").
    pub id: String,
    /// Resolved display name (locale-aware).
    pub name: String,
    /// Exec= value with XDG field codes (%f, %F, %u, etc.) already stripped.
    pub exec: String,
    /// Icon name from Icon= field.
    pub icon: Option<String>,
    /// Whether Terminal=true (launch in terminal).
    pub terminal: bool,
    /// Categories= split by `;`, empty strings dropped.
    pub categories: Vec<String>,
    /// GenericName= (locale-aware) — e.g. "Web Browser".
    pub generic_name: Option<String>,
    /// Comment= (locale-aware) — e.g. "Browse the web".
    pub comment: Option<String>,
    /// Keywords= split by `;`, empty strings dropped.
    pub keywords: Vec<String>,
    /// NoDisplay=true — hidden from the default launcher list (stays in state).
    pub no_display: bool,
    /// Hidden=true — hidden from the default launcher list (stays in state).
    pub hidden: bool,
    /// `[Desktop Action <id>]` groups (each with `Name=` + `Exec=`).
    pub actions: Vec<DesktopAction>,
}

impl AppEntry {
    /// Test/one-off constructor: set `id` and `name`, leave the rest defaulted.
    /// Keeps downstream test literals from spelling out the new fields.
    pub fn fixture(id: impl Into<String>, name: impl Into<String>) -> Self {
        AppEntry {
            id: id.into(),
            name: name.into(),
            ..AppEntry::default()
        }
    }

    /// Visibility filter for the default launcher list: an entry is listed
    /// unless it is `NoDisplay` or `Hidden`. Hidden entries stay in the
    /// service state (T265-G can surface them later).
    pub fn is_listed(&self) -> bool {
        !self.no_display && !self.hidden
    }
}

/// Reactive snapshot of all desktop entries on the system.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplicationsState {
    /// Default-visible entries (`AppEntry::is_listed`).
    pub entries: Vec<AppEntry>,
    /// NoDisplay/Hidden entries, retained for future "show hidden" support.
    pub hidden: Vec<AppEntry>,
}

/// Commands issued by UI. Currently unused — reserved for future use.
#[derive(Clone, Debug)]
pub enum ApplicationsCommand {
    Noop,
}

/// Strip XDG field codes from an Exec= string.
///
/// Field codes: %f %F %u %U %d %D %n %N %i %c %k %v %m
pub fn strip_field_codes(exec: &str) -> String {
    let mut result = String::with_capacity(exec.len());
    let mut chars = exec.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            chars.next();
        } else {
            result.push(ch);
        }
    }
    let mut collapsed = String::with_capacity(result.len());
    let mut prev_space = false;
    for ch in result.chars() {
        if ch == ' ' {
            if !prev_space {
                collapsed.push(ch);
            }
            prev_space = true;
        } else {
            collapsed.push(ch);
            prev_space = false;
        }
    }
    collapsed.trim().to_string()
}

/// Parse a .desktop file into an AppEntry. Returns None if:
/// - File cannot be read
/// - Type is not "Application"
/// - Missing required Name= or Exec= fields
///
/// `NoDisplay`/`Hidden` entries are still parsed (not dropped) — visibility is
/// decided downstream by `AppEntry::is_listed` / `scan_all` (T265-A).
pub fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut desktop_type = None;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut terminal = false;
    let mut no_display = false;
    let mut hidden = false;
    let mut generic_name = None;
    let mut comment = None;
    let mut keywords: Vec<String> = Vec::new();
    let mut categories = Vec::new();
    let mut actions: Vec<DesktopAction> = Vec::new();

    let locale = std::env::var("LANG").ok().and_then(|l| {
        let lang_part = l.split('.').next()?;
        let lang_base = lang_part.split('_').next()?;
        if lang_base == "C" || lang_base.is_empty() {
            None
        } else {
            Some(lang_base.to_string())
        }
    });

    // Only fields inside `[Desktop Entry]` count as the main entry. `.desktop`
    // files may carry extra groups (`[Desktop Action NewWorkspace]`, …) with
    // their own `Name=`/`Exec=` — those must not leak into the main entry (see
    // `desktop_action_section_does_not_override_main_name` regression test) and
    // are collected into `actions` instead (T265-A).
    let mut in_main_section = false;
    let mut current_action: Option<DesktopAction> = None;

    // Flush the in-progress `[Desktop Action]` group: strip field codes and
    // drop incomplete groups (id/name/exec must all be present).
    fn flush_action(action: &mut Option<DesktopAction>, actions: &mut Vec<DesktopAction>) {
        if let Some(mut action) = action.take() {
            if action.id.is_empty() || action.name.is_empty() || action.exec.is_empty() {
                return;
            }
            action.exec = strip_field_codes(&action.exec);
            actions.push(action);
        }
    }

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            flush_action(&mut current_action, &mut actions);
            in_main_section = line == "[Desktop Entry]";
            if let Some(id) = line
                .strip_prefix("[Desktop Action ")
                .and_then(|s| s.strip_suffix(']'))
            {
                current_action = Some(DesktopAction {
                    id: id.trim().to_string(),
                    name: String::new(),
                    exec: String::new(),
                });
            }
            continue;
        }
        if in_main_section {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "Type" => desktop_type = Some(value.to_string()),
                    "Name" => name = Some(value.to_string()),
                    "GenericName" => generic_name = Some(value.to_string()),
                    "Comment" => comment = Some(value.to_string()),
                    "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
                    "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                    "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                    "Exec" if exec.is_none() => exec = Some(value.to_string()),
                    "Icon" => icon = Some(value.to_string()),
                    "Keywords" => {
                        keywords = value
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                    }
                    "Categories" => {
                        categories = value
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                    }
                    _ => {}
                }
                // Locale-aware fallback: `Name[ru]`, `GenericName[ru]`,
                // `Comment[ru]` override the bare key when it matches LANG.
                if key.ends_with(']') {
                    if let Some(open) = key.find('[') {
                        let base = &key[..open];
                        let key_lang = &key[open + 1..key.len() - 1];
                        if let Some(lang) = &locale {
                            if key_lang == lang {
                                match base {
                                    "Name" => name = Some(value.to_string()),
                                    "GenericName" => generic_name = Some(value.to_string()),
                                    "Comment" => comment = Some(value.to_string()),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(action) = current_action.as_mut() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "Name" => action.name = value.to_string(),
                    "Exec" => action.exec = value.to_string(),
                    _ => {}
                }
            }
        }
    }
    flush_action(&mut current_action, &mut actions);

    if desktop_type.as_deref() != Some("Application") {
        return None;
    }

    let id = path.file_stem()?.to_str()?.to_string();
    let name = name?;
    let exec = strip_field_codes(&exec?);

    Some(AppEntry {
        id,
        name,
        exec,
        icon,
        terminal,
        categories,
        generic_name,
        comment,
        keywords,
        no_display,
        hidden,
        actions,
    })
}

#[cfg(test)]
// `set_var`/`remove_var` are process-global and unsafe since Rust 2024 edition;
// confined to single-threaded test-only LANG fiddling for locale-fallback coverage.
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_desktop_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{name}.desktop"));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn parse_minimal_valid_entry() {
        let dir = std::env::temp_dir().join("app-service-test-minimal");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "firefox",
            "[Desktop Entry]\nType=Application\nName=Firefox\nExec=/usr/bin/firefox\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.id, "firefox");
        assert_eq!(entry.name, "Firefox");
        assert_eq!(entry.exec, "/usr/bin/firefox");
        assert!(!entry.terminal);
        assert!(entry.icon.is_none());
        assert!(entry.is_listed());
        assert!(entry.actions.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_non_application_type() {
        let dir = std::env::temp_dir().join("app-service-test-nonapp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "readme",
            "[Desktop Entry]\nType=Link\nName=Readme\nURL=https://example.com\n",
        );
        assert!(parse_desktop_file(&path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_display_parsed_and_not_listed() {
        let dir = std::env::temp_dir().join("app-service-test-nodisplay");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "hidden",
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=/usr/bin/hidden\nNoDisplay=true\n",
        );
        let entry = parse_desktop_file(&path).expect("NoDisplay entries are parsed, not dropped");
        assert!(entry.no_display);
        assert!(!entry.is_listed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_parsed_and_not_listed() {
        let dir = std::env::temp_dir().join("app-service-test-hidden");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "secret",
            "[Desktop Entry]\nType=Application\nName=Secret\nExec=/usr/bin/secret\nHidden=true\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert!(entry.hidden);
        assert!(!entry.no_display);
        assert!(!entry.is_listed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_generic_name_comment_keywords() {
        let dir = std::env::temp_dir().join("app-service-test-fields");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "alacritty",
            "[Desktop Entry]\nType=Application\nName=Alacritty\nGenericName=Terminal\nComment=A fast terminal emulator\nKeywords=terminal;shell;console;\nExec=/usr/bin/alacritty\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.generic_name.as_deref(), Some("Terminal"));
        assert_eq!(entry.comment.as_deref(), Some("A fast terminal emulator"));
        assert_eq!(entry.keywords, vec!["terminal", "shell", "console"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strip_field_codes_removes_percent_args() {
        assert_eq!(
            strip_field_codes("/usr/bin/app %f --flag"),
            "/usr/bin/app --flag"
        );
        assert_eq!(strip_field_codes("/usr/bin/app %u"), "/usr/bin/app");
        assert_eq!(strip_field_codes("/usr/bin/app %F %U"), "/usr/bin/app");
        assert_eq!(strip_field_codes("/usr/bin/app"), "/usr/bin/app");
    }

    #[test]
    fn locale_fallback_overrides_bare_keys() {
        // Single test touching the process-global LANG — a second one would
        // race against this one and flake (env vars are global).
        let dir = std::env::temp_dir().join("app-service-test-locale");
        std::fs::create_dir_all(&dir).unwrap();
        let original_lang = std::env::var("LANG").ok();
        unsafe { std::env::set_var("LANG", "ru_RU.UTF-8") };

        let path = write_desktop_file(
            &dir,
            "testapp",
            "[Desktop Entry]\nType=Application\nName=English\nName[ru]=Russkii\nGenericName=Tool\nGenericName[ru]=Instrument\nComment=Does things\nComment[ru]=Delaet veschi\nExec=/usr/bin/test\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.name, "Russkii");
        assert_eq!(entry.generic_name.as_deref(), Some("Instrument"));
        assert_eq!(entry.comment.as_deref(), Some("Delaet veschi"));

        match original_lang {
            Some(val) => unsafe { std::env::set_var("LANG", val) },
            None => unsafe { std::env::remove_var("LANG") },
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_with_icon_and_terminal() {
        let dir = std::env::temp_dir().join("app-service-test-full");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "htop",
            "[Desktop Entry]\nType=Application\nName=htop\nExec=/usr/bin/htop\nIcon=htop\nTerminal=true\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.icon.as_deref(), Some("htop"));
        assert!(entry.terminal);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_categories_splits_and_drops_empty() {
        let dir = std::env::temp_dir().join("app-service-test-categories");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "game",
            "[Desktop Entry]\nType=Application\nName=Game\nExec=/usr/bin/game\nCategories=Game;Action;\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.categories, vec!["Game", "Action"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_no_categories_defaults_to_empty() {
        let dir = std::env::temp_dir().join("app-service-test-no-categories");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "app",
            "[Desktop Entry]\nType=Application\nName=App\nExec=/usr/bin/app\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert!(entry.categories.is_empty());
        assert!(entry.keywords.is_empty());
        assert!(entry.generic_name.is_none());
        assert!(entry.comment.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_action_section_does_not_override_main_name() {
        // Regression: Zed's real .desktop file has a `[Desktop Action
        // NewWorkspace]` group with its own `Name=`/`Exec=` — the parser
        // didn't track sections, so that group's `Name=` silently
        // overwrote the main entry's name ("Zed" -> "Open a new
        // workspace"), making the launcher's fuzzy search (which only
        // indexes `entry.name`) unable to find it by typing "zed".
        let dir = std::env::temp_dir().join("app-service-test-action-section");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "dev.zed.Zed",
            "[Desktop Entry]\nType=Application\nName=Zed\nExec=/usr/bin/zed %U\nActions=NewWorkspace;\n\n[Desktop Action NewWorkspace]\nExec=/usr/bin/zed --new %U\nName=Open a new workspace\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.name, "Zed", "Name from [Desktop Action] group must not leak into the main entry");
        assert_eq!(entry.exec, "/usr/bin/zed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktop_actions_collected_with_stripped_exec() {
        let dir = std::env::temp_dir().join("app-service-test-actions");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "dev.zed.Zed",
            "[Desktop Entry]\nType=Application\nName=Zed\nExec=/usr/bin/zed %U\nActions=NewWorkspace;\n\n[Desktop Action NewWorkspace]\nExec=/usr/bin/zed --new %U\nName=Open a new workspace\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.actions.len(), 1);
        let action = &entry.actions[0];
        assert_eq!(action.id, "NewWorkspace");
        assert_eq!(action.name, "Open a new workspace");
        assert_eq!(action.exec, "/usr/bin/zed --new", "field codes must be stripped from action exec");
        // Main entry fields stay intact alongside the action group.
        assert_eq!(entry.name, "Zed");
        assert_eq!(entry.exec, "/usr/bin/zed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn incomplete_desktop_action_group_is_dropped() {
        let dir = std::env::temp_dir().join("app-service-test-action-incomplete");
        std::fs::create_dir_all(&dir).unwrap();
        // Action has a Name but no Exec — must not be collected.
        let path = write_desktop_file(
            &dir,
            "app",
            "[Desktop Entry]\nType=Application\nName=App\nExec=/usr/bin/app\n\n[Desktop Action Broken]\nName=Broken\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert!(entry.actions.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_strips_field_codes_from_exec() {
        let dir = std::env::temp_dir().join("app-service-test-fieldcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "fieldcodes",
            "[Desktop Entry]\nType=Application\nName=FieldCodes\nExec=/usr/bin/app %u --flag %f\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.exec, "/usr/bin/app --flag", "field codes must be stripped at parse time");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
