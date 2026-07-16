//! Launcher overlay view: search input + fuzzy-matched result list.

use gpui::{App, Focusable, Render, SharedString, Window, div, prelude::*, px};

use chronos_ui::Theme;

use crate::launcher::cache::DesktopEntryCache;
use crate::launcher::entry::DesktopEntry;
use crate::launcher::launch::launch;
use crate::launcher::search::FuzzySearch;

const INPUT_HEIGHT: f32 = 40.;
const ROW_HEIGHT: f32 = 32.;
const MAX_VISIBLE_ROWS: usize = 10;

// Цвета теперь берутся из `Theme` (chronos-ui) в render(); константы удалены.
// Соответствие прежним значениям:
//   BG_COLOR     0x1e1e2e -> theme.bg.primary
//   INPUT_BG     0x313142 -> theme.bg.elevated
//   SELECTED_BG  0x454566 -> theme.interactive.hover
//   HINT_COLOR   0x6c7080 -> theme.text.muted

/// Centered overlay view showing fuzzy search results over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    pattern: String,
    selected: usize,
    results: Vec<DesktopEntry>,
    focus: gpui::FocusHandle,
}

impl LauncherView {
    /// Build a launcher view seeded with the current desktop entry cache.
    pub fn new(cx: &mut App) -> Self {
        let cache = cx.global::<DesktopEntryCache>();
        let mut search = FuzzySearch::new();
        search.set_items(cache.entries.clone());

        let mut view = Self {
            search,
            pattern: String::new(),
            selected: 0,
            results: Vec::new(),
            focus: cx.focus_handle(),
        };
        view.refresh_results();
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

    fn handle_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, _cx: &mut App) {
        let key = event.keystroke.key.as_str();

        match key {
            "Escape" => {
                window.remove_window();
            }
            "Enter" => {
                if let Some(entry) = self.results.get(self.selected).cloned() {
                    if let Err(err) = launch(&entry.exec) {
                        tracing::error!("Failed to launch {}: {:#}", entry.name, err);
                    }
                }
                window.remove_window();
            }
            "Up" => {
                if self.selected > 0 {
                    self.selected -= 1;
                    window.refresh();
                }
            }
            "Down" | "Tab" => {
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                    window.refresh();
                }
            }
            "Backspace" => {
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
        // Focus hygiene: as an XDG toplevel the compositor grants focus via
        // the normal focus policy (reinforced by the Hyprland `stay_focused`
        // windowrule), but the very first paint can race ahead of the
        // compositor's focus ack, and some relayout paths (Hyprland/Niri) can
        // drop focus mid-session. Re-assert focus every frame if we don't
        // already hold it — cheap, and makes the launcher typeable the
        // instant it appears, no click needed.
        // Historical note: this block predates the toplevel migration — it
        // was originally added as a workaround for `KeyboardInteractivity::
        // OnDemand` on a layer-shell surface (which never granted focus
        // automatically). With the layer-shell path retired
        // (see `mod.rs::window_options` for why — `Exclusive` wedged the
        // input stack on Hyprland/Niri), this re-assert stays because it
        // still earns its keep for the toplevel.
        if !self.focus.is_focused(_window) {
            self.focus.focus(_window, cx);
        }

        let theme = Theme::global(cx);

        let pattern: SharedString = self.pattern.clone().into();
        let selected = self.selected;
        let results: Vec<(usize, SharedString)> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, e)| (i, SharedString::from(e.name.clone())))
            .collect();

        div()
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
                    .children(results.into_iter().map(|(i, name)| {
                        let is_selected = i == selected;
                        div()
                            .h(px(ROW_HEIGHT))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .when(is_selected, |el| el.bg(theme.interactive.hover))
                            .child(
                                div()
                                    .when(is_selected, |el| el.child("> "))
                                    .when(!is_selected, |el| el.child("  ")),
                            )
                            .child(name)
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
