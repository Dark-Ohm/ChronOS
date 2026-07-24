//! Встроенные схемы оформления ChronOS.

use anyhow::Result;

use super::{Base16Colors, Theme};

/// Описание одной схемы: имя, краткое описание и готовая [`Theme`].
pub struct ThemeScheme {
    pub name: &'static str,
    pub description: &'static str,
    pub theme: Theme,
}

impl ThemeScheme {
    fn new(name: &'static str, description: &'static str, theme: Theme) -> Self {
        Self {
            name,
            description,
            theme,
        }
    }
}

/// Базовая тёмная палитра ChronOS (используется как [`Theme::default`]).
///
/// Вынесена как константа Base16, чтобы `default_scheme` и `Theme::default`
/// опирались на один и тот же источник истинны, без ручного дублирования.
pub const DEFAULT_BASE16: [&str; 16] = [
    "1e1e2e", // base00 bg.primary
    "25253b", // base01 bg.secondary
    "181825", // base02 bg.tertiary
    "313244", // base03 bg.elevated
    "45475a", // base04 text.disabled / interactive.active
    "6c7086", // base05 text.muted
    "a6adc8", // base06 text.secondary
    "cdd6f4", // base07 text.primary
    "f38ba8", // base08 status.error (Catppuccin Mocha maroon/red)
    "f9e2af", // base09 status.warning (Catppuccin Mocha yellow)
    "94e2d5", // base0a status.warning/alt (Catppuccin Mocha teal)
    "a6e3a1", // base0b status.success (Catppuccin Mocha green)
    "89b4fa", // base0c status.info (Catppuccin Mocha blue)
    "007acc", // base0d accent.primary / border.focused
    "cba6f7", // base0e accent.hover / toggle_on_hover
    "f38ba8", // base0f active/emph
];

fn default_scheme() -> ThemeScheme {
    ThemeScheme::new(
        "Default",
        "Тёмная схема ChronOS (Mocha-подобная)",
        Theme::default(),
    )
}

fn light_scheme() -> ThemeScheme {
    // Светлая схема ChronOS «Light C» (айдентика, НЕ инверсия Latte).
    // Эталон: design/Project Switcher.dc.html, вариант Light C
    // (lightBase + override-блок «Light — popup open (Light C, accepted)»).
    // Принцип: холодная сине-лавандовая база, индиго-текст, неон — только
    // в линиях/деталях. Акцент НЕ переопределяется — остаётся #007acc
    // (правило design.md/DECISIONS: светлая тема не красит акцент).
    let mut theme = Theme::default();

    // Поверхности — холодная сине-лавандовая база Light C.
    theme.bg.primary = hex("dde0f2"); // pageBg — базовый фон страницы/окна
    theme.is_light = true;
    theme.bg.secondary = hex("e6e9fa"); // cardBg (accepted) — поверхность карточки/попапа
    theme.bg.tertiary = hex("eceefa"); // cardBase (lightBase) — фон пилюли/свёрнутого
    theme.bg.elevated = hex("e0e3f4"); // hoverBg — приподнятый слой/hover-фон

    // Текст — глубокий индиго, НЕ чёрный.
    theme.text.primary = hex("2c2e4a"); // textPrimary — основной индиго-текст
    theme.text.secondary = hex("5a5d80"); // textMuted — приглушённый (вторичный)
    theme.text.muted = hex("7d80a6"); // chevron — ещё приглушённый (третичный)
    // disabled/placeholder — мокап не диктует, выводим по духу палитры
    // (разбеливание muted к лавандовому). См. отчёт «додумано».
    theme.text.disabled = hex("9a9dc0"); // додумано — disabled индиго-лавандовый
    theme.text.placeholder = hex("9a9dc0"); // додумано — placeholder = disabled

    // Бордеры — из палитры Light C.
    theme.border.default = hex("c4c8e6"); // cardBorder — разделитель карточки
    // subtle — тоньше default (мокап не диктует, выводим осветлением).
    theme.border.subtle = hex("d4d7ee"); // додумано — subtle-разделитель
    // focused — акцентная линия (неон в деталях, не в заливке).
    theme.border.focused = hex("007acc"); // accent — glow-ребро/фокус-контур

    // Акцент НЕ переопределяется — правило design.md/DECISIONS.
    // accent.primary/selection/hover остаются из Theme::default (#007acc/
    // #007acc/#1f9bdc) — на светлом фоне читаются, MVP.

    // Interactive — из палитры Light C по ролям.
    theme.interactive.default = hex("c4c8e6"); // cardBorder — контур контрола
    theme.interactive.hover = hex("e0e3f4"); // hoverBg — hover-состояние
    // active — чуть глубже hover (мокап не диктует, выводим затемнением).
    theme.interactive.active = hex("d4d7ee"); // додумано — active-состояние
    // toggle_on — акцент (неон в деталях), toggle_on_hover — дефолтный #1f9bdc.
    theme.interactive.toggle_on = hex("007acc"); // accent — включённый тоггл

    // status.* — Catppuccin LATTE, не Mocha. Пастельные Mocha-статусы
    // рассчитаны на тёмный фон: живой смок 2026-07-20 показал, что
    // `status.warning` (#f9e2af) как ЦВЕТ ТЕКСТА в виджете обновлений на
    // светлом баре практически невидим. Latte — те же смыслы в той же
    // палитре, но с контрастом под светлую поверхность. Заливки (бейджи)
    // темнеют вместе с этим, а контент поверх них ведёт `on_fill`.
    theme.status.error = hex("d20f39"); // Latte red
    theme.status.warning = hex("df8e1d"); // Latte peach/yellow
    theme.status.success = hex("40a02b"); // Latte green
    theme.status.info = hex("1e66f5"); // Latte blue

    ThemeScheme::new("Light", "Светлая схема ChronOS (Light C)", theme)
}

