//! kitty.conf theme loader for the desktop-terminal widget.
//!
//! Parses a subset of kitty.conf directives that affect the terminal's
//! appearance: font, colors and opacity. Unknown directives are ignored
//! silently — kitty.conf supports hundreds of options we do not care about.
//!
//! Known limitation: only hex colors (`#rrggbb`) are parsed. Named X11 colors
//! (e.g. `red`, `dark blue`) are left as `None` — if you need them, add a
//! `const X11 palette` lookup and wire it in `parse_color`.

use std::path::{Path, PathBuf};

/// RGBA color with 8-bit channels. GPUI-independent so the services crate
/// does not pull in a UI dependency.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Subset of kitty.conf that the terminal view actually consumes.
///
/// Every field is `Option` — missing or unparseable values fall back to the
/// view's built-in defaults. Construct via [`load`] or [`Default::default`]
/// (all `None`).
#[derive(Debug, Clone, Default)]
pub struct KittyTheme {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub background_opacity: Option<f32>,
    pub foreground: Option<Rgba8>,
    pub background: Option<Rgba8>,
    /// ANSI color0..color15. `None` means "use the existing ANSI mapping".
    pub palette: [Option<Rgba8>; 16],
}

/// Default kitty.conf location: `~/.config/kitty/kitty.conf`.
///
/// Returns `None` if `$HOME` is unset or the file does not exist — the caller
/// should then use [`KittyTheme::default`] (no theme).
pub fn default_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".config/kitty/kitty.conf");
    path.exists().then_some(path)
}

/// Parse `path` into a [`KittyTheme`].
///
/// Missing file, unreadable file, or a file where none of our directives are
/// set → returns [`KittyTheme::default`] (all `None`). Never panics on bad
/// input: unknown keys, malformed values, and hex parse errors all silently
/// skip the line.
pub fn load(path: &Path) -> KittyTheme {
    let Some(text) = std::fs::read_to_string(path).ok() else {
        return KittyTheme::default();
    };
    parse(&text, path, 0)
}

fn parse(text: &str, base: &Path, depth: u8) -> KittyTheme {
    let mut theme = KittyTheme::default();
    let dir = base.parent();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("include ") {
            if depth >= 1 {
                continue;
            }
            let include_path = Path::new(rest.trim());
            let full = match dir {
                Some(d) if include_path.is_relative() => d.join(include_path),
                _ => include_path.to_path_buf(),
            };
            if let Some(included) = std::fs::read_to_string(&full).ok() {
                let nested = parse(&included, &full, depth + 1);
                merge(&mut theme, nested);
            }
            continue;
        }

        let Some((key, value)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "font_family" => theme.font_family = Some(value.to_string()),
            "font_size" => {
                if let Ok(size) = value.parse::<f32>() {
                    theme.font_size = Some(size);
                }
            }
            "background_opacity" => {
                if let Ok(op) = value.parse::<f32>() {
                    theme.background_opacity = Some(op.clamp(0.0, 1.0));
                }
            }
            "foreground" => theme.foreground = parse_color(value),
            "background" => theme.background = parse_color(value),
            key if key.starts_with("color") => {
                if let Ok(n) = key[5..].parse::<usize>()
                    && n < 16
                {
                    theme.palette[n] = parse_color(value);
                }
            }
            _ => {}
        }
    }

    theme
}

fn parse_color(value: &str) -> Option<Rgba8> {
    let v = value.trim();
    let hex = v.strip_prefix('#')?;
    match hex.len() {
        6 => Some(Rgba8 {
            r: hex2(hex.as_bytes()[0..2].try_into().ok()?)?,
            g: hex2(hex.as_bytes()[2..4].try_into().ok()?)?,
            b: hex2(hex.as_bytes()[4..6].try_into().ok()?)?,
            a: 0xff,
        }),
        8 => Some(Rgba8 {
            r: hex2(hex.as_bytes()[0..2].try_into().ok()?)?,
            g: hex2(hex.as_bytes()[2..4].try_into().ok()?)?,
            b: hex2(hex.as_bytes()[4..6].try_into().ok()?)?,
            a: hex2(hex.as_bytes()[6..8].try_into().ok()?)?,
        }),
        _ => None,
    }
}

fn hex2(bytes: [u8; 2]) -> Option<u8> {
    let hi = hex1(bytes[0])?;
    let lo = hex1(bytes[1])?;
    Some(hi << 4 | lo)
}

