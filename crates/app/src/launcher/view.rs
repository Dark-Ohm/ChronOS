//! Launcher overlay view: search input + fuzzy-matched result list.
//!
//! Redesigned per `docs/design/Chronos-OSD-Launcher.dc.html` (T261):
//! centered 720px card on a gradient backdrop, with header, search row,
//! scrollable result list, and a footer (luau badge + reload dot).
//!
//! T275 (волна 1): the search field is now a real `gpui-component` `Input`
//! bound to an `InputState` — caret, cursor movement, selection, IME, paste,
//! ctrl+w all come from the component (no hand-rolled caret). List navigation
//! (up/down/tab/enter/escape) stays on the launcher; text editing is delegated
//! to the `Input`. Results are ranked by frecency (T275 Часть C) and launching
//! records frecency. Right-click on a row opens a Pin/Unpin menu (T275 Часть D).

use gpui::{
    self, App, Bounds, Entity, ImageSource, MouseButton, Render, ScrollHandle, SharedString, Size,
    Subscription, Window, div, img, linear_color_stop, linear_gradient, prelude::*, px, svg,
};

use chronos_services::applications::frecency;
use chronos_services::{AppEntry, Service};
use chronos_ui::{Theme, WindowRootExt};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::icon_resolution::resolve_icon;
use crate::launcher::launch::launch;
use crate::launcher::pin_menu;
use crate::launcher::search::FuzzySearch;
use crate::state;

const INPUT_HEIGHT: f32 = 44.;
const ROW_HEIGHT: f32 = 42.;
/// Soft cap on the rendered result set — NOT on visibility. The list scrolls,
/// so this only bounds per-frame render cost. Was a hard 10-row cap
/// (`MAX_VISIBLE_ROWS`) which made the scroll container dead (T265-0).
const MAX_RESULTS: usize = 200;

/// Centered overlay view showing fuzzy search results over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    /// Real editable text buffer (replaces the old `String` pattern).
    input: Entity<InputState>,
    /// Mirror of the input text, updated on `InputEvent::Change`.
    pattern: String,
    selected: usize,
    results: Vec<AppEntry>,
    scroll: ScrollHandle,
    /// Subscription to `InputState` change events (drives re-search).
    _input_sub: Subscription,
}

impl LauncherView {
    /// Build a launcher view seeded with the current desktop entries from the
    /// applications service and the live `InputState` created by the opener.
    pub fn new(cx: &mut Context<Self>, input: Entity<InputState>) -> Self {
        let svc = state::AppState::applications(cx);
        let entries = svc.get().entries;
        let mut search = FuzzySearch::new();
        search.set_items(entries);

        let pattern = input.read(cx).text().to_string();

        let input_for_sub = input.clone();
        let sub = cx.subscribe(&input, move |this: &mut Self, _state, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.pattern = input_for_sub.read(cx).text().to_string();
                this.selected = 0;
                this.refresh_results();
                cx.notify();
            }
        });

        let mut view = Self {
            search,
            input,
            pattern,
            selected: 0,
            results: Vec::new(),
            scroll: ScrollHandle::new(),
            _input_sub: sub,
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

    /// Focus the launcher's input field (the component `InputState`).
    pub fn focus_input(&self, window: &mut Window, cx: &mut App) {
        self.input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    fn refresh_results(&mut self) {
        self.search.update_pattern(&self.pattern);
        // `results()` returns `(AppEntry, nucleo_score)`; the score becomes the
        // primary ranking key inside `frecency::rank` for typed queries.
        let raw: Vec<(AppEntry, f32)> = self.search.results(MAX_RESULTS);

        // T275 Часть C: rank by frecency. Empty query -> frecency primary;
        // typed query -> nucleo relevance primary, frecency secondary.
        let data = frecency::cached();
        let now = frecency::now();
        self.results = frecency::rank(raw, &self.pattern, &data, now);

        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
        // Keep the selection in view; with a fresh pattern this scrolls back
        // to the top. Safe before first layout — the scroll request stays
        // pending until the child exists (see scroll_to_active_item).
        self.scroll.scroll_to_item(self.selected);
    }

    fn handle_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
        let key = event.keystroke.key.as_str();

        match key {
            "escape" => {
                crate::launcher::close_this(window, cx);
            }
            "enter" => {
                if let Some(entry) = self.results.get(self.selected).cloned() {
                    // T275 Часть C: record the launch before firing it.
                    frecency::record_launch(&entry.id);
                    if let Err(err) = launch(&entry.exec) {
                        tracing::error!("Failed to launch {}: {:#}", entry.name, err);
                    }
                }
                crate::launcher::close_this(window, cx);
            }
            "up" => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.scroll.scroll_to_item(self.selected);
                    window.refresh();
                }
            }
            "down" | "tab" => {
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                    self.scroll.scroll_to_item(self.selected);
                    window.refresh();
                }
            }
            // All text editing (letters, backspace, ctrl+w, home/end, paste,
            // cursor movement) is owned by the component `Input`; the launcher
            // no longer touches the buffer. The raw keydown still bubbles here
            // but we intentionally do nothing for those keys.
            _ => {}
        }
    }
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);

        let selected = self.selected;
        let is_empty = self.results.is_empty();
        let results: Vec<(usize, SharedString, AppEntry)> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, e)| (i, SharedString::from(e.name.clone()), e.clone()))
            .collect();

        div()
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
            .child(self.render_card(theme, selected, is_empty, results))
            .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
    }
}

