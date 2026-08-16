//! Start-menu view (T265-H): left rail (Places + Categories + user/power)
//! and right main (search + breadcrumb + app grid).
//!
//! Canon: `docs/design/chronos-start-menu.html`. This is a *second surface*
//! over the OSD launcher's model, not a second model: search (`FuzzySearch`),
//! frecency ranking, favorites/recents resolution, category building and
//! launch all come from the shared `launcher/*` modules. Folders / DnD are
//! NOT ported here (out of scope — B/C land in the compact menu later).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use gpui::{
    App, Entity, FocusHandle, ImageSource, KeyDownEvent, Render, ScrollHandle, SharedString,
    Subscription, Window, div, img, prelude::*, px, svg,
};

use chronos_services::applications::frecency;
use chronos_services::{AppEntry, Service};
use chronos_ui::{Theme, WindowRootExt};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::icon_resolution::resolve_icon;
use crate::launcher::favorites::{index_by_id, resolve_favorites, top_recents};
use crate::launcher::grid::{build_categories, filter_by_category, move_2d, Move2D};
use crate::launcher::launch::launch;
use crate::launcher::launcher_config::{self, LauncherConfig};
use crate::launcher::search::FuzzySearch;
use crate::launcher::system_actions;
use crate::power::{ARM_TIMEOUT, ArmState, PowerAction, is_confirming_click, on_click, on_timeout};
use crate::state;

/// Rail width (mockup 210px).
const RAIL_WIDTH: f32 = 210.;
/// Search row height.
const SEARCH_H: f32 = 52.;
/// Breadcrumb row height.
const CRUMBS_H: f32 = 28.;
/// Grid geometry: the main area is 720 − 210 = 510px; with 12px padding and
/// 6px gaps, five 84px cells fit (mockup `minmax(84px, 1fr)`).
const GRID_COLUMNS: usize = 5;
const CELL_WIDTH: f32 = 84.;
const CELL_HEIGHT: f32 = 102.;
const GRID_GAP: f32 = 6.;
const GRID_ICON: f32 = 34.;
/// PageUp/PageDown stride (rows).
const PAGE_ROWS: usize = 4;
/// Soft cap on ranked results (grid scrolls; this bounds per-frame cost).
const MAX_RESULTS: usize = 200;

/// Left-rail nav target (mockup's Places + category entries).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Nav {
    All,
    Pinned,
    Recent,
    Files,
    Category(String),
}

impl Nav {
    fn breadcrumb(&self) -> String {
        match self {
            Nav::All => "All Apps".to_string(),
            Nav::Pinned => "Pinned".to_string(),
            Nav::Recent => "Recent".to_string(),
            Nav::Files => "Files".to_string(),
            Nav::Category(cat) => format!("All Apps › {cat}"),
        }
    }
}

/// Rail power-mini actions — the mockup's five (Hibernate lives in the OSD
/// header only; the rail keeps the compact row to 5 tiles).
fn rail_power_actions() -> [PowerAction; 5] {
    [
        PowerAction::Lock,
        PowerAction::Sleep,
        PowerAction::LogOut,
        PowerAction::Restart,
        PowerAction::Shutdown,
    ]
}

fn power_icon(action: PowerAction) -> &'static str {
    match action {
        PowerAction::Lock => "icons/lock.svg",
        PowerAction::Sleep => "icons/suspend.svg",
        PowerAction::Hibernate => "icons/suspend.svg",
        PowerAction::LogOut => "icons/sign-out.svg",
        PowerAction::Restart => "icons/arrows-clockwise.svg",
        PowerAction::Shutdown => "icons/power.svg",
    }
}

/// Pure nav filter: map the rail selection + search results into the grid's
/// visible list. Pinned/Recent preserve their own order (not the search rank);
/// typing narrows them to entries that also matched the query.
fn nav_filter(
    nav: &Nav,
    pattern: &str,
    results: &[AppEntry],
    favorites: &[AppEntry],
    recents: &[AppEntry],
) -> Vec<AppEntry> {
    match nav {
        Nav::All => results.to_vec(),
        Nav::Pinned => {
            if pattern.trim().is_empty() {
                favorites.to_vec()
            } else {
                keep_in_order(favorites, results)
            }
        }
        Nav::Recent => {
            if pattern.trim().is_empty() {
                recents.to_vec()
            } else {
                keep_in_order(recents, results)
            }
        }
        Nav::Files => Vec::new(),
        Nav::Category(cat) => filter_by_category(results, Some(cat)),
    }
}

