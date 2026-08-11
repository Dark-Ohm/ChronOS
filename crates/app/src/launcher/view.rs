//! Launcher overlay view: search input + fuzzy-matched result list.
//!
//! Redesigned per `docs/design/Chronos-OSD-Launcher.dc.html` (T261):
//! centered 720px card on a gradient backdrop, with header, search row,
//! scrollable result list, and a static footer (luau badge + reload dot).

use gpui::{
    self, linear_color_stop, linear_gradient, svg, App, Focusable, ImageSource, Render,
    SharedString, Window, div, img, prelude::*, px,
};

use chronos_services::{AppEntry, Service};
use chronos_ui::{Theme, WindowRootExt};

use crate::icon_resolution::resolve_icon;
use crate::launcher::launch::launch;
use crate::launcher::search::FuzzySearch;
use crate::state;

const INPUT_HEIGHT: f32 = 44.;
const ROW_HEIGHT: f32 = 42.;
const MAX_VISIBLE_ROWS: usize = 10;

/// Centered overlay view showing fuzzy search results over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    pattern: String,
    selected: usize,
    results: Vec<AppEntry>,
    focus: gpui::FocusHandle,
}

impl LauncherView {
    /// Build a launcher view seeded with the current desktop entries from the
    /// applications service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let svc = state::AppState::applications(cx);
        let entries = svc.get().entries;
        let mut search = FuzzySearch::new();
        search.set_items(entries);

        let mut view = Self {
            search,
            pattern: String::new(),
            selected: 0,
            results: Vec::new(),
            focus: cx.focus_handle(),
        };
        view.refresh_results();

        // Subscribe to desktop entry changes — live updates without restart.
        let signal = state::AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this, state, cx| {
            this.search.set_items(state.entries);
            this.refresh_results();
            cx.notify();
        });

        view
    }

    /// Focus the launcher's input field.
    pub fn focus_input(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }

    fn refresh_results(&mut self) {
        self.search.update_pattern(&self.pattern);
        self.results = self
            .search
            .results(MAX_VISIBLE_ROWS)
            .into_iter()
            .cloned()
            .collect();
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
    }

    fn handle_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
        let key = event.keystroke.key.as_str();

        match key {
            "escape" => {
                crate::launcher::close_this(window, cx);
            }
            "enter" => {
                if let Some(entry) = self.results.get(self.selected).cloned() {
                    if let Err(err) = launch(&entry.exec) {
                        tracing::error!("Failed to launch {}: {:#}", entry.name, err);
                    }
                }
                crate::launcher::close_this(window, cx);
            }
            "up" => {
                if self.selected > 0 {
                    self.selected -= 1;
                    window.refresh();
                }
            }
            "down" | "tab" => {
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                    window.refresh();
                }
            }
            "backspace" => {
                self.pattern.pop();
                self.selected = 0;
                self.refresh_results();
                window.refresh();
            }
            _ => {
                if let Some(ch) = event.keystroke.key_char.as_ref() {
                    if !event.keystroke.modifiers.alt
                        && !event.keystroke.modifiers.platform
                        && !event.keystroke.modifiers.control
                    {
                        self.pattern.push_str(ch);
                        self.selected = 0;
                        self.refresh_results();
                        window.refresh();
                    }
                }
            }
        }
    }
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);

        let pattern: SharedString = self.pattern.clone().into();
        let selected = self.selected;
        let has_pattern = !self.pattern.is_empty();
        let is_empty = self.results.is_empty();
        let results: Vec<(usize, SharedString, AppEntry)> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, e)| (i, SharedString::from(e.name.clone()), e.clone()))
            .collect();

        div()
            .track_focus(&self.focus)
            .window_font(theme)
            .size_full()
            .bg(theme.bg.tertiary)
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.text.primary)
            .relative()
            // Backdrop glow: linear gradient approximating the radial-gradient
            // in the reference (the fork has no radial gradient primitive).
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .right(px(0.))
                    .h(px(360.))
                    .bg(linear_gradient(
                        0.0,
                        linear_color_stop(theme.accent.primary.opacity(0.07), 0.0),
                        linear_color_stop(theme.transparent, 0.6),
                    )),
            )
            .child(self.render_card(theme, pattern, selected, has_pattern, is_empty, results))
            .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
    }
}