fn solarized_dark_scheme() -> Result<ThemeScheme> {
    // Реальная палитра Solarized Dark, маппится через Base16Colors.
    let colors = Base16Colors::from_hex(&[
        "002b36", // base00
        "073642", // base01
        "586e75", // base02
        "657b83", // base03
        "839496", // base04
        "93a1a1", // base05
        "eee8d5", // base06
        "fdf6e3", // base07
        "dc322f", // base08 error
        "cb4b16", // base09 warning
        "b58900", // base0a warning/alt
        "859900", // base0b success
        "2aa198", // base0c info
        "268bd2", // base0d accent
        "6c71c4", // base0e accent/hover
        "d33682", // base0f
    ])?;
    Ok(ThemeScheme::new(
        "Solarized Dark",
        "Solarized Dark через Base16",
        colors.to_theme(),
    ))
}

#[inline]
fn hex(s: &str) -> gpui::Hsla {
    super::parse_hex(s).expect("встроенный hex валиден")
}

/// Возвращает все встроенные схемы (дефолт, светлая, solarized dark).
pub fn builtin_schemes() -> Vec<ThemeScheme> {
    let mut out = vec![default_scheme(), light_scheme()];
    if let Ok(solarized) = solarized_dark_scheme() {
        out.push(solarized);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_scheme_uses_light_c_palette() {
        let s = light_scheme();
        assert_eq!(s.name, "Light");
        assert_eq!(s.description, "Светлая схема ChronOS (Light C)");
        // Якоря Light C из мокапа.
        assert_eq!(s.theme.bg.primary, hex("dde0f2")); // pageBg
        assert_eq!(s.theme.bg.secondary, hex("e6e9fa")); // cardBg (accepted)
        assert_eq!(s.theme.bg.tertiary, hex("eceefa")); // cardBase
        assert_eq!(s.theme.bg.elevated, hex("e0e3f4")); // hoverBg
        assert_eq!(s.theme.text.primary, hex("2c2e4a")); // textPrimary
        assert_eq!(s.theme.text.secondary, hex("5a5d80")); // textMuted
        assert_eq!(s.theme.text.muted, hex("7d80a6")); // chevron
        assert_eq!(s.theme.border.default, hex("c4c8e6")); // cardBorder
        // Акцент НЕ переопределяется — остаётся #007acc из дефолта.
        assert_eq!(s.theme.accent.primary, hex("007acc"));
        assert_eq!(s.theme.border.focused, hex("007acc"));
        assert_eq!(s.theme.interactive.toggle_on, hex("007acc"));
    }

    #[test]
    fn light_scheme_status_is_latte_not_mocha() {
        // Пастельные Mocha-статусы невидимы как текст на светлом фоне
        // (живой смок 2026-07-20: `↑ 19` бледно-жёлтым на светлом баре).
        // Светлая схема обязана нести Latte-статусы, а не дефолтные.
        let s = light_scheme();
        let d = Theme::default();
        assert_ne!(s.theme.status, d.status);
        assert_eq!(s.theme.status.warning, hex("df8e1d"));
        assert_eq!(s.theme.status.error, hex("d20f39"));

        // Контраст: Latte-статусы темнее светлой поверхности (#dde0f2),
        // иначе текст ими снова поплывёт.
        assert!(s.theme.status.warning.l < s.theme.bg.primary.l);
        assert!(s.theme.status.error.l < s.theme.bg.primary.l);
    }

    #[test]
    fn select_scheme_default_when_unset() {
        let t = Theme::select_scheme(None);
        assert_eq!(t, Theme::default());
        let t = Theme::select_scheme(Some(String::new()));
        assert_eq!(t, Theme::default());
        let t = Theme::select_scheme(Some("   ".to_string()));
        assert_eq!(t, Theme::default());
    }

    #[test]
    fn select_scheme_by_name_case_insensitive() {
        let light_lower = Theme::select_scheme(Some("light".to_string()));
        let light_upper = Theme::select_scheme(Some("LIGHT".to_string()));
        let light_mixed = Theme::select_scheme(Some("LiGhT".to_string()));
        let expected = light_scheme().theme;
        assert_eq!(light_lower, expected);
        assert_eq!(light_upper, expected);
        assert_eq!(light_mixed, expected);

        let default_named = Theme::select_scheme(Some("default".to_string()));
        assert_eq!(default_named, Theme::default());
    }

    #[test]
    fn select_scheme_garbage_falls_back_to_default() {
        let t = Theme::select_scheme(Some("nope-not-a-scheme".to_string()));
        assert_eq!(t, Theme::default());
    }

    #[test]
    fn is_light_flag_matches_scheme() {
        assert!(!default_scheme().theme.is_light);
        assert!(light_scheme().theme.is_light);
    }

    #[test]
    fn builtin_schemes_contains_default_and_light() {
        let names: Vec<&'static str> = builtin_schemes().iter().map(|s| s.name).collect();
        assert!(names.contains(&"Default"));
        assert!(names.contains(&"Light"));
    }
}
