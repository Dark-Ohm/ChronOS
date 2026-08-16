//! Launcher overlay view: search input + category bar + sections + app grid.
//!
//! Redesigned per `docs/design/Chronos-OSD-Launcher.dc.html` (T261): a centered
//! 720px card on a gradient backdrop with header, search row, footer (luau
//! badge + reload dot). T265-B replaced the flat result list with an app grid
//! (icon + label) and an XDG category bar (hover-open + click-lock). T265-C
//! adds three curated sections above the grid — Favorites (manual order, DnD),
//! Recents (top-N frecency), Folders (DnD-create, expand, component-Input
//! rename) — plus a "new" badge on recently-installed `.desktop` entries.
//!
//! T275 (волна 1): the search field is a real `gpui-component` `Input` bound
//! to an `InputState` — caret, cursor movement, selection, IME, paste and
//! ctrl+w come from the component (no hand-rolled caret). The launcher owns
//! navigation keys; text editing is delegated to the `Input`. Results are
//! ranked by frecency (T275 Часть C), and launching records frecency. Right-
//! click on a cell opens a Pin/Unpin menu (T275 Часть D).

use std::collections::{HashMap, HashSet};

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
use crate::launcher::favorites::{
    desktop_dirs, desktop_mtime, folder_add_app, index_by_id, is_new, move_item, next_folder_id,
    resolve_folder_apps, resolve_favorites, top_recents, NEW_DAYS,
};
use crate::launcher::grid::{
    build_categories, filter_by_category, move_2d, CELL_HEIGHT, CELL_WIDTH, GRID_COLUMNS, GRID_GAP,
    PAGE_ROWS, Move2D,
};
use crate::launcher::app_menu;
use crate::launcher::launch::launch;
use crate::launcher::launcher_config::{self, Folder, LauncherConfig};
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
/// Section (favorites / recents / folders) cell geometry — denser than grid
/// cells, wrapping at `SECTION_COLUMNS` per row.
const SECTION_CELL_WIDTH: f32 = 80.;
const SECTION_CELL_HEIGHT: f32 = 76.;
const SECTION_COLUMNS: usize = 8;
const SECTION_ICON: f32 = 28.;

/// Which region of the launcher owns the keyboard (T265-B Tab cycling).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FocusSection {
    Search,
    Categories,
    Grid,
}

/// Internal DnD payload (GPUI in-window drag, NOT a file drag — T270).
///
/// The drop target decides the action (reorder vs insert vs create-folder),
/// so the payload carries only the dragged app's identity.
#[derive(Clone, Debug)]
struct LauncherDrag {
    app_id: String,
    app_name: String,
}

impl Render for LauncherDrag {
    /// Ghost pill shown under the cursor while dragging.
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        div()
            .pl(px(14.))
            .pt(px(6.))
            .child(
                div()
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .bg(theme.bg.elevated)
                    .border_1()
                    .border_color(theme.border.subtle)
                    .shadow_md()
                    .text_color(theme.text.primary)
                    .text_xs()
                    .child(self.app_name.clone()),
            )
    }
}

