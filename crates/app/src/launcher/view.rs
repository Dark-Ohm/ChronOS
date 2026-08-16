//! Launcher overlay view: search input + category bar + app grid.
//!
//! Redesigned per `docs/design/Chronos-OSD-Launcher.dc.html` (T261): a centered
//! 720px card on a gradient backdrop with header, search row, footer (luau
//! badge + reload dot). T265-B replaced the flat result list with an app grid
//! (icon + label) and an XDG category bar (hover-open + click-lock).
//!
//! T275 (волна 1): the search field is a real `gpui-component` `Input` bound
//! to an `InputState` — caret, cursor movement, selection, IME, paste and
//! ctrl+w come from the component (no hand-rolled caret). The launcher owns
//! navigation keys; text editing is delegated to the `Input`. Results are
//! ranked by frecency (T275 Часть C), and launching records frecency. Right-
//! click on a cell opens a Pin/Unpin menu (T275 Часть D).

use gpui::{
    self, App, Bounds, Entity, FocusHandle, ImageSource, MouseButton, Render, ScrollHandle,
    SharedString, Size, Subscription, Window, div, img, linear_color_stop, linear_gradient,
    prelude::*, px, svg,
};

use chronos_services::applications::frecency;
use chronos_services::{AppEntry, Service};
use chronos_ui::{Theme, WindowRootExt};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::icon_resolution::resolve_icon;
use crate::launcher::grid::{
    build_categories, filter_by_category, move_2d, CELL_HEIGHT, CELL_WIDTH, GRID_COLUMNS, GRID_GAP,
    PAGE_ROWS, Move2D,
};
use crate::launcher::launch::launch;
use crate::launcher::pin_menu;
use crate::launcher::search::FuzzySearch;
use crate::state;

const INPUT_HEIGHT: f32 = 44.;
const CATEGORY_BAR_HEIGHT: f32 = 40.;
/// Soft cap on the ranked result set — NOT on visibility. The grid scrolls,
/// so this only bounds per-frame render cost. Was a hard 10-row cap
/// (`MAX_VISIBLE_ROWS`) which made the scroll container dead (T265-0).
const MAX_RESULTS: usize = 200;
/// App icon size inside a grid cell.
const GRID_ICON: f32 = 36.;

/// Which region of the launcher owns the keyboard (T265-B Tab cycling).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FocusSection {
    Search,
    Categories,
    Grid,
}

