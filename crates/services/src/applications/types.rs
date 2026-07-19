//! Desktop entry data types.

use std::path::Path;

/// A parsed XDG .desktop file (Type=Application only).
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

/// Reactive snapshot of all desktop entries on the system.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplicationsState {
    pub entries: Vec<AppEntry>,
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
/// - NoDisplay=true
/// - Missing required Name= or Exec= fields
pub fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut desktop_type = None;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut terminal = false;
    let mut no_display = false;

    let locale = std::env::var("LANG").ok().and_then(|l| {
        let lang_part = l.split('.').next()?;
        let lang_base = lang_part.split('_').next()?;
        if lang_base == "C" || lang_base.is_empty() {
            None
        } else {
            Some(lang_base.to_string())
        }
    });

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Type" => desktop_type = Some(value.to_string()),
                "Name" => name = Some(value.to_string()),
                "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                "Exec" if exec.is_none() => exec = Some(value.to_string()),
                "Icon" => icon = Some(value.to_string()),
                _ => {}
            }
            if key.starts_with("Name[") && key.ends_with(']') {
                if let Some(lang) = &locale {
                    let key_lang = &key[5..key.len() - 1];
                    if key_lang == lang {
                        name = Some(value.to_string());
                    }
                }
            }
        }
    }

    if desktop_type.as_deref() != Some("Application") {
        return None;
    }
    if no_display {
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
    fn skip_no_display() {
        let dir = std::env::temp_dir().join("app-service-test-nodisplay");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_desktop_file(
            &dir,
            "hidden",
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=/usr/bin/hidden\nNoDisplay=true\n",
        );
        assert!(parse_desktop_file(&path).is_none());
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
    fn locale_name_fallback() {
        let dir = std::env::temp_dir().join("app-service-test-locale");
        std::fs::create_dir_all(&dir).unwrap();
        let original_lang = std::env::var("LANG").ok();
        unsafe { std::env::set_var("LANG", "ru_UF.UTF-8") };

        let path = write_desktop_file(
            &dir,
            "testapp",
            "[Desktop Entry]\nType=Application\nName=English\nName[ru]=Russkii\nExec=/usr/bin/test\n",
        );
        let entry = parse_desktop_file(&path).unwrap();
        assert_eq!(entry.name, "Russkii");

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