/// Centered overlay view showing a searchable app grid over desktop entries.
pub struct LauncherView {
    search: FuzzySearch,
    /// Real editable text buffer (replaces the old `String` pattern).
    input: Entity<InputState>,
    /// Second component input for folder rename (T265-C).
    rename_input: Entity<InputState>,
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
    /// Every listed entry as delivered by the service, BEFORE the user-hidden
    /// filter (T265-D) — the re-filter source for `all`.
    raw_entries: Vec<AppEntry>,
    /// Listed entries minus user-hidden ids — what sections/search/grid see.
    all: Vec<AppEntry>,
    /// id → entry for resolving favorites/folders.
    by_id: HashMap<String, AppEntry>,
    /// Persisted favorites/recents/folders config (view's working copy).
    config: LauncherConfig,
    /// Resolved favorites in display order.
    favorites: Vec<AppEntry>,
    /// Top-N recents (frecency).
    recents: Vec<AppEntry>,
    /// Folders (persisted model).
    folders: Vec<Folder>,
    /// The single expanded folder id, if any.
    expanded: Option<String>,
    /// The folder id currently being renamed, if any.
    rename: Option<String>,
    /// ids of entries whose `.desktop` is "new" (badge).
    new_ids: HashSet<String>,
    /// Number of section children rendered before the grid (scroll indexing).
    grid_row_offset: usize,
    scroll: ScrollHandle,
    /// Subscription to `InputState` change events (drives re-search).
    _input_sub: Subscription,
    /// Subscription to rename-input events (Enter/Blur commit).
    _rename_sub: Subscription,
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
    /// Build a launcher view seeded with the current desktop entries, the live
    /// search `InputState`, the rename `InputState`, and the window (for
    /// `subscribe_in` on the rename input).
    pub fn new(
        cx: &mut gpui::Context<Self>,
        input: Entity<InputState>,
        rename_input: Entity<InputState>,
        window: &Window,
    ) -> Self {
        let svc = state::AppState::applications(cx);
        let entries = svc.get().entries;

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

        let rename_sub = cx.subscribe_in(&rename_input, window, {
            move |this: &mut Self, _state, event: &InputEvent, window, cx| {
                match event {
                    // Clicking away commits the rename (blur); guard so a commit
                    // that already cleared `rename` does not double-fire.
                    InputEvent::Blur => {
                        if this.rename.is_some() {
                            this.commit_rename(window, cx);
                        }
                    }
                    _ => {}
                }
            }
        });

        let mut view = Self {
            search: FuzzySearch::new(),
            input,
            rename_input,
            pattern,
            selected: 0,
            results: Vec::new(),
            categories: Vec::new(),
            visible: Vec::new(),
            raw_entries: Vec::new(),
            all: Vec::new(),
            by_id: HashMap::new(),
            config: launcher_config::get(),
            favorites: Vec::new(),
            recents: Vec::new(),
            folders: Vec::new(),
            expanded: None,
            rename: None,
            new_ids: HashSet::new(),
            grid_row_offset: 0,
            scroll: ScrollHandle::new(),
            _input_sub: sub,
            _rename_sub: rename_sub,
            selected_category: None,
            hover_category: None,
            compact: false,
            focus_section: FocusSection::Search,
            category_index: 0,
            focus_handle: cx.focus_handle(),
        };
        view.set_entries(entries);
        // Repaint after the synchronous seed so the first frame shows the
        // populated grid, not the empty `results` the view was built with
        // (T275: empty query rendered "No matches" without this notify).
        cx.notify();

        // Subscribe to desktop entry changes — live updates without restart.
        let signal = state::AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this, state, cx| {
            this.set_entries(state.entries);
            cx.notify();
        });

        // T265-D: config mutations (favorite / hide / folder ops) re-filter the
        // hidden set and refresh sections immediately, without a restart.
        let config_signal = launcher_config::subscribe();
        state::watch(cx, config_signal, |this, _, cx| {
            this.apply_hidden_filter();
            this.recompute_sections();
            this.refresh_results();
            cx.notify();
        });

