//! Wallpaper / waytrogen companion card for the System tab.
//!
//! Shows: current wallpaper path (if set), "Next" hotpath button,
//! "Open gallery (waytrogen)" primary action, or install CTA when missing.

use gpui::{App, ElementId, IntoElement, SharedString, div, prelude::*, px};
use chronos_ui::Theme;

use crate::side_panel_right::surfaces;
use chronos_services::WallpaperState;


/// Wallpaper path display — truncate to basename for the card.
fn wallpaper_label(state: &WallpaperState) -> String {
    match &state.current {
        Some(path) => {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            // Show parent dir hint if short enough.
            if let Some(parent) = path.parent() {
                let parent_name = parent
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !parent_name.is_empty() && parent_name.len() < 20 {
                    format!("{parent_name}/{name}")
                } else {
                    name
                }
            } else {
                name
            }
        }
        None => "not set".to_string(),
    }
}

pub fn render_wallpaper_card(
    state: &WallpaperState,
    waytrogen_available: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = *Theme::global(cx);
    let label = wallpaper_label(state);

    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .rounded(px(9.))
        .bg(surfaces::card(&theme))
        .border_1()
        .border_color(theme.border.subtle)
        // Title row
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.status.info)
                        .child("Wallpapers"),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(theme.text.muted)
                        .child(label),
                ),
        )
        // Button row
        .child(
            div()
                .flex()
                .gap(px(6.))
                .child(action_button(ElementId::Name(SharedString::from("wallpaper-next")), "Next", &theme, {
                        move |_, _, cx: &mut gpui::App| {
                            tracing::info!("wallpaper_card: Next clicked");
                            crate::wallpaper_ctl::next(cx);
                        }
                    },
                ))
                .when(waytrogen_available, |row| {
                    row.child(action_button(ElementId::Name(SharedString::from("wallpaper-gallery")), "Open waytrogen", &theme, {
                            move |_, _, cx: &mut gpui::App| {
                                tracing::info!("wallpaper_card: Open waytrogen clicked");
                                if let Err(e) = crate::wallpaper_ctl::open_waytrogen_gallery() {
                                    tracing::warn!("wallpaper_card: {e}");
                                    return;
                                }
                                // Delayed resync (same idea as IPC gallery arm).
                                let wallpaper =
                                    crate::state::AppState::wallpaper(cx).clone();
                                cx.spawn(async move |cx| {
                                    cx.background_executor()
                                        .timer(std::time::Duration::from_secs(3))
                                        .await;
                                    wallpaper.refresh();
                                })
                                .detach();
                            }
                        },
                    ))
                })
                .when(!waytrogen_available, |row| {
                    row.child(install_cta(&theme))
                }),
        )
}

fn action_button(
    id: ElementId,
    label: &'static str,
    theme: &Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .py(px(6.))
        .rounded(px(5.))
        .border_1()
        .border_color(theme.text.disabled)
        .text_size(px(10.))
        .text_color(theme.text.primary)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.border.default))
        .on_click(on_click)
        .child(label)
}

fn install_cta(theme: &Theme) -> impl IntoElement {
    div()
        .flex_1()
        .py(px(6.))
        .rounded(px(5.))
        .border_1()
        .border_color(theme.text.disabled)
        .text_size(px(9.5))
        .text_color(theme.text.muted)
        .flex()
        .items_center()
        .justify_center()
        .child("waytrogen not found — yay -S waytrogen")
}