impl LauncherView {
    fn render_card(
        &self,
        theme: &Theme,
        pattern: SharedString,
        selected: usize,
        has_pattern: bool,
        is_empty: bool,
        results: Vec<(usize, SharedString, AppEntry)>,
    ) -> impl IntoElement {
        div()
            .w(px(720.))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .rounded_lg()
            .shadow(card_shadow(theme))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_header(theme))
            .child(self.render_search(theme, pattern, has_pattern))
            .child(self.render_results(theme, selected, is_empty, results))
            .child(self.render_footer(theme))
    }

    fn render_header(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(14.))
            .h(px(42.))
            .border_b_1()
            .border_color(theme.border.subtle)
            // Sigil.
            .child(
                svg()
                    .path("icons/chronos-sigil.svg")
                    .size(px(18.))
                    .text_color(theme.accent.primary),
            )
            // Title "launcher".
            .child(
                div()
                    .font_family(theme.font_mono)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text.primary)
                    .child("launcher"),
            )
            // Separator.
            .child(div().w(px(1.)).h(px(15.)).bg(theme.border.subtle))
            // Mode chip "APPS" (static — scope cycling out of T261).
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(8.))
                    .h(px(22.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.border.subtle)
                    .text_color(theme.text.muted)
                    .child(
                        div()
                            .font_family(theme.font_mono)
                            .text_size(theme.font_sizes.xs)
                            .child("APPS"),
                    ),
            )
            .child(div().flex_1())
            // Hotkey hint: SUPER SPACE.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.xs)
                    .text_color(theme.text.faint)
                    .child("invoke")
                    .child(kbd("SUPER", theme))
                    .child(kbd("SPACE", theme)),
            )
    }

    fn render_search(
        &self,
        theme: &Theme,
        pattern: SharedString,
        has_pattern: bool,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(14.))
            .h(px(INPUT_HEIGHT))
            .border_b_1()
            .border_color(theme.border.subtle.opacity(0.5))
            // Search icon (no magnifier asset — reuse sigil as placeholder).
            .child(
                svg()
                    .path("icons/chronos-sigil.svg")
                    .size(px(18.))
                    .text_color(theme.text.muted),
            )
            // Pattern display (fake input, matches current architecture).
            .child(
                div()
                    .flex_1()
                    .text_lg()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text.primary)
                    .child(if has_pattern {
                        div().child(pattern)
                    } else {
                        div().text_color(theme.text.faint).child("Search applications, commands, files…")
                    }),
            )
            // Clear button (visible only when pattern is non-empty).
            .when(has_pattern, |el| {
                el.child(
                    div()
                        .size(px(24.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text.faint)
                        .child(svg().path("icons/x.svg").size(px(14.))),
                )
            })
    }

    fn render_results(
        &self,
        theme: &Theme,
        selected: usize,
        is_empty: bool,
        results: Vec<(usize, SharedString, AppEntry)>,
    ) -> impl IntoElement {
        div()
            .id("launcher-results")
            .flex_1()
            .overflow_y_scroll()
            .pt(px(4.))
            .pb(px(8.))
            .px(px(8.))
            .children(results.into_iter().map(|(i, name, entry)| {
                let is_selected = i == selected;
                let entry_for_click = entry.clone();
                let icon_el = resolve_app_icon(&entry, theme);

                div()
                    .id(format!("launcher-row-{i}"))
                    .h(px(ROW_HEIGHT))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .rounded(px(8.))
                    .relative()
                    .cursor_pointer()
                    .text_color(theme.text.primary)
                    .when(is_selected, |el| el.bg(theme.bg.selection))
                    .when(!is_selected, |el| {
                        el.hover(|s| s.bg(theme.interactive.hover))
                    })
                    // Accent bar on the left of the selected row.
                    .when(is_selected, |el| {
                        el.child(
                            div()
                                .absolute()
                                .left(px(0.))
                                .top(px(9.))
                                .bottom(px(9.))
                                .w(px(3.))
                                .rounded(px(2.))
                                .bg(theme.accent.primary),
                        )
                    })
                    // App icon (SVG via system theme, or letter fallback).
                    .child(
                        div()
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(if is_selected {
                                theme.accent.primary
                            } else {
                                theme.text.muted
                            })
                            .child(icon_el),
                    )
                    // Name.
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .min_w(px(0.))
                            .when(is_selected, |el| el.text_color(theme.accent.primary))
                            .child(name),
                    )
                    .on_click(move |_event, window, cx: &mut App| {
                        if let Err(err) = launch(&entry_for_click.exec) {
                            tracing::error!(
                                "Failed to launch {}: {:#}",
                                entry_for_click.name,
                                err
                            );
                        }
                        crate::launcher::close_this(window, cx);
                    })
            }))
            .when(is_empty, |el| {
                el.child(
                    div()
                        .py(px(34.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text.faint)
                        .text_sm()
                        .child("No matches"),
                )
            })
    }

    fn render_footer(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .h(px(38.))
            .border_t_1()
            .border_color(theme.border.subtle)
            .bg(theme.bg.tertiary)
            // Tune button (static — fine-tune panel is out of T261 scope).
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .px(px(11.))
                    .h(px(30.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(theme.border.subtle)
                    .text_color(theme.text.muted)
                    .child(
                        div()
                            .font_family(theme.font_mono)
                            .text_size(theme.font_sizes.sm)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("tune"),
                    ),
            )
            .child(div().flex_1())
            // Luau plugin badge.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.xs)
                    .text_color(theme.text.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_color(theme.accent.secondary)
                            .child(
                                svg().path("icons/chronos-sigil.svg")
                                    .size(px(13.))
                                    .text_color(theme.accent.secondary),
                            )
                            .child("luau"),
                    )
                    .child("plugin · hot-reload")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_color(theme.text.faint)
                            .child(div().size(px(6.)).rounded_full().bg(theme.accent.secondary))
                            .child("live"),
                    ),
            )
    }
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}

