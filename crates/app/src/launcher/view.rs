//! Launcher overlay view: search input + fuzzy-matched result list.

use gpui::{App, Focusable, Render, SharedString, Window, div, prelude::*, px};

use chronos_services::{AppEntry, Service};
use chronos_ui::Theme;

use crate::launcher::launch::launch;
use crate::launcher::search::FuzzySearch;
use crate::state;

const INPUT_HEIGHT: f32 = 40.;
const ROW_HEIGHT: f32 = 32.;
const MAX_VISIBLE_ROWS: usize = 10;

/// Centered overlay view showing fuzzy search results over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    pattern: String,
    selected: usize,
    results: Vec<AppEntry>,
    focus: gpui::FocusHandle,
    /// Set `true` when user clicks on a result row (handled by click handler
    /// before activation observer fires). Observer checks this gate and skips
    /// close — the explicit click handler already calls close_this.
    pub interacted: bool,
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
            interacted: false,
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
            // gpui key names are lowercase ("escape", not "Escape") — see
            // gpui_linux platform.rs Keysym mapping.
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
            // Printable single character (ignore raw modifiers / non-text keys).
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
        // NOTE: Per-frame focus re-assert intentionally REMOVED.
        // With `stay_focused` removed from the Lua windowrule, the compositor
        // grants focus via normal focus policy. Re-asserting every frame would
        // fight the compositor and recreate the focus trap. The initial
        // `focus_input()` call in `open()` handles the "type immediately"
        // requirement; after that, focus follows compositor policy.

        let theme = Theme::global(cx);

        let pattern: SharedString = self.pattern.clone().into();
        let selected = self.selected;
        let results: Vec<(usize, SharedString, chronos_services::AppEntry)> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, e)| (i, SharedString::from(e.name.clone()), e.clone()))
            .collect();
        let view_handle = cx.entity();

        div()
            // Attach the focus handle to this element: key events dispatch
            // along the focused element's ancestor path, so focusing an
            // untracked handle sends keystrokes into the void.
            .track_focus(&self.focus)
            .size_full()
            .bg(theme.bg.primary)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
            .child(
                div()
                    .h(px(INPUT_HEIGHT))
                    .bg(theme.bg.elevated)
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .child(format!("🔍 {pattern}")),
            )
            .child(
                div()
                    .flex_1()
                    .flex_col()
                    .children(results.into_iter().map(|(i, name, entry)| {
                        let is_selected = i == selected;
                        let entry_for_click = entry.clone();
                        let vh = view_handle.clone();
                        div()
                            .id(format!("launcher-row-{i}"))
                            .h(px(ROW_HEIGHT))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(theme.interactive.hover))
                            .child(
                                div()
                                    .when(is_selected, |el| el.child("> "))
                                    .when(!is_selected, |el| el.child("  ")),
                            )
                            .child(name)
                            .on_click(move |_event, window, cx: &mut App| {
                                vh.update(cx, |view, _| view.interacted = true);
                                if let Err(err) = launch(&entry_for_click.exec) {
                                    tracing::error!("Failed to launch {}: {:#}", entry_for_click.name, err);
                                }
                                crate::launcher::close_this(window, cx);
                            })
                    }))
                    .when(self.results.is_empty(), |el| {
                        el.child(
                            div()
                                .h(px(ROW_HEIGHT))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .child(div().text_color(theme.text.muted).child("No results")),
                        )
                    }),
            )
    }
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus.clone()
    }
}
