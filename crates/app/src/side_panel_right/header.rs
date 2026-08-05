//! Panel header — active window title (Hyprland-backed, T256).
//! Styles from `design/System Sidebar.dc.html` (header block).
//!
//! T256: the close button used to sit on the same row as the title and read
//! as a fake OS-window ⨯. Now that the title is real, that visual adjacency
//! would have been a regression in plausibility — so the close button is
//! gone from this header. The panel still has explicit dismiss paths
//! (bar-widget toggle, hotkey, click-away, hover-leave debounce in peek —
//! see `side_panel_right/mod.rs:1-12`). If a discoverable in-tab close
//! affordance becomes a real UX complaint, add it in a dedicated visual
//! location — but never again next to a window title.

use gpui::{App, IntoElement, div, prelude::*, px};
use chronos_services::ActiveWindow;
use chronos_ui::Theme;

/// Fallback shown when Hyprland reports no focused window (desktop-only
/// focus, special workspaces, listener just started). NEVER a faked
/// third-party class name — T256: hardcoded "kitty" survived 12 reports
/// because the real kitty was on screen in the captured frames. "Desktop"
/// is Hyprland's natural empty-focus label.
const NO_ACTIVE_WINDOW: &str = "Desktop";

/// Pick the title the header should render. Owned `String` because GPUI's
/// element tree requires values to outlive the render call; one allocation
/// per header render, and the panel re-renders only on Hyprland focus change
/// (T256), not per frame. Pure — unit-testable without `cx`.
pub(crate) fn pick_title(active: Option<&ActiveWindow>) -> String {
    match active {
        Some(w) => w.title.clone(),
        None => NO_ACTIVE_WINDOW.to_string(),
    }
}

pub fn render_header(cx: &App, active_window: Option<&ActiveWindow>) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let title = pick_title(active_window);
    div()
        .flex()
        .items_center()
        .flex_none()
        .px(px(14.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme.border.subtle)
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text.secondary)
                .child(title),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_title_returns_desktop_when_none() {
        let t = pick_title(None);
        assert_eq!(t, "Desktop");
        assert_ne!(t, "kitty", "T256 regression guard: never fakes a window class");
    }

    #[test]
    fn pick_title_returns_window_title_when_some() {
        let w = ActiveWindow {
            title: "Freebuff: пиши отчет".to_string(),
            class: "codebuff".to_string(),
            address: "0x123".to_string(),
        };
        assert_eq!(pick_title(Some(&w)), "Freebuff: пиши отчет");
    }

    #[test]
    fn pick_title_preserves_empty_title_honestly() {
        let w = ActiveWindow {
            title: String::new(), // e.g. webview with no document title
            class: "firefox".to_string(),
            address: "0x456".to_string(),
        };
        assert_eq!(pick_title(Some(&w)), "");
    }

    #[test]
    fn pick_title_does_not_use_class_as_fallback() {
        // Class is the WM app class (e.g. "kitty", "firefox") — we deliberately
        // do NOT fall back to it on empty title, because that's exactly the
        // T256 lie. Empty title means the window really has no title — render
        // empty, never the class.
        let w = ActiveWindow {
            title: String::new(),
            class: "kitty".to_string(),
            address: "0x789".to_string(),
        };
        let t = pick_title(Some(&w));
        assert_ne!(t, "kitty", "class MUST NOT leak as title");
        assert_eq!(t, "");
    }
}