/// Keep `source`'s order but drop entries absent from `matched`.
fn keep_in_order(source: &[AppEntry], matched: &[AppEntry]) -> Vec<AppEntry> {
    let ids: HashSet<&str> = matched.iter().map(|e| e.id.as_str()).collect();
    source
        .iter()
        .filter(|e| ids.contains(e.id.as_str()))
        .cloned()
        .collect()
}

/// Compact start menu over the shared launcher model.
pub struct StartMenuView {
    search: FuzzySearch,
    input: Entity<InputState>,
    pattern: String,
    selected: usize,
    /// Ranked + search-filtered entries (all listed apps, frecency-ranked).
    results: Vec<AppEntry>,
    /// Distinct Main Categories across the full catalog (rail badges).
    categories: Vec<(String, usize)>,
    /// `results` filtered by the active nav selection — what the grid shows.
    visible: Vec<AppEntry>,
    raw_entries: Vec<AppEntry>,
    /// Listed entries minus user-hidden ids.
    all: Vec<AppEntry>,
    by_id: HashMap<String, AppEntry>,
    config: LauncherConfig,
    favorites: Vec<AppEntry>,
    recents: Vec<AppEntry>,
    nav: Nav,
    columns: usize,
    display_name: String,
    user_initial: String,
    face_path: Option<PathBuf>,
    power_arm: ArmState,
    scroll: ScrollHandle,
    focus_handle: FocusHandle,
    _input_sub: Subscription,
}

impl StartMenuView {
    pub fn new(cx: &mut gpui::Context<Self>, input: Entity<InputState>, window: &Window) -> Self {
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

        let config = launcher_config::get();
        let display_name =
            system_actions::user_full_name().unwrap_or_else(system_actions::user_name);
        let mut view = Self {
            search: FuzzySearch::new(),
            input,
            pattern,
            selected: 0,
            results: Vec::new(),
            categories: Vec::new(),
            visible: Vec::new(),
            raw_entries: Vec::new(),
            all: Vec::new(),
            by_id: HashMap::new(),
            config,
            favorites: Vec::new(),
            recents: Vec::new(),
            nav: Nav::All,
            columns: GRID_COLUMNS,
            display_name,
            user_initial: system_actions::user_initial(),
            face_path: system_actions::face_path(),
            power_arm: ArmState::default(),
            scroll: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            _input_sub: sub,
        };
        view.set_entries(entries);
        cx.notify();

        // Live desktop-entry updates.
        let signal = state::AppState::applications(cx).subscribe();
        state::watch(cx, signal, |this, state, cx| {
            this.set_entries(state.entries);
            cx.notify();
        });

        // Live config updates (favorites / hidden / recents limit from the
        // settings page or the OSD's context menu — one store, two surfaces).
        let config_signal = launcher_config::subscribe();
        state::watch(cx, config_signal, |this, _, cx| {
            this.config = launcher_config::get();
            this.apply_hidden_filter();
            this.recompute_sections();
            this.refresh_results();
            cx.notify();
        });

        view
    }

    fn set_entries(&mut self, entries: Vec<AppEntry>) {
        self.raw_entries = entries;
        self.apply_hidden_filter();
        self.recompute_sections();
        self.refresh_results();
    }

    fn apply_hidden_filter(&mut self) {
        let hidden: HashSet<String> = self.config.hidden.iter().cloned().collect();
        self.all = self
            .raw_entries
            .iter()
            .filter(|e| !hidden.contains(&e.id))
            .cloned()
            .collect();
        self.by_id = index_by_id(&self.all);
        self.search.set_items(self.all.clone());
    }

    fn recompute_sections(&mut self) {
        self.favorites = resolve_favorites(
            &self.config.favorites.order,
            &self.by_id,
            self.config.favorites.sort_alpha,
        );
        let data = frecency::cached();
        let now = frecency::now();
        self.recents = top_recents(&self.all, &data, now, self.config.recents.limit);
        self.categories = build_categories(&self.all);
    }