fn hex1(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn merge(theme: &mut KittyTheme, other: KittyTheme) {
    if theme.font_family.is_none() {
        theme.font_family = other.font_family;
    }
    if theme.font_size.is_none() {
        theme.font_size = other.font_size;
    }
    if theme.background_opacity.is_none() {
        theme.background_opacity = other.background_opacity;
    }
    if theme.foreground.is_none() {
        theme.foreground = other.foreground;
    }
    if theme.background.is_none() {
        theme.background = other.background;
    }
    for i in 0..16 {
        if theme.palette[i].is_none() {
            theme.palette[i] = other.palette[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_parses_full_file() {
        let text = "\
font_family JetBrains Mono
font_size 14
background_opacity 0.85
foreground #cdd6f4
background #1e1e2e
color0 #45475a
color1 #f38ba8
color2 #a6e3a1
color3 #f9e2af
color4 #89b4fa
color5 #f5c2e7
color6 #94e2d5
color7 #bac2de
";
        let dir = std::env::temp_dir();
        let path = dir.join("kitty-test-full.conf");
        std::fs::write(&path, text).unwrap();
        let t = load(&path);
        assert_eq!(t.font_family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(t.font_size, Some(14.0));
        assert_eq!(t.background_opacity, Some(0.85));
        assert_eq!(
            t.foreground,
            Some(Rgba8 {
                r: 0xcd,
                g: 0xd6,
                b: 0xf4,
                a: 0xff
            })
        );
        assert_eq!(
            t.background,
            Some(Rgba8 {
                r: 0x1e,
                g: 0x1e,
                b: 0x2e,
                a: 0xff
            })
        );
        assert_eq!(t.palette[0], Some(Rgba8 { r: 0x45, g: 0x47, b: 0x5a, a: 0xff }));
        assert_eq!(t.palette[6], Some(Rgba8 { r: 0x94, g: 0xe2, b: 0xd5, a: 0xff }));
        assert_eq!(t.palette[15], None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_resolves_include() {
        let inc = "color0 #ff0000\nfont_size 18\n";
        let dir = std::env::temp_dir();
        let inc_path = dir.join("kitty-test-include.conf");
        std::fs::write(&inc_path, inc).unwrap();

        let main = format!("include {}\ncolor0 #00ff00\n", inc_path.display());
        let main_path = dir.join("kitty-test-main.conf");
        std::fs::write(&main_path, main).unwrap();

        let t = load(&main_path);
        // main's color0 takes precedence over include's
        assert_eq!(t.palette[0], Some(Rgba8 { r: 0x00, g: 0xff, b: 0x00, a: 0xff }));
        // font_size comes from include (not set in main)
        assert_eq!(t.font_size, Some(18.0));

        let _ = std::fs::remove_file(&main_path);
        let _ = std::fs::remove_file(&inc_path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let t = load(Path::new("/nonexistent/kitty.conf"));
        assert_eq!(t.font_family, None);
        assert_eq!(t.font_size, None);
        assert_eq!(t.background_opacity, None);
        assert_eq!(t.foreground, None);
        assert_eq!(t.background, None);
        assert!(t.palette.iter().all(Option::is_none));
    }

    #[test]
    fn load_ignores_unknown_keys() {
        let text = "\
font_size 16
scrollback_lines 10000
background_image foo.png
color9 #abcdef
";
        let dir = std::env::temp_dir();
        let path = dir.join("kitty-test-unknown.conf");
        std::fs::write(&path, text).unwrap();
        let t = load(&path);
        assert_eq!(t.font_size, Some(16.0));
        assert_eq!(t.palette[9], Some(Rgba8 { r: 0xab, g: 0xcd, b: 0xef, a: 0xff }));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn named_colors_not_parsed() {
        let text = "foreground red\ncolor0 blue\n";
        let dir = std::env::temp_dir();
        let path = dir.join("kitty-test-named.conf");
        std::fs::write(&path, text).unwrap();
        let t = load(&path);
        assert_eq!(t.foreground, None, "named X11 colors are not supported");
        assert_eq!(t.palette[0], None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn opacity_clamped_to_0_1() {
        let text = "background_opacity 2.5\n";
        let dir = std::env::temp_dir();
        let path = dir.join("kitty-test-opacity.conf");
        std::fs::write(&path, text).unwrap();
        let t = load(&path);
        assert_eq!(t.background_opacity, Some(1.0));
        let _ = std::fs::remove_file(&path);
    }
}