/// Centered overlay view showing a searchable app grid over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    /// Real editable text buffer (replaces the old `String` pattern).
    input: Entity<InputState>,
    /// Mirror of the input text, updated on `InputEvent::Change`.
    pattern: String,
    /// Flat index of the selected cell in `visible` (row-major).
    selected: usize,
    /// Ranked + search-filtered entries, NOT category-filtered.
    results: Vec<AppEntry>,
    /// Distinct categories present in `results`, sorted (count desc, name asc).
    categories: Vec<(String, usize)>,
    /// `results` filtered by the effective category — what the grid shows.
    visible: Vec<AppEntry>,
    scroll: ScrollHandle,
    /// Subscription to `InputState` change events (drives re-search).
    _input_sub: Subscription,
    /// Category locked via click/Enter (`None` = "All").
    selected_category: Option<String>,
    /// Category hovered with the mouse — wins over `selected_category` while set.
    hover_category: Option<String>,
    /// Compact mode: the grid is collapsed, only search + category bar show.
    compact: bool,
    /// Keyboard focus region, cycled by Tab.
    focus_section: FocusSection,
    /// Keyboard position in the category bar (0 = "All").
    category_index: usize,
    /// Focus target for Categories/Grid sections (the Input is focused in Search).
    focus_handle: FocusHandle,
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
            categories: Vec::new(),
            visible: Vec::new(),
            scroll: ScrollHandle::new(),
            _input_sub: sub,
            selected_category: None,
            hover_category: None,
            compact: false,
            focus_section: FocusSection::Search,
            category_index: 0,
            focus_handle: cx.focus_handle(),
        };
        view.refresh_results();
        // Repaint after the synchronous seed so the first frame shows the
        // populated grid, not the empty `results` the view was built with
        // (T275: empty query rendered "No matches" without this notify).
        cx.notify();

        // Subscribe to desktop entry changes — live updates without restart.
        let signal = state::AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this, state, cx| {
            this.search.set_items(state.entries);
            this.refresh_results();
            cx.notify();
        });

        view
    }

    /// Focus the launcher's input field (the component `InputState`) and put
    /// the keyboard back on the search section.
    pub fn focus_input(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_section = FocusSection::Search;
        self.sync_focus(window, cx);
    }

    /// Route real focus to whichever section currently owns the keyboard:
    /// the Input for Search, the view's own handle for Categories/Grid.
    fn sync_focus(&mut self, window: &mut Window, cx: &mut App) {
        if self.focus_section == FocusSection::Search {
            self.input.update(cx, |input, cx| input.focus(window, cx));
        } else {
            self.focus_handle.focus(window, cx);
        }
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

        self.categories = build_categories(&self.results);
        // Category bar = "All" (index 0) + `categories`. Clamp the keyboard
        // cursor if the bar shrank (e.g. typing removed a category).
        if self.category_index > self.categories.len() {
            self.category_index = self.categories.len();
        }
        self.apply_category_filter();
    }

    /// The category the grid currently obeys: hover wins over the locked one.
    fn effective_category(&self) -> Option<String> {
        self.hover_category.clone().or_else(|| self.selected_category.clone())
    }

    /// Recompute `visible` (ranked results filtered by effective category) and
    /// clamp the selection into the new grid.
    fn apply_category_filter(&mut self) {
        let cat = self.effective_category();
        self.visible = filter_by_category(&self.results, cat.as_deref());
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
        self.scroll_to_selected();
    }

    /// Scroll the selected cell into view. The grid's scroll container children
    /// are ROWS (one per `GRID_COLUMNS` cells), so the scroll index is the row,
    /// not the flat cell index. Safe before first layout — the request stays
    /// pending until the child exists.
    fn scroll_to_selected(&self) {
        self.scroll.scroll_to_item(self.selected / GRID_COLUMNS);
    }

    /// Category at bar position `index` (0 = "All" = `None`).
    fn category_at(&self, index: usize) -> Option<String> {
        if index == 0 {
            None
        } else {
            self.categories.get(index - 1).map(|(c, _)| c.clone())
        }
    }

    /// Lock the category at bar position `pos` (click/Enter) and move keyboard
    /// focus to the grid. Caller repaints.
    fn select_category_at(&mut self, pos: usize) {
        self.selected_category = self.category_at(pos);
        self.category_index = pos;
        self.hover_category = None;
        self.selected = 0;
        self.focus_section = FocusSection::Grid;
        self.apply_category_filter();
    }

    /// Hover-open: show `cat` while the pointer is over its chip; `None` on
    /// leave. Caller repaints.
    fn set_hover_category(&mut self, cat: Option<String>) {
        if self.hover_category != cat {
            self.hover_category = cat;
            self.apply_category_filter();
        }
    }

    /// Keyboard cursor movement in the category bar. Caller repaints.
    fn move_category(&mut self, left: bool) {
        let max = self.categories.len(); // positions 0..=max (0 = "All")
        if left {
            self.category_index = self.category_index.saturating_sub(1);
        } else if self.category_index < max {
            self.category_index += 1;
        }
    }

    fn cycle_focus_section(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_section = match self.focus_section {
            FocusSection::Search => FocusSection::Categories,
            // A collapsed grid has no cells — skip it in the Tab cycle.
            FocusSection::Categories => {
                if self.compact {
                    FocusSection::Search
                } else {
                    FocusSection::Grid
                }
            }
            FocusSection::Grid => FocusSection::Search,
        };
        self.sync_focus(window, cx);
        window.refresh();
    }

    fn navigate_grid(&mut self, mv: Move2D, window: &mut Window) {
        let next = move_2d(self.selected, GRID_COLUMNS, self.visible.len(), mv, PAGE_ROWS);
        if next != self.selected {
            self.selected = next;
            self.scroll_to_selected();
            window.refresh();
        }
    }

    fn launch_selected(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(entry) = self.visible.get(self.selected).cloned() {
            // T275 Часть C: record the launch before firing it.
            frecency::record_launch(&entry.id);
            if let Err(err) = launch(&entry.exec) {
                tracing::error!("Failed to launch {}: {:#}", entry.name, err);
            }
        }
        crate::launcher::close_this(window, cx);
    }

    fn handle_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
        let key = event.keystroke.key.as_str();

        match key {
            "escape" => {
                crate::launcher::close_this(window, cx);
            }
            "tab" => self.cycle_focus_section(window, cx),
            "enter" => {
                if self.focus_section == FocusSection::Categories {
                    let pos = self.category_index;
                    self.select_category_at(pos);
                    self.sync_focus(window, cx);
                    window.refresh();
                } else {
                    self.launch_selected(window, cx);
                }
            }
            "left" | "right" => {
                if self.focus_section == FocusSection::Categories {
                    self.move_category(key == "left");
                    window.refresh();
                } else if self.focus_section == FocusSection::Grid {
                    self.navigate_grid(if key == "left" { Move2D::Left } else { Move2D::Right }, window);
                }
                // In Search, left/right are the Input's cursor keys — ignored here.
            }
            "up" | "down" => {
                if self.focus_section == FocusSection::Categories {
                    self.focus_section = FocusSection::Grid;
                    self.sync_focus(window, cx);
                    window.refresh();
                } else {
                    self.navigate_grid(if key == "up" { Move2D::Up } else { Move2D::Down }, window);
                }
            }
            "home" | "end" => {
                if self.focus_section == FocusSection::Categories {
                    self.category_index = if key == "home" { 0 } else { self.categories.len() };
                    window.refresh();
                } else if self.focus_section == FocusSection::Grid {
                    self.navigate_grid(if key == "home" { Move2D::Home } else { Move2D::End }, window);
                }
                // In Search, home/end are the Input's cursor keys — ignored here.
            }
            "pageup" | "pagedown" => {
                if self.focus_section == FocusSection::Grid {
                    self.navigate_grid(
                        if key == "pageup" { Move2D::PageUp } else { Move2D::PageDown },
                        window,
                    );
                }
            }
            // All text editing (letters, backspace, ctrl+w, paste, cursor
            // movement) is owned by the component `Input` in the Search
            // section; the raw keydown still bubbles here but we do nothing.
            _ => {}
        }
    }
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let entity = cx.entity();

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
            .child(self.render_card(theme, entity))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
    }
}