    fn refresh_results(&mut self) {
        self.search.update_pattern(&self.pattern);
        let raw = self.search.results(MAX_RESULTS);
        let data = frecency::cached();
        let now = frecency::now();
        self.results = frecency::rank(raw, &self.pattern, &data, now);
        self.apply_nav_filter();
    }

    fn apply_nav_filter(&mut self) {
        self.visible = nav_filter(
            &self.nav,
            &self.pattern,
            &self.results,
            &self.favorites,
            &self.recents,
        );
        if self.selected >= self.visible.len() {
            self.selected = self.visible.len().saturating_sub(1);
        }
        self.scroll_to_selected();
    }

    fn scroll_to_selected(&self) {
        if self.visible.is_empty() {
            return;
        }
        // Scroll container children: [section label] + one child per grid row.
        self.scroll
            .scroll_to_item(1 + self.selected / self.columns);
    }

    fn select_nav(&mut self, nav: Nav) {
        self.nav = nav;
        self.selected = 0;
        self.apply_nav_filter();
    }

    fn navigate_grid(&mut self, mv: Move2D, window: &mut Window) {
        let next = move_2d(self.selected, self.columns, self.visible.len(), mv, PAGE_ROWS);
        if next != self.selected {
            self.selected = next;
            self.scroll_to_selected();
            window.refresh();
        }
    }

    fn launch_selected(&mut self, window: &mut Window, cx: &mut App) {
        if self.nav == Nav::Files {
            Self::open_files(window, cx);
            return;
        }
        if let Some(entry) = self.visible.get(self.selected).cloned() {
            frecency::record_launch(&entry.id);
            if let Err(err) = launch(&entry.exec) {
                tracing::error!("start_menu: failed to launch {}: {:#}", entry.name, err);
            }
        }
        crate::start_menu::close_this(window, cx);
    }

    fn open_files(window: &mut Window, cx: &mut App) {
        crate::side_panel_right::select_tab(
            crate::side_panel_right::tabs::PanelTab::Files,
            cx,
        );
        crate::start_menu::close_this(window, cx);
    }

    fn launch_entry(&self, entry: &AppEntry) {
        frecency::record_launch(&entry.id);
        if let Err(err) = launch(&entry.exec) {
            tracing::error!("start_menu: failed to launch {}: {:#}", entry.name, err);
        }
    }

    // --- Power mini (reuses the shared arm/confirm state machine) ---

    fn on_power_click(&mut self, action: PowerAction, cx: &mut gpui::Context<Self>) {
        if !action.needs_confirm() {
            self.execute_power(action, cx);
            return;
        }
        if is_confirming_click(&self.power_arm, action) {
            self.execute_power(action, cx);
            self.power_arm = ArmState::Idle;
            cx.notify();
            return;
        }
        let armed = on_click(self.power_arm, action);
        self.power_arm = armed;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(ARM_TIMEOUT).await;
            if let Err(err) = this.update(cx, |this, cx| {
                if this.power_arm == armed {
                    this.power_arm = on_timeout(armed);
                    cx.notify();
                }
            }) {
                tracing::warn!("start_menu: power arm timeout could not disarm ({err})");
            }
        })
        .detach();
    }

    fn execute_power(&self, action: PowerAction, cx: &App) {
        match action {
            PowerAction::Lock => state::AppState::power(cx).lock(),
            PowerAction::LogOut => state::AppState::power(cx).log_out(),
            PowerAction::Sleep => state::AppState::power(cx).suspend(),
            PowerAction::Hibernate => state::AppState::power(cx).hibernate(),
            PowerAction::Restart => state::AppState::power(cx).restart(),
            PowerAction::Shutdown => state::AppState::power(cx).shutdown(),
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut App) {
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => crate::start_menu::close_this(window, cx),
            "enter" => self.launch_selected(window, cx),
            "up" | "down" => {
                self.navigate_grid(if key == "up" { Move2D::Up } else { Move2D::Down }, window);
            }
            "left" | "right" => {
                self.navigate_grid(if key == "left" { Move2D::Left } else { Move2D::Right }, window);
            }
            "home" => self.navigate_grid(Move2D::Home, window),
            "end" => self.navigate_grid(Move2D::End, window),
            "pageup" => self.navigate_grid(Move2D::PageUp, window),
            "pagedown" => self.navigate_grid(Move2D::PageDown, window),
            _ => {}
        }
    }
}