impl LauncherView {
    fn render_card(
        &self,
        theme: &Theme,
        selected: usize,
        is_empty: bool,
        results: Vec<(usize, SharedString, AppEntry)>,
    ) -> impl IntoElement {
        div()
            .w(px(720.))
            // Bounded height, so the results child has a leftover to flex into.
            // Without it the card grows with the list and its header slides off
            // the top of the window (T265-0 raised the row cap to 200).
            .h_full()
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .rounded_lg()
            .shadow(card_shadow(theme))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_header(theme))
            .child(self.render_search(theme))
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

    fn render_search(&self, theme: &Theme) -> impl IntoElement {
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
            // Real editable field: gpui-component `Input` bound to `input`.
            // `appearance(false)` lets it blend into the launcher row (no extra
            // box); the component owns the caret, selection, IME and editing.
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .child(
                        Input::new(&self.input)
                            .appearance(false)
                            .cleanable(true)
                            .text_color(theme.text.primary)
                            .text_size(px(17.)),
                    ),
            )
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
            // A flex child sizes to its content unless it may shrink below it.
            // With the row cap lifted (T265-0) the 200-row list grew the
            // container past the window and pushed the header, search field
            // and footer off-screen. `min_h(0)` is what makes `flex_1` mean
            // "take the leftover height" instead of "take the content height".
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .pt(px(4.))
            .pb(px(8.))
            .px(px(8.))
            .children(results.into_iter().map(|(i, name, entry)| {
                let is_selected = i == selected;
                let entry_for_click = entry.clone();
                let entry_for_menu = entry.clone();
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
                    // T275 Часть D: right-click a result row to pin/unpin it.
                    .on_mouse_down(
                        MouseButton::Right,
                        {
                            let menu_id = entry_for_menu.id.clone();
                            move |event, window, cx| {
                                let anchor = Bounds::new(
                                    event.position,
                                    Size::new(px(220.), px(34.)),
                                );
                                pin_menu::open(cx, anchor, window.window_handle(), menu_id.clone());
                            }
                        },
                    )
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
                        // T275 Часть C: record the launch on click too. Use the
                        // already-cloned entry so the closure owns its data and
                        // `self` does not escape the `render` method body.
                        frecency::record_launch(&entry_for_click.id);
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
            // NOTE (T275 Часть B): the old "tune" button was removed — it had
            // no `on_click` and only pretended to be a settings control, which
            // violates the T246 "no control without a backend" rule. It returns
            // only with a real launcher settings panel.
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
            // `PathBuf`, never a `String`: `impl From<String> for ImageSource`
            // routes anything that is not a URI into `Resource::Embedded`, so
            // an absolute path was looked up among the app's bundled assets
            // and silently rendered as nothing. The dock always did it this
            // way — that is why its icons showed and the launcher's did not.
            let src: ImageSource = path_buf.into();
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