/// Card shadow for the launcher popup — softer than bar elevation, tuned
/// for a floating overlay.
fn card_shadow(theme: &Theme) -> Vec<gpui::BoxShadow> {
    vec![
        gpui::BoxShadow::new(px(0.), px(10.), theme.bg.primary.opacity(0.45)).blur_radius(px(40.)),
        gpui::BoxShadow::new(px(0.), px(1.), theme.bg.primary.opacity(0.3)).blur_radius(px(2.)),
    ]
}

/// Keyboard key chip (mono, bordered, slight bottom accent).
fn kbd(label: &'static str, theme: &Theme) -> impl IntoElement {
    div()
        .font_family(theme.font_mono)
        .text_size(px(10.))
        .px(px(6.))
        .pt(px(1.))
        .pb(px(2.))
        .rounded(px(5.))
        .border_1()
        .border_color(theme.border.subtle)
        .bg(theme.bg.tertiary)
        .text_color(theme.text.primary)
        .child(label)
}

/// Resolve an application's icon: try the .desktop Icon= field via the system
/// icon theme first, fall back to a letter glyph.
fn resolve_app_icon(entry: &AppEntry, theme: &Theme) -> gpui::AnyElement {
    if let Some(name) = entry.icon.as_deref() {
        if let Some(path_buf) = resolve_icon(name) {
            let src: ImageSource = path_buf.to_string_lossy().to_string().into();
            return img(src).size(px(18.)).into_any_element();
        }
    }
    // Fallback: first letter of the app name.
    let letter = entry
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    div()
        .size(px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .bg(theme.bg.elevated)
        .child(div().text_sm().text_color(theme.text.primary).child(letter))
        .into_any_element()
}