impl Render for StartMenuView {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx);
        let entity = cx.entity();

        div()
            .window_font(theme)
            .size_full()
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.subtle)
            .rounded(px(12.))
            .shadow(card_shadow(theme))
            .overflow_hidden()
            .flex()
            .flex_row()
            .text_color(theme.text.primary)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, window, cx| this.handle_key(event, window, cx)))
            .child(self.render_rail(theme, entity.clone()))
            .child(self.render_main(theme, entity))
    }
}

impl StartMenuView {
    // --- Left rail ---

    fn render_rail(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        div()
            .id("start-menu-rail")
            .w(px(RAIL_WIDTH))
            .h_full()
            .flex_none()
            .bg(theme.bg.tertiary)
            .border_r_1()
            .border_color(theme.border.subtle.opacity(0.5))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .id("start-menu-rail-nav")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .py(px(6.))
                    .child(self.render_places(theme, entity.clone()))
                    .child(
                        div()
                            .mx(px(16.))
                            .mt(px(4.))
                            .mb(px(6.))
                            .h(px(1.))
                            .bg(theme.border.subtle.opacity(0.45)),
                    )
                    .child(self.render_categories(theme, entity.clone())),
            )
            .child(self.render_rail_footer(theme, entity))
    }

    fn render_places(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let total = self.all.len();
        div()
            .flex()
            .flex_col()
            .px(px(8.))
            .child(rail_label(theme, "Places"))
            .child(nav_item(
                theme,
                entity.clone(),
                Nav::All,
                "All Apps",
                Some(total),
                self.nav == Nav::All,
            ))
            .child(nav_item(
                theme,
                entity.clone(),
                Nav::Pinned,
                "Pinned",
                Some(self.favorites.len()),
                self.nav == Nav::Pinned,
            ))
            .child(nav_item(
                theme,
                entity.clone(),
                Nav::Recent,
                "Recent",
                Some(self.recents.len()),
                self.nav == Nav::Recent,
            ))
            .child(nav_item(
                theme,
                entity.clone(),
                Nav::Files,
                "Files",
                None,
                self.nav == Nav::Files,
            ))
    }

    fn render_categories(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let nav = self.nav.clone();
        div()
            .flex()
            .flex_col()
            .px(px(8.))
            .py(px(6.))
            .child(rail_label(theme, "Categories"))
            .children(self.categories.iter().map(|(cat, count)| {
                let active = matches!(&nav, Nav::Category(c) if c == cat);
                let cat_owned = cat.clone();
                nav_item(
                    theme,
                    entity.clone(),
                    Nav::Category(cat_owned),
                    cat,
                    Some(*count),
                    active,
                )
            }))
    }

    fn render_rail_footer(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .border_t_1()
            .border_color(theme.border.subtle.opacity(0.5))
            .p(px(10.))
            .child(self.render_user_card(theme))
            .child(self.render_power_mini(theme, entity))
    }

    fn render_avatar(&self, theme: &Theme) -> impl IntoElement {
        if let Some(path) = &self.face_path {
            let src: ImageSource = path.clone().into();
            return div()
                .size(px(32.))
                .rounded_full()
                .overflow_hidden()
                .border_1()
                .border_color(theme.border.subtle)
                .child(img(src).size(px(32.)))
                .into_any_element();
        }
        div()
            .size(px(32.))
            .rounded_full()
            .bg(theme.accent.primary)
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.bg.primary)
            .text_size(px(13.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(self.user_initial.clone())
            .into_any_element()
    }

    fn render_user_card(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .p(px(8.))
            .rounded(px(8.))
            .hover(|s| s.bg(theme.bg.elevated))
            .child(self.render_avatar(theme))
            .child(
                div()
                    .flex_col()
                    .min_w(px(0.))
                    .flex_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(self.display_name.clone()),
                    )
                    .child(
                        div()
                            .font_family(theme.font_mono)
                            .text_size(px(10.))
                            .text_color(theme.text.faint)
                            .child("chronos-shell"),
                    ),
            )
    }

    fn render_power_mini(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        // Eagerly build the five buttons so the temporary `actions` array and
        // the iterator borrow do not outlive this render call.
        let buttons: Vec<_> = rail_power_actions()
            .iter()
            .map(|action| {
                // Copy the enum out of the iterator so the click closure owns
                // the value (the array is a temporary; a borrowed `&action`
                // would dangle past this call).
                let action = *action;
                let armed = self.power_arm == ArmState::Armed(action);
                let danger = action == PowerAction::Shutdown || armed;
                let disabled = !system_actions::available(action);
                let color = if disabled {
                    theme.text.faint
                } else if danger {
                    theme.accent.secondary
                } else {
                    theme.text.muted
                };
                let entity = entity.clone();
                div()
                    .id(format!("start-menu-power-{action:?}"))
                    .size(px(30.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg.elevated))
                    .child(svg().path(power_icon(action)).size(px(15.)).text_color(color))
                    .on_click(move |_event, _window, cx: &mut App| {
                        entity.update(cx, |this, cx| this.on_power_click(action, cx));
                    })
            })
            .collect();

        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .mt(px(8.))
            .children(buttons)
    }

    // --- Right main ---

    fn render_main(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        div()
            .id("start-menu-main")
            .flex_1()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_search(theme))
            .child(self.render_crumbs(theme))
            .child(self.render_grid(theme, entity))
    }

    fn render_search(&self, theme: &Theme) -> impl IntoElement {
        let input = Input::new(&self.input)
            .appearance(false)
            .cleanable(true)
            .text_color(theme.text.primary)
            .text_size(px(14.5));

        div()
            .id("start-menu-search")
            .flex_none()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(16.))
            .h(px(SEARCH_H))
            .border_b_1()
            .border_color(theme.border.subtle.opacity(0.5))
            .child(
                svg()
                    .path("icons/chronos-sigil.svg")
                    .size(px(16.))
                    .text_color(theme.text.muted),
            )
            .child(div().flex_1().min_w(px(0.)).child(input))
    }

    fn render_crumbs(&self, theme: &Theme) -> impl IntoElement {
        div()
            .id("start-menu-crumbs")
            .flex_none()
            .flex()
            .items_center()
            .px(px(16.))
            .h(px(CRUMBS_H))
            .text_size(px(11.))
            .text_color(theme.accent.primary)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(self.nav.breadcrumb())
    }

    fn render_grid(&self, theme: &Theme, entity: Entity<Self>) -> impl IntoElement {
        let selected = self.selected;
        let is_files = self.nav == Nav::Files;
        let is_recent = self.nav == Nav::Recent;
        let favorite_ids: HashSet<&str> = self.favorites.iter().map(|e| e.id.as_str()).collect();
        let frecency = if is_recent {
            Some(frecency::cached())
        } else {
            None
        };
        let now = if is_recent { Some(frecency::now()) } else { None };
        let rows: Vec<&[AppEntry]> = self.visible.chunks(self.columns).collect();
        let label = self.nav.breadcrumb();

        div()
            .id("start-menu-grid")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .px(px(12.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .when(is_files, |el| {
                el.child(
                    div()
                        .id("start-menu-files-empty")
                        .py(px(48.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .text_color(theme.text.faint)
                        .text_sm()
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.text.primary))
                        .on_click(|_event, window, cx| {
                            StartMenuView::open_files(window, cx);
                        })
                        .child(
                            svg()
                                .path("icons/folder.svg")
                                .size(px(32.))
                                .text_color(theme.text.muted),
                        )
                        .child("Open the File Manager to browse folders and files."),
                )
            })
            .when(!is_files && self.visible.is_empty(), |el| {
                el.child(
                    div()
                        .py(px(34.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text.faint)
                        .text_sm()
                        .child(format!("No matches for “{}”", self.pattern)),
                )
            })
            .when(!is_files && !self.visible.is_empty(), |el| {
                el.child(section_label(theme, &label))
                    .children(rows.into_iter().enumerate().map(|(ri, row)| {
                        div()
                            .flex()
                            .gap(px(GRID_GAP))
                            .pb(px(GRID_GAP))
                            .children(row.iter().enumerate().map(|(ci, entry)| {
                                let flat = ri * self.columns + ci;
                                let recent_time = frecency.as_ref().and_then(|data| {
                                    data.entries.get(&entry.id).map(|rec| {
                                        relative_launch_time(rec.last_launched_at, now.unwrap_or(0))
                                    })
                                });
                                self.render_cell(
                                    theme,
                                    entry,
                                    flat == selected,
                                    favorite_ids.contains(entry.id.as_str()),
                                    recent_time,
                                    entity.clone(),
                                )
                            }))
                    }))
            })
    }

    fn render_cell(
        &self,
        theme: &Theme,
        entry: &AppEntry,
        is_selected: bool,
        is_favorite: bool,
        recent_time: Option<String>,
        entity: Entity<Self>,
    ) -> impl IntoElement {
        let entry_for_click = entry.clone();
        let icon_el = resolve_app_icon(entry, theme, GRID_ICON);
        let name = SharedString::from(entry.name.clone());
        let icon_color = if is_selected {
            theme.accent.primary
        } else {
            theme.text.primary
        };

        div()
            .id(format!("start-menu-cell-{}", entry.id))
            .w(px(CELL_WIDTH))
            .h(px(CELL_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(5.))
            .rounded(px(8.))
            .cursor_pointer()
            .when(is_selected, |el| el.bg(theme.bg.selection))
            .when(!is_selected, |el| el.hover(|s| s.bg(theme.interactive.hover)))
            .child(
                div()
                    .relative()
                    .size(px(GRID_ICON))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if is_selected {
                        theme.accent.primary
                    } else {
                        theme.border.default
                    })
                    .text_color(icon_color)
                    .child(icon_el)
                    .when(is_favorite, |el| {
                        el.child(
                            div()
                                .absolute()
                                .top(px(-3.))
                                .right(px(-3.))
                                .size(px(8.))
                                .rounded_full()
                                .bg(theme.bg.elevated)
                                .border_2()
                                .border_color(theme.bg.primary),
                        )
                    }),
            )
            .child(
                div()
                    .max_w(px(CELL_WIDTH - 8.))
                    .text_size(px(10.5))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(is_selected, |el| el.text_color(theme.accent.primary))
                    .child(name),
            )
            .when_some(recent_time, |el, time| {
                el.child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme.text.faint)
                        .child(time),
                )
            })
            .on_click(move |_event, window, cx: &mut App| {
                let entry = entry_for_click.clone();
                entity.update(cx, |this, cx| {
                    this.launch_entry(&entry);
                    cx.notify();
                });
                crate::start_menu::close_this(window, cx);
            })
    }
}

