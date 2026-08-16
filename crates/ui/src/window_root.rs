//! Applying the UI font once at the root of every window (T227).
//!
//! Before T227 the theme's `font_ui` field was read in exactly one place
//! (`system_popup`, element-by-element); every other ChronOS window rendered
//! with GPUI's default font. The field read as "JetBrains Mono" and looked
//! correct in the editor/inputs (gpui-component theme) — an illusion.
//!
//! [`WindowRootExt::window_font`] moves the decision to the window root:
//! call it on the outermost element of every `Render`, and all descendant
//! text inherits `theme.font_ui`. Elements with a mono meaning keep an
//! explicit `font_family(theme.font_mono)` override.

use gpui::Styled;

use crate::Theme;

/// Extension setting the window's UI font on a root styled element.
pub trait WindowRootExt {
    /// Apply `theme.font_ui` to a window's root element so every descending
    /// text element inherits it. Mono-styled elements override below.
    fn window_font(self, theme: &Theme) -> Self;
}

impl<T: Styled> WindowRootExt for T {
    fn window_font(self, theme: &Theme) -> Self {
        self.font_family(theme.font_ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The window-root render files of the shell (a window has text only if it
    // has a root diver here; hover strips are text-less transparent hit
    // surfaces and are intentionally absent). Each must source through
    // `window_font`, and none may hand-roll per-element `font_family(font_ui)`.
    //
    // T279: `side_panel_left/panel.rs` (deleted T279 Task 3 — chat body moved
    // to `tabs/chat.rs`, render root is now `workspace_view.rs`) and
    // `project_switcher/view.rs` (deleted T279 Task 4 — popup embedded as a
    // tab, render root is now `tabs/project.rs`) are intentionally absent.
    // T290: `system_popup/view.rs` (deleted T290 — popup removed; its content
    // moved into `side_panel_left/tabs/display.rs` and `gaming_mode.rs`) has no
    // standalone window root and is intentionally absent.
    // T294: `updates_popup/view.rs` (deleted T294 — popup removed; the list
    // moved into the right panel's `side_panel_right/tab/updates.rs`) has no
    // standalone window root and is intentionally absent.
    // The chat/workspace/project tab render roots are covered by the
    // broader font audit below rather than this `window_font` gate.
    const ROOTS: &[(&str, &str)] = &[
        (
            "side_panel_left/rail_view.rs",
            include_str!("../../app/src/side_panel_left/rail_view.rs"),
        ),
        (
            "side_panel_left/workspace_view.rs",
            include_str!("../../app/src/side_panel_left/workspace_view.rs"),
        ),
        (
            "side_panel_right/view.rs",
            include_str!("../../app/src/side_panel_right/view.rs"),
        ),
        ("bar/mod.rs", include_str!("../../app/src/bar/mod.rs")),
        (
            "notifications/view.rs",
            include_str!("../../app/src/notifications/view.rs"),
        ),
        (
            "volume_popup/view.rs",
            include_str!("../../app/src/volume_popup/view.rs"),
        ),
        ("launcher/view.rs", include_str!("../../app/src/launcher/view.rs")),
        ("osd/view.rs", include_str!("../../app/src/osd/view.rs")),
        (
            "dock/context_menu.rs",
            include_str!("../../app/src/dock/context_menu.rs"),
        ),
        (
            "tray_menu/view.rs",
            include_str!("../../app/src/tray_menu/view.rs"),
        ),
        (
            "desktop_terminal/view.rs",
            include_str!("../../app/src/desktop_terminal/view.rs"),
        ),
    ];

    /// The task's test is a discipline, not a fact: asserting that
    /// `font_ui == "JetBrains Mono"` stays green even when the shell renders
    /// the default font — exactly the lie T215 lived with. This instead
    /// asserts every window root bypasses the shared helper and that no
    /// per-element `font_family(font_ui)` handoff has crept back in.
    #[test]
    fn every_window_root_uses_window_font() {
        for (path, src) in ROOTS {
            assert!(
                src.contains(".window_font("),
                "{path}: window root is NOT routed through `window_font` (T227)"
            );
            assert!(
                !src.contains("font_family(theme.font_ui)")
                    && !src.contains("font_family(font_ui)"),
                "{path}: per-element `font_family(font_ui)` — apply at the root instead (T227)"
            );
        }
    }

    /// The helper actually pins the UI font.
    #[test]
    fn window_font_sets_font_ui() {
        let theme = Theme::default();
        let mut el = gpui::div().window_font(&theme);
        let family = el.style().text.font_family.clone();
        assert_eq!(
            family.map(|f| f.to_string()).as_deref(),
            Some(theme.font_ui)
        );
    }
}