impl LauncherView {
    fn render_card(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        div()
            .w(px(720.))
            // Bounded height, so the grid child has a leftover to flex into.
            // Without it the card grows with the grid and its header slides off
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
            .child(self.render_category_bar(theme, entity))
            .when(!self.compact, |el| el.child(self.render_grid(theme)))
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
        // T265-A: faint inline-completion tail (first result) at the end of
        // the field. Purely a hint — Enter still launches the selected cell and
        // does not "complete then wait for a second Enter".
        let mut input = Input::new(&self.input)
            .appearance(false)
            .cleanable(true)
            .text_color(theme.text.primary)
            .text_size(px(17.));
        if let Some(ghost) = completion_hint(&self.pattern, &self.visible) {
            input = input.suffix(
                div()
                    .text_color(theme.text.faint)
                    .text_size(px(17.))
                    .whitespace_nowrap()
                    .child(ghost),
            );
        }

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
                    .child(input),
            )
    }

    fn render_category_bar(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let total = self.results.len();
        let active = self.effective_category();
        let kb = self.focus_section == FocusSection::Categories;

        // Bar items: "All" (None) followed by the distinct categories.
        let mut bar_items: Vec<Option<String>> = Vec::with_capacity(self.categories.len() + 1);
        bar_items.push(None);
        bar_items.extend(self.categories.iter().map(|(c, _)| Some(c.clone())));

        div()
            .id("launcher-categories")
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(10.))
            .h(px(CATEGORY_BAR_HEIGHT))
            .border_b_1()
            .border_color(theme.border.subtle.opacity(0.5))
            // Horizontally-scrollable chips.
            .child(
                div()
                    .id("launcher-categories-scroll")
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .gap(px(6.))
                    .overflow_x_scroll()
                    .children(bar_items.iter().enumerate().map(|(pos, cat)| {
                        let count = if pos == 0 {
                            total
                        } else {
                            self.categories.get(pos - 1).map(|(_, n)| *n).unwrap_or(0)
                        };
                        let label = cat.clone().unwrap_or_else(|| "All".to_string());
                        let is_on = active.as_deref() == cat.as_deref();
                        let is_kb = kb && self.category_index == pos;
                        let entity = entity.clone();
                        let cat_owned = cat.clone();

                        div()
                            .id(format!("launcher-cat-{pos}"))
                            .h(px(28.))
                            .px(px(11.))
                            .rounded(px(7.))
                            .border_1()
                            .border_color(if is_kb { theme.accent.primary } else { theme.border.subtle })
                            .bg(if is_on || is_kb { theme.bg.selection } else { theme.bg.primary })
                            .text_color(if is_on || is_kb { theme.text.primary } else { theme.text.muted })
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_size(theme.font_sizes.xs)
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .font_family(theme.font_mono)
                                    .text_size(px(10.))
                                    .text_color(theme.text.faint)
                                    .child(count.to_string()),
                            )
                            .on_click({
                                let entity = entity.clone();
                                move |_, _window, cx: &mut App| {
                                    entity.update(cx, |this, cx| {
                                        this.select_category_at(pos);
                                        cx.notify();
                                    });
                                }
                            })
                            .on_hover({
                                let entity = entity.clone();
                                move |hovered, _window, cx: &mut App| {
                                    let cat = if *hovered { cat_owned.clone() } else { None };
                                    entity.update(cx, |this, cx| {
                                        this.set_hover_category(cat);
                                        cx.notify();
                                    });
                                }
                            })
                    })),
            )
            // Compact-mode chevron (▾ collapse / ▸ expand).
            .child(
                div()
                    .id("launcher-chevron")
                    .size(px(28.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(theme.border.subtle)
                    .bg(theme.bg.primary)
                    .text_color(theme.text.muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(if self.compact { "▸" } else { "▾" })
                    .on_click({
                        let entity = entity.clone();
                        move |_, _window, cx: &mut App| {
                            entity.update(cx, |this, cx| {
                                this.compact = !this.compact;
                                if this.compact && this.focus_section == FocusSection::Grid {
                                    this.focus_section = FocusSection::Search;
                                }
                                cx.notify();
                            });
                        }
                    }),
            )
    }

    fn render_grid(&self, theme: &Theme) -> impl IntoElement {
        let selected = self.selected;
        let rows: Vec<&[AppEntry]> = self.visible.chunks(GRID_COLUMNS).collect();

        div()
            .id("launcher-grid")
            .flex_1()
            // A flex child sizes to its content unless it may shrink below it.
            // `min_h(0)` makes `flex_1` mean "take the leftover height" instead
            // of "take the content height" (T265-0).
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .pt(px(4.))
            .pb(px(8.))
            .px(px(8.))
            .flex()
            .flex_col()
            .gap(px(GRID_GAP))
            .children(rows.into_iter().enumerate().map(|(ri, row)| {
                div()
                    .flex()
                    .gap(px(GRID_GAP))
                    .children(row.iter().enumerate().map(|(ci, entry)| {
                        let flat = ri * GRID_COLUMNS + ci;
                        self.render_cell(theme, entry, flat, flat == selected)
                    }))
            }))
            .when(self.visible.is_empty(), |el| {
                el.child(
                    div()
                        .py(px(34.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text.faint)
                        .text_sm()
                        .child("No matches"),
                )
            })
    }

    fn render_cell(&self, theme: &Theme, entry: &AppEntry, flat: usize, is_selected: bool) -> impl IntoElement {
        let entry_for_click = entry.clone();
        let entry_for_menu = entry.clone();
        let icon_el = resolve_app_icon(entry, theme, GRID_ICON);
        let name = SharedString::from(entry.name.clone());

        div()
            .id(format!("launcher-cell-{flat}"))
            .w(px(CELL_WIDTH))
            .h(px(CELL_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(8.))
            .cursor_pointer()
            .when(is_selected, |el| el.bg(theme.bg.selection))
            .when(!is_selected, |el| el.hover(|s| s.bg(theme.interactive.hover)))
            // T275 Часть D: right-click a cell to pin/unpin it.
            .on_mouse_down(
                MouseButton::Right,
                {
                    let menu_id = entry_for_menu.id.clone();
                    move |event, window, cx: &mut App| {
                        // anchor_rect (AnchoredPopup) is window-local;
                        // pin_menu::open() derives the click-catcher's
                        // output-local hole itself — see `catcher_anchor_for`
                        // in pin_menu.rs (T275).
                        let anchor = Bounds::new(event.position, Size::new(px(1.), px(1.)));
                        pin_menu::open(cx, anchor, event.position, window.window_handle(), menu_id.clone());
                    }
                },
            )
            // App icon (SVG via system theme, or letter fallback).
            .child(
                div()
                    .size(px(GRID_ICON))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(if is_selected { theme.accent.primary } else { theme.text.muted })
                    .child(icon_el),
            )
            // Label (centered by the cell's `items_center`, truncated).
            .child(
                div()
                    .max_w(px(CELL_WIDTH - 8.))
                    .text_xs()
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(is_selected, |el| el.text_color(theme.accent.primary))
                    .child(name),
            )
            .on_click(move |_event, window, cx: &mut App| {
                // T275 Часть C: record the launch on click too. Use the
                // already-cloned entry so the closure owns its data and
                // `self` does not escape the `render` method body.
                frecency::record_launch(&entry_for_click.id);
                if let Err(err) = launch(&entry_for_click.exec) {
                    tracing::error!("Failed to launch {}: {:#}", entry_for_click.name, err);
                }
                crate::launcher::close_this(window, cx);
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
/// icon theme first, fall back to a letter glyph. `size` is the square side in
/// pixels (list rows used 18, grid cells use 36).
fn resolve_app_icon(entry: &AppEntry, theme: &Theme, size: f32) -> gpui::AnyElement {
    if let Some(name) = entry.icon.as_deref() {
        if let Some(path_buf) = resolve_icon(name) {
            // `PathBuf`, never a `String`: `impl From<String> for ImageSource`
            // routes anything that is not a URI into `Resource::Embedded`, so
            // an absolute path was looked up among the app's bundled assets
            // and silently rendered as nothing. The dock always did it this
            // way — that is why its icons showed and the launcher's did not.
            let src: ImageSource = path_buf.into();
            return img(src).size(px(size)).into_any_element();
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
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(size * 0.25))
        .bg(theme.bg.elevated)
        .child(div().text_size(px(size * 0.45)).text_color(theme.text.primary).child(letter))
        .into_any_element()
}

/// Inline-completion hint: the first visible result's name, shown as a faint
/// tail in the search field when there is a typed pattern and at least one
/// match. Purely visual — Enter still launches the selected cell (T265-A).
fn completion_hint(pattern: &str, results: &[AppEntry]) -> Option<String> {
    if pattern.trim().is_empty() || results.is_empty() {
        return None;
    }
    Some(results[0].name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_hint_shows_first_result_when_typed() {
        let results = vec![
            AppEntry::fixture("firefox", "Firefox"),
            AppEntry::fixture("files", "Files"),
        ];
        assert_eq!(completion_hint("fir", &results).as_deref(), Some("Firefox"));
    }

    #[test]
    fn completion_hint_none_on_empty_pattern() {
        let results = vec![AppEntry::fixture("firefox", "Firefox")];
        assert_eq!(completion_hint("", &results), None);
        assert_eq!(completion_hint("   ", &results), None);
    }

    #[test]
    fn completion_hint_none_when_no_results() {
        assert_eq!(completion_hint("fir", &[]), None);
    }
}