/// Small uppercase rail section label (mockup `.rail-label`).
fn rail_label(theme: &Theme, text: &'static str) -> impl IntoElement {
    div()
        .px(px(8.))
        .pt(px(4.))
        .pb(px(6.))
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.text.faint)
        .child(text.to_uppercase())
}

/// A rail nav item (Place or Category). Badge is optional; `active` colors it.
fn nav_item(
    theme: &Theme,
    entity: Entity<StartMenuView>,
    nav: Nav,
    label: &str,
    badge: Option<usize>,
    active: bool,
) -> impl IntoElement {
    div()
        .relative()
        .flex()
        .items_center()
        .gap(px(10.))
        .h(px(34.))
        .px(px(10.))
        .rounded(px(7.))
        .cursor_pointer()
        .text_sm()
        .font_weight(gpui::FontWeight::MEDIUM)
        .when(active, |el| el.bg(theme.bg.selection).text_color(theme.accent.primary))
        .when(!active, |el| el.text_color(theme.text.primary).hover(|s| s.bg(theme.bg.elevated)))
        .when(active, |el| {
            el.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(6.))
                    .bottom(px(6.))
                    .w(px(3.))
                    .rounded_tr(px(2.))
                    .rounded_br(px(2.))
                    .bg(theme.accent.primary),
            )
        })
        .child(
            div()
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(label.to_string()),
        )
        .child(div().flex_1())
        .when(badge.is_some(), |el| {
            el.child(
                div()
                    .font_family(theme.font_mono)
                    .text_size(px(10.))
                    .text_color(if active { theme.accent.primary } else { theme.text.muted })
                    .child(badge.unwrap_or(0).to_string()),
            )
        })
        .id(format!("start-menu-nav-{label}"))
        .on_click(move |_event, _window, cx: &mut App| {
            entity.update(cx, |this, cx| {
                this.select_nav(nav.clone());
                cx.notify();
            });
        })
}