        view
    }

    /// Replace the full listed entry set (initial seed + inotify rescan).
    fn set_entries(&mut self, entries: Vec<AppEntry>) {
        self.raw_entries = entries;
        self.apply_hidden_filter();
        self.recompute_new_ids();
        self.recompute_sections();
        self.refresh_results();
    }

    /// Re-filter the listed set by user-hidden ids (T265-D) and rebuild the id
    /// map + search index from the visible remainder.
    fn apply_hidden_filter(&mut self) {
        let hidden: HashSet<String> = launcher_config::get().hidden.iter().cloned().collect();
        self.all = self
            .raw_entries
            .iter()
            .filter(|e| !hidden.contains(&e.id))
            .cloned()
            .collect();
        self.by_id = index_by_id(&self.all);
        self.search.set_items(self.all.clone());
    }

    /// Which listed entries carry the "new" badge (`.desktop` mtime < NEW_DAYS).
    fn recompute_new_ids(&mut self) {
        let now = frecency::now();
        let dirs = desktop_dirs();
        self.new_ids = self
            .all
            .iter()
            .filter(|e| is_new(desktop_mtime(&e.id, &dirs), now, NEW_DAYS))
            .map(|e| e.id.clone())
            .collect();
    }

    /// Recompute the three sections + scroll offset from the current config and
    /// entry set. Called on entry/config changes, not on every keystroke.
    fn recompute_sections(&mut self) {
        self.favorites = resolve_favorites(
            &self.config.favorites.order,
            &self.by_id,
            self.config.favorites.sort_alpha,
        );
        let data = frecency::cached();
        let now = frecency::now();
        self.recents = top_recents(&self.all, &data, now, self.config.recents.limit);
        self.folders = self.config.folders.clone();
        // Drop stale expansion/rename references if the folder vanished.
        if let Some(id) = &self.expanded {
            if !self.folders.iter().any(|f| &f.id == id) {
                self.expanded = None;
            }
        }
        if let Some(id) = &self.rename {
            if !self.folders.iter().any(|f| &f.id == id) {
                self.rename = None;
            }
        }
        // Number of children rendered before the grid in the scroll container:
        // one block per non-empty section (must match `render_content`'s `.when`).
        self.grid_row_offset = usize::from(!self.favorites.is_empty())
            + usize::from(!self.recents.is_empty())
            + usize::from(!self.folders.is_empty());
    }

    /// Persist the view's config into the debounced store, then refresh sections.
    fn persist_config(&mut self) {
        launcher_config::update(|c| *c = self.config.clone());
        self.recompute_sections();
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

    /// Scroll the selected cell into view. The scroll container's children are
    /// the section blocks followed by ROWS (one per `GRID_COLUMNS` cells), so
    /// the scroll index is `grid_row_offset` + the flat cell's row. Safe before
    /// first layout — the request stays pending until the child exists.
    fn scroll_to_selected(&self) {
        self.scroll
            .scroll_to_item(self.grid_row_offset + self.selected / GRID_COLUMNS);
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

    /// Launch an entry from a section cell / folder (mouse path).
    fn launch_entry(&self, entry: &AppEntry) {
        frecency::record_launch(&entry.id);
        if let Err(err) = launch(&entry.exec) {
            tracing::error!("Failed to launch {}: {:#}", entry.name, err);
        }
    }

    // --- DnD drop handlers (mutate `config`, then persist + recompute) ---

    /// Drop onto a favorite cell: reorder if the drag started in Favorites,
    /// otherwise insert the dragged app at that position.
    fn handle_drop_on_favorite_cell(&mut self, drag: &LauncherDrag, target_index: usize) {
        let order = &self.config.favorites.order;
        let from = order.iter().position(|id| id == &drag.app_id);
        match from {
            Some(from) => {
                let new_order = move_item(order, from, target_index);
                self.config.favorites.order = new_order;
            }
            None => {
                let at = target_index.min(self.config.favorites.order.len());
                self.config.favorites.order.insert(at, drag.app_id.clone());
            }
        }
        self.persist_config();
    }

    /// Drop onto the Favorites section's empty area: append.
    fn handle_drop_on_favorites_append(&mut self, drag: &LauncherDrag) {
        if !self.config.favorites.order.iter().any(|id| id == &drag.app_id) {
            self.config.favorites.order.push(drag.app_id.clone());
            self.persist_config();
        }
    }

    /// Drop an app icon onto another app icon: create a folder with both.
    fn handle_drop_create_folder(&mut self, drag: &LauncherDrag, target_id: &str) {
        if drag.app_id == target_id {
            return;
        }
        let id = next_folder_id(&self.folders);
        let name = format!("Folder {}", self.folders.len() + 1);
        self.config.folders.push(Folder {
            id,
            name,
            apps: vec![target_id.to_string(), drag.app_id.clone()],
        });
        self.persist_config();
    }

    /// Drop an app icon onto a folder tile: add to that folder.
    fn handle_drop_on_folder(&mut self, drag: &LauncherDrag, folder_id: &str) {
        let mut added = false;
        if let Some(folder) = self.config.folders.iter_mut().find(|f| f.id == folder_id) {
            added = folder_add_app(folder, &drag.app_id);
        }
        if added {
            self.persist_config();
        }
    }

    // --- Folder interactions ---

    fn toggle_folder(&mut self, folder_id: &str) {
        if self.expanded.as_deref() == Some(folder_id) {
            self.expanded = None;
        } else {
            self.expanded = Some(folder_id.to_string());
        }
    }

    fn start_rename(&mut self, folder_id: String, window: &mut Window, cx: &mut App) {
        let name = self
            .config
            .folders
            .iter()
            .find(|f| f.id == folder_id)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        self.rename = Some(folder_id);
        self.rename_input.update(cx, |input, cx| input.set_value(name, window, cx));
        self.rename_input.update(cx, |input, cx| input.focus(window, cx));
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut App) {
        let Some(folder_id) = self.rename.take() else {
            return;
        };
        let text = self.rename_input.read(cx).text().to_string();
        let text = text.trim().to_string();
        if !text.is_empty() {
            if let Some(folder) = self.config.folders.iter_mut().find(|f| f.id == folder_id) {
                folder.name = text;
            }
            self.persist_config();
        }
        self.focus_input(window, cx);
    }

    fn cancel_rename(&mut self, window: &mut Window, cx: &mut App) {
        self.rename = None;
        self.focus_input(window, cx);
    }

    fn handle_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
        let key = event.keystroke.key.as_str();

        // Rename mode swallows launcher keys: Enter commits, Esc cancels, all
        // else is left to the focused rename Input.
        if self.rename.is_some() {
            match key {
                "escape" => self.cancel_rename(window, cx),
                "enter" => self.commit_rename(window, cx),
                _ => {}
            }
            return;
        }

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
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
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
            // Bounded height, so the content child has a leftover to flex into.
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
            .child(self.render_category_bar(theme, entity.clone()))
            .when(!self.compact, |el| el.child(self.render_content(theme, entity)))
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

    /// The single scroll region: section blocks (favorites / recents / folders)
    /// then the "All apps" grid rows. `grid_row_offset` (the number of section
    /// blocks) keeps keyboard scroll-to-selected indexing correct.
    fn render_content(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let selected = self.selected;
        let rows: Vec<&[AppEntry]> = self.visible.chunks(GRID_COLUMNS).collect();

        div()
            .id("launcher-content")
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
            .when(!self.favorites.is_empty(), |el| {
                el.child(self.render_favorites_block(theme, entity.clone()))
            })
            .when(!self.recents.is_empty(), |el| {
                el.child(self.render_recents_block(theme, entity.clone()))
            })
            .when(!self.folders.is_empty(), |el| {
                el.child(self.render_folders_block(theme, entity.clone()))
            })
            .children(rows.into_iter().enumerate().map(|(ri, row)| {
                div()
                    .flex()
                    .gap(px(GRID_GAP))
                    .children(row.iter().enumerate().map(|(ci, entry)| {
                        let flat = ri * GRID_COLUMNS + ci;
                        self.render_cell(theme, entry, flat, flat == selected, entity.clone())
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

    // --- Sections ---

    fn render_section_header(&self, theme: &Theme, title: &'static str, count: usize) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(2.))
            .child(
                div()
                    .font_family(theme.font_mono)
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text.muted)
                    .child(title),
            )
            .child(
                div()
                    .font_family(theme.font_mono)
                    .text_size(px(10.))
                    .text_color(theme.text.faint)
                    .child(count.to_string()),
            )
    }

    fn render_favorites_block(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let hide_labels = self.config.favorites.hide_labels;
        let block = div()
            .id("launcher-favorites")
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(self.render_section_header(theme, "Favorites", self.favorites.len()))
            .children(self.favorites.chunks(SECTION_COLUMNS).map(|row| {
                div().flex().gap(px(GRID_GAP)).children(
                    row.iter()
                        .enumerate()
                        .map(|(ci, entry)| {
                            self.render_section_cell(theme, entity.clone(), entry, hide_labels, true, ci)
                        }),
                )
            }));

        // Drop on the block's empty area appends to favorites.
        let entity_for_drop = entity.clone();
        block.on_drop::<LauncherDrag>(move |drag, _window, cx: &mut App| {
            let drag = drag.clone();
            entity_for_drop.update(cx, |this, cx| {
                this.handle_drop_on_favorites_append(&drag);
                cx.notify();
            });
        })
    }

    fn render_recents_block(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        div()
            .id("launcher-recents")
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(self.render_section_header(theme, "Recents", self.recents.len()))
            .children(self.recents.chunks(SECTION_COLUMNS).map(|row| {
                div().flex().gap(px(GRID_GAP)).children(row.iter().map(|entry| {
                    self.render_section_cell(theme, entity.clone(), entry, false, false, 0)
                }))
            }))
    }

    fn render_folders_block(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let expanded_apps: Vec<AppEntry> = self
            .expanded
            .as_ref()
            .map(|id| {
                self.folders
                    .iter()
                    .find(|f| &f.id == id)
                    .map(|f| resolve_folder_apps(f, &self.by_id))
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        div()
            .id("launcher-folders")
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(self.render_section_header(theme, "Folders", self.folders.len()))
            .children(self.folders.chunks(SECTION_COLUMNS).map(|row| {
                div()
                    .flex()
                    .gap(px(GRID_GAP))
                    .children(row.iter().map(|folder| {
                        self.render_folder_tile(theme, entity.clone(), folder)
                    }))
            }))
            .when(!expanded_apps.is_empty(), |el| {
                el.child(div().flex().gap(px(GRID_GAP)).children(expanded_apps.iter().map(|entry| {
                    self.render_section_cell(theme, entity.clone(), entry, false, false, 0)
                })))
            })
    }

    /// A favorite / recents / folder-app cell. `is_favorite` enables reorder-on-drop
    /// (target = `index`); otherwise a drop creates a folder with the target app.
    fn render_section_cell(
        &self,
        theme: &Theme,
        entity: Entity<Self>,
        entry: &AppEntry,
        hide_label: bool,
        is_favorite: bool,
        index: usize,
    ) -> impl IntoElement {
        let icon_el = resolve_app_icon(entry, theme, SECTION_ICON);
        let name = SharedString::from(entry.name.clone());
        let launch_entry = entry.clone();
        let pin_entry = entry.clone();
        let drag_payload = LauncherDrag {
            app_id: entry.id.clone(),
            app_name: entry.name.clone(),
        };
        let drop_target = entry.id.clone();
        let entity_for_drop = entity.clone();
        let entity_for_launch = entity.clone();

        let mut cell = div()
            .id(format!("launcher-sec-{}", entry.id))
            .w(px(SECTION_CELL_WIDTH))
            .h(px(SECTION_CELL_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(8.))
            .cursor_pointer()
            .hover(|s| s.bg(theme.interactive.hover))                .on_mouse_down(
                MouseButton::Right,
                move |event, window, cx: &mut App| {
                    let anchor = Bounds::new(event.position, Size::new(px(1.), px(1.)));
                    app_menu::open(cx, anchor, event.position, window.window_handle(), pin_entry.clone());
                },
            )
            .child(
                div()
                    .size(px(SECTION_ICON))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text.muted)
                    .child(icon_el),
            )
            .when(!hide_label, |el| {
                el.child(
                    div()
                        .max_w(px(SECTION_CELL_WIDTH - 6.))
                        .text_size(px(10.))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(name),
                )
            })
            .on_drag(drag_payload, |info, _pos, _window, cx: &mut App| {
                cx.new(|_| info.clone())
            })
            .on_click(move |_event, window, cx: &mut App| {
                let launch_entry = launch_entry.clone();
                entity_for_launch.update(cx, |this, cx| {
                    this.launch_entry(&launch_entry);
                    cx.notify();
                });
                crate::launcher::close_this(window, cx);
            });

        if is_favorite {
            cell = cell.on_drop::<LauncherDrag>(move |drag, _window, cx: &mut App| {
                let drag = drag.clone();
                entity_for_drop.update(cx, |this, cx| {
                    this.handle_drop_on_favorite_cell(&drag, index);
                    cx.notify();
                });
            });
        } else {
            cell = cell.on_drop::<LauncherDrag>(move |drag, _window, cx: &mut App| {
                let drag = drag.clone();
                let target = drop_target.clone();
                entity_for_drop.update(cx, |this, cx| {
                    this.handle_drop_create_folder(&drag, &target);
                    cx.notify();
                });
            });
        }

        cell
    }

    fn render_folder_tile(&self, theme: &Theme, entity: Entity<Self>, folder: &Folder) -> impl IntoElement {
        let is_expanded = self.expanded.as_deref() == Some(folder.id.as_str());
        let is_renaming = self.rename.as_deref() == Some(folder.id.as_str());
        let letter = folder
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        let name = SharedString::from(folder.name.clone());
        let folder_id = folder.id.clone();
        let entity_for_drop = entity.clone();
        let entity_for_toggle = entity.clone();
        let entity_for_rename = entity.clone();

        let mut tile = div()
            .id(format!("launcher-folder-{}", folder.id))
            .w(px(SECTION_CELL_WIDTH))
            .h(px(SECTION_CELL_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(8.))
            .cursor_pointer()
            .when(is_expanded, |el| el.bg(theme.bg.selection))
            .when(!is_expanded, |el| el.hover(|s| s.bg(theme.interactive.hover)))
            .on_drop::<LauncherDrag>({
                let folder_id = folder_id.clone();
                move |drag, _window, cx: &mut App| {
                    let drag = drag.clone();
                    entity_for_drop.update(cx, |this, cx| {
                        this.handle_drop_on_folder(&drag, &folder_id);
                        cx.notify();
                    });
                }
            })
            .child(
                div()
                    .size(px(SECTION_ICON))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(7.))
                    .bg(theme.accent.secondary.opacity(0.18))
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(theme.accent.secondary)
                            .child(letter),
                    ),
            );

        // Name label, or the rename Input when renaming.
        if is_renaming {
            tile = tile.child(
                div()
                    .max_w(px(SECTION_CELL_WIDTH - 6.))
                    .child(
                        Input::new(&self.rename_input)
                            .appearance(false)
                            .text_color(theme.text.primary)
                            .text_size(px(11.)),
                    ),
            );
        } else {
            tile = tile.child(
                div()
                    .max_w(px(SECTION_CELL_WIDTH - 6.))
                    .text_size(px(10.))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(name.clone()),
            );
        }

        // Click on the tile body toggles expand — but NOT while renaming
        // (a click into the rename Input must not collapse the folder).
        if !is_renaming {
            tile = tile.on_click({
                let folder_id = folder_id.clone();
                move |_event, _window, cx: &mut App| {
                    entity_for_toggle.update(cx, |this, cx| {
                        this.toggle_folder(&folder_id);
                        cx.notify();
                    });
                }
            });
        }

        // Decorative expand chevron (sits in the tile's toggle hit area).
        tile = tile.child(
            div()
                .id(format!("launcher-folder-chevron-{}", folder.id))
                .h(px(14.))
                .px(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(theme.bg.elevated))
                .text_color(theme.text.faint)
                .text_size(px(9.))
                .child(if is_expanded { "▾" } else { "▸" }),
        );

        // Pencil enters rename mode. `stop_propagation` keeps the tile's toggle
        // from also firing on the same click (nested on_click bubbles).
        tile = tile.child(
            div()
                .id(format!("launcher-folder-pencil-{}", folder.id))
                .h(px(14.))
                .px(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(theme.bg.elevated))
                .text_color(theme.text.faint)
                .text_size(px(9.))
                .child("✎")
                .on_click({
                    let folder_id = folder.id.clone();
                    move |_event, window, cx: &mut App| {
                        cx.stop_propagation();
                        entity_for_rename.update(cx, |this, cx| {
                            this.start_rename(folder_id.clone(), window, cx);
                            cx.notify();
                        });
                    }
                }),
        );

        tile
    }

    fn render_cell(
        &self,
        theme: &Theme,
        entry: &AppEntry,
        flat: usize,
        is_selected: bool,
        entity: Entity<Self>,
    ) -> impl IntoElement {
        let entry_for_click = entry.clone();
        let entry_for_menu = entry.clone();
        let icon_el = resolve_app_icon(entry, theme, GRID_ICON);
        let name = SharedString::from(entry.name.clone());
        let is_new = self.new_ids.contains(&entry.id);
        let drag_payload = LauncherDrag {
            app_id: entry.id.clone(),
            app_name: entry.name.clone(),
        };
        let drop_target = entry.id.clone();
        let entity_for_drop = entity.clone();
        let entity_for_launch = entity.clone();

        div()
            .id(format!("launcher-cell-{flat}"))
            .w(px(CELL_WIDTH))
            .h(px(CELL_HEIGHT))
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(8.))
            .cursor_pointer()
            .when(is_selected, |el| el.bg(theme.bg.selection))
            .when(!is_selected, |el| el.hover(|s| s.bg(theme.interactive.hover)))
            // T265-C "new" badge: dot in the cell corner for a fresh .desktop.
            .when(is_new, |el| {
                el.child(
                    div()
                        .absolute()
                        .top(px(5.))
                        .right(px(5.))
                        .size(px(6.))
                        .rounded_full()
                        .bg(theme.accent.primary),
                )
            })
            // T275 Часть D: right-click a cell to pin/unpin it.
            .on_mouse_down(
                MouseButton::Right,
                move |event, window, cx: &mut App| {
                    let anchor = Bounds::new(event.position, Size::new(px(1.), px(1.)));
                    app_menu::open(cx, anchor, event.position, window.window_handle(), entry_for_menu.clone());
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
            // Internal DnD source + drop target (folder creation).
            .on_drag(drag_payload, |info, _pos, _window, cx: &mut App| {
                cx.new(|_| info.clone())
            })
            .on_drop::<LauncherDrag>(move |drag, _window, cx: &mut App| {
                let drag = drag.clone();
                let target = drop_target.clone();
                entity_for_drop.update(cx, |this, cx| {
                    this.handle_drop_create_folder(&drag, &target);
                    cx.notify();
                });
            })
            .on_click(move |_event, window, cx: &mut App| {
                let entry_for_click = entry_for_click.clone();
                entity_for_launch.update(cx, |this, cx| {
                    this.launch_entry(&entry_for_click);
                    cx.notify();
                });
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