/// Grid section label (mockup `.section-label` with the trailing hairline).
fn section_label(theme: &Theme, title: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .py(px(12.))
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.text.faint)
        .child(title.to_uppercase())
        .child(div().flex_1().h(px(1.)).bg(theme.border.subtle.opacity(0.4)))
}

/// Card shadow for the start menu (mockup `--shadow`).
fn card_shadow(theme: &Theme) -> Vec<gpui::BoxShadow> {
    vec![
        gpui::BoxShadow::new(px(0.), px(10.), theme.bg.primary.opacity(0.45)).blur_radius(px(40.)),
        gpui::BoxShadow::new(px(0.), px(1.), theme.bg.primary.opacity(0.3)).blur_radius(px(2.)),
    ]
}

/// Resolve an application icon (system theme) or fall back to a letter glyph.
/// Compact relative age for the Recent grid (mockup `2m ago` / `1h ago`).
fn relative_launch_time(last_launched_at: i64, now: i64) -> String {
    let secs = (now - last_launched_at).max(0);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

/// Resolve an application icon (system theme) or fall back to a letter glyph.
fn resolve_app_icon(entry: &AppEntry, theme: &Theme, size: f32) -> gpui::AnyElement {
    if let Some(name) = entry.icon.as_deref() {
        if let Some(path_buf) = resolve_icon(name) {
            let src: ImageSource = path_buf.into();
            return img(src).size(px(size)).into_any_element();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, name: &str) -> AppEntry {
        AppEntry::fixture(id, name)
    }

    #[test]
    fn nav_filter_all_returns_ranked_results() {
        let results = vec![e("a", "A"), e("b", "B")];
        assert_eq!(
            nav_filter(&Nav::All, "", &results, &[], &[])
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn nav_filter_files_is_empty() {
        assert!(nav_filter(&Nav::Files, "", &[e("a", "A")], &[], &[]).is_empty());
    }

    #[test]
    fn nav_filter_pinned_preserves_order_and_narrows_on_query() {
        let favorites = vec![e("kitty", "Kitty"), e("firefox", "Firefox")];
        let results = vec![e("firefox", "Firefox")];
        // Empty query: pinned order as-is.
        assert_eq!(
            nav_filter(&Nav::Pinned, "", &results, &favorites, &[])
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
            vec!["kitty", "firefox"]
        );
        // Typed query: keep only entries that matched, still pinned order.
        assert_eq!(
            nav_filter(&Nav::Pinned, "fire", &results, &favorites, &[])
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
            vec!["firefox"]
        );
    }

    #[test]
    fn nav_filter_category_filters_by_category() {
        let dev = AppEntry {
            categories: vec!["Development".into()],
            ..AppEntry::fixture("code", "Code")
        };
        let web = AppEntry {
            categories: vec!["Network".into()],
            ..AppEntry::fixture("firefox", "Firefox")
        };
        let results = vec![dev.clone(), web.clone()];
        assert_eq!(
            nav_filter(&Nav::Category("Development".into()), "", &results, &[], &[])
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>(),
            vec!["code"]
        );
    }

    #[test]
    fn breadcrumbs_are_stable() {
        assert_eq!(Nav::All.breadcrumb(), "All Apps");
        assert_eq!(Nav::Pinned.breadcrumb(), "Pinned");
        assert_eq!(Nav::Recent.breadcrumb(), "Recent");
        assert_eq!(Nav::Files.breadcrumb(), "Files");
        assert_eq!(Nav::Category("Dev".into()).breadcrumb(), "All Apps › Dev");
    }

    #[test]
    fn relative_launch_time_buckets() {
        assert_eq!(relative_launch_time(100, 130), "just now");
        assert_eq!(relative_launch_time(0, 120), "2m ago");
        assert_eq!(relative_launch_time(0, 3 * 3600), "3h ago");
        assert_eq!(relative_launch_time(0, 2 * 86_400), "2d ago");
    }

    #[test]
    fn rail_power_actions_match_mockup_five() {
        assert_eq!(
            rail_power_actions(),
            [
                PowerAction::Lock,
                PowerAction::Sleep,
                PowerAction::LogOut,
                PowerAction::Restart,
                PowerAction::Shutdown,
            ]
        );
    }

    #[test]
    fn power_icons_resolve_for_every_rail_action() {
        for action in rail_power_actions() {
            assert!(power_icon(action).starts_with("icons/"));
        }
        assert_eq!(power_icon(PowerAction::Lock), "icons/lock.svg");
        assert_eq!(power_icon(PowerAction::Sleep), "icons/suspend.svg");
    }
}
