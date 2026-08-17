//! T279 / Slice A2 — the Sessions tab body.
//!
//! A standalone session-list entity rendered inside the content canvas when
//! `active_tab == Sessions`. It owns its own `ThreadStore` handle + list
//! items and reuses `sessions_list::ThreadListItem` title helpers — no title
//! logic is duplicated here.
//!
//! T287-B: the list moved onto the gpui-component kit. Search is a kit
//! `Input` bound to `search` (filtering `short_title()`); rows render through
//! `v_virtual_list`; and each row's `⋯` / right-click opens a kit `PopupMenu`
//! (an anchored popup window, same engine as dock/tray/launcher) with
//! Pin/Unpin, Archive/Unarchive, Rename (inline kit `Input`), and Delete
//! (inline confirmation modal). All four actions write straight through
//! `ThreadStore` — no new layer.
//!
//! Selection emits `SessionsEvent::SelectThread` upward; the coordinator
//! (`select_session` reducer) writes the id into the SoT and switches to
//! Chat. The tab itself does not own the active session id — that lives in
//! `SidePanelLeftState_`.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, Bounds, ClickEvent, Context, DisplayId, DismissEvent, Entity, Focusable,
    Global, IntoElement, MouseButton, MouseDownEvent, Pixels, Point, Render, Size, Subscription,
    WeakEntity, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div, layer_shell::*, point,
    popup::{
        PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupNotSupportedError, PopupOptions,
    },
    prelude::*, px,
};

use chronos_services::threads::ThreadStore;
use chronos_ui::{Theme, WindowRootExt};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::{Root, v_virtual_list};

use crate::side_panel_left::sessions_list::{ThreadListItem, format_timestamp};
use crate::side_panel_left::workspace_view::WorkspaceView;

/// Fixed row height for the virtual list (title + timestamp + ⋯ in one line).
const ROW_HEIGHT: f32 = 44.;
/// Context-menu card width (px).
const MENU_WIDTH: f32 = 200.;
/// Context-menu height estimate for the click-catcher hole (4 items).
const MENU_HEIGHT: f32 = 116.;

/// Event emitted by `SessionsTab` to the workspace coordinator.
#[derive(Clone, Debug)]
pub enum SessionsEvent {
    /// User clicked a thread — coordinator sets active id + opens Chat.
    SelectThread(String),
    /// User clicked "+ New".
    CreateThread,
}

/// Immutable snapshot of a row, carried into the `PopupMenu` window so the
/// menu items know the thread's id, pin/archive state and title without
/// reaching back into the tab entity.
#[derive(Clone)]
struct ThreadMenuSnapshot {
    id: String,
    pinned: bool,
    archived: bool,
}

/// The Sessions tab body — a full-panel thread list backed by `ThreadStore`.
pub struct SessionsTab {
    /// Loaded thread list, sorted pinned-first then updated_at desc.
    threads: Vec<ThreadListItem>,
    /// Search filter query (filters `short_title()`).
    search: String,
    /// Currently selected thread id — written on click, painted in render.
    /// The SoT (`SidePanelLeftState_.active_session_id`) remains the source
    /// of truth for the coordinator; this mirror exists purely so the row
    /// can paint its selected background without re-querying the global.
    selected: Option<String>,
    /// The project this list is scoped to. `None` before any project is
    /// active — shows no project-scoped threads (empty list is honest).
    project_path: Option<PathBuf>,
    /// Weak handle to the owning `WorkspaceView` — used to forward events
    /// to the coordinator without this tab owning panel state.
    coordinator: WeakEntity<WorkspaceView>,
    /// Cached store handle (opened on scope load, reused by the actions).
    store: Option<ThreadStore>,
    /// Whether archived threads are included (default hidden).
    show_archived: bool,
    /// Kit search input state, created lazily on first render (needs a Window).
    search_input: Option<Entity<InputState>>,
    /// Kit rename input state, created lazily on first render (needs a Window).
    rename_input: Option<Entity<InputState>>,
    /// Subscription keeping `search` in sync with the kit search input.
    _search_subscription: Option<Subscription>,
    /// Subscription committing/cancelling the inline rename.
    _rename_subscription: Option<Subscription>,
    /// Thread id currently being renamed inline, if any.
    renaming: Option<String>,
    /// Title to pre-fill the rename input with, consumed by the next render.
    rename_seed: Option<String>,
    /// Thread id awaiting delete confirmation, if any.
    confirm_delete: Option<String>,
}

impl SessionsTab {
    pub fn new(coordinator: WeakEntity<WorkspaceView>) -> Self {
        // T280/T283: the canonical active project comes from
        // `ProjectsConfig` (the sole backend owner). Delegate to the
        // testable core; `None` → honest empty scope, no store read.
        Self::with_active_project(coordinator, crate::project_switcher::cached().active)
    }

    /// T283 — construct with an explicit active project scope. `None`
    /// yields an honest empty scope (no project path, no selection, no
    /// threads) and never touches the store — a no-project tab must NOT
    /// fall back to the whole-store unscoped `list()` (that leaked every
    /// project's threads onto the screen). The runtime entry point reads
    /// the process-global `ProjectsConfig`; this core is separate so the
    /// no-project contract is unit-testable without the global cache or
    /// the user's on-disk store.
    fn with_active_project(coordinator: WeakEntity<WorkspaceView>, active: Option<String>) -> Self {
        let mut tab = Self {
            threads: Vec::new(),
            search: String::new(),
            selected: None,
            project_path: None,
            coordinator,
            store: None,
            show_archived: false,
            search_input: None,
            rename_input: None,
            _search_subscription: None,
            _rename_subscription: None,
            renaming: None,
            rename_seed: None,
            confirm_delete: None,
        };
        if let Some(active) = active {
            tab.project_path = Some(PathBuf::from(&active));
            if let Ok(store) = ThreadStore::open_default() {
                if let Ok(records) = store.list_for_project(&active, false) {
                    tab.threads = records
                        .into_iter()
                        .map(|record| ThreadListItem { record, active: false })
                        .collect::<Vec<_>>();
                    Self::sort(&mut tab.threads);
                }
                tab.store = Some(store);
            }
        }
        tab
    }

    fn sort(threads: &mut [ThreadListItem]) {
        threads.sort_by(|a, b| match (a.record.pinned, b.record.pinned) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.record.updated_at.cmp(&a.record.updated_at),
        });
    }

    /// T280 — set the active project scope and reload the list via
    /// `list_for_project`. Old-project threads are dropped, not merely
    /// hidden; a cleared selection keeps an old highlight from persisting.
    pub fn set_project(&mut self, project_path: PathBuf, cx: &mut Context<Self>) {
        self.project_path = Some(project_path);
        self.selected = None;
        self.search.clear();
        self.renaming = None;
        self.rename_seed = None;
        self.confirm_delete = None;
        if let Ok(store) = ThreadStore::open_default() {
            self.store = Some(store);
        }
        self.reload(cx);
    }

    /// Re-read the scoped list from the store and re-sort. No-op without a
    /// project scope. Used after every store mutation so pin/archive/rename/
    /// delete reflect immediately and survive a restart.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project_path.clone() else {
            return;
        };
        let path = project.to_string_lossy().to_string();
        if let Some(store) = &self.store {
            if let Ok(records) = store.list_for_project(&path, self.show_archived) {
                self.threads = records
                    .into_iter()
                    .map(|record| ThreadListItem { record, active: false })
                    .collect::<Vec<_>>();
                Self::sort(&mut self.threads);
            }
        }
        cx.notify();
    }

    /// Currently selected thread id (written on click; the SoT keeps the
    /// coordinator-side copy).
    pub fn selected_thread(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// T283 — project removal clears the whole scope: project path,
    /// selection, and the loaded list. The removed project's threads must
    /// not stay on screen. Reload for the *next* project happens in
    /// `set_project` (Select/Add); no store read here.
    pub fn clear_for_project(&mut self, cx: &mut Context<Self>) {
        self.empty_scope();
        cx.notify();
    }

    /// T283 — honest empty scope, shared by the no-project constructor
    /// path and `clear_for_project` (project removal). Resets the project
    /// scope, the selection, and drops every loaded thread, plus any
    /// transient row-action state.
    fn empty_scope(&mut self) {
        self.project_path = None;
        self.selected = None;
        self.threads.clear();
        self.search.clear();
        self.renaming = None;
        self.rename_seed = None;
        self.confirm_delete = None;
        self.show_archived = false;
    }

    /// Visible threads after applying the search filter. The filter source
    /// is `short_title()` (NOT `display_title()`) — T287-B keeps this
    /// contract on the kit migration.
    fn visible(&self) -> Vec<&ThreadListItem> {
        if self.search.trim().is_empty() {
            self.threads.iter().collect()
        } else {
            let q = self.search.to_lowercase();
            self.threads
                .iter()
                .filter(|t| t.short_title().to_lowercase().contains(&q))
                .collect()
        }
    }

    /// T287-B: flip the archived filter and reload (both `list` and
    /// `list_for_project` already accept `include_archived`).
    fn toggle_show_archived(&mut self, cx: &mut Context<Self>) {
        self.show_archived = !self.show_archived;
        self.reload(cx);
    }

    fn set_pinned(&mut self, id: &str, pinned: bool, cx: &mut Context<Self>) {
        if let Some(store) = &self.store {
            if let Err(e) = store.set_pinned(id, pinned) {
                tracing::warn!("sessions: set_pinned({id}) failed: {e}");
            }
        }
        self.reload(cx);
    }

    fn set_archived(&mut self, id: &str, archived: bool, cx: &mut Context<Self>) {
        if let Some(store) = &self.store {
            if let Err(e) = store.set_archived(id, archived) {
                tracing::warn!("sessions: set_archived({id}) failed: {e}");
            }
        }
        self.reload(cx);
    }

    fn begin_rename(&mut self, id: &str, cx: &mut Context<Self>) {
        let title = self
            .threads
            .iter()
            .find(|t| t.record.id == id)
            .map(|t| t.display_title().to_string())
            .unwrap_or_default();
        self.renaming = Some(id.to_string());
        self.rename_seed = Some(title);
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.renaming.clone() else {
            return;
        };
        let value = self
            .rename_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            if let Some(store) = &self.store {
                // Persist both title and title_override so `display_title`
                // (override wins) shows the rename after a restart too.
                if let Err(e) = store.update(&id, None, Some(trimmed), Some(trimmed), None) {
                    tracing::warn!("sessions: rename({id}) failed: {e}");
                }
            }
        }
        self.renaming = None;
        self.rename_seed = None;
        self.reload(cx);
    }

    fn request_delete(&mut self, id: &str, cx: &mut Context<Self>) {
        self.confirm_delete = Some(id.to_string());
        cx.notify();
    }

    fn do_delete(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.confirm_delete.clone() else {
            return;
        };
        if let Some(store) = &self.store {
            if let Err(e) = store.delete(&id) {
                tracing::warn!("sessions: delete({id}) failed: {e}");
            }
        }
        if self.selected.as_deref() == Some(id.as_str()) {
            self.selected = None;
        }
        self.confirm_delete = None;
        self.reload(cx);
    }

    fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.confirm_delete = None;
        cx.notify();
    }

    fn emit(&self, event: SessionsEvent, cx: &mut Context<Self>) {
        if let Some(view) = self.coordinator.upgrade() {
            view.update(cx, |view, cx| view.on_sessions_event(event, cx));
        }
    }
}

impl Render for SessionsTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);

        // Kit search input — created on first render (a `Window` is required
        // by `InputState::new`, and `SessionsTab::new` runs without one).
        if self.search_input.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search sessions…"));
            let sub = cx.subscribe_in(&input, window, |this, input, event, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let q = input.read(cx).value().to_string();
                    if this.search != q {
                        this.search = q;
                        cx.notify();
                    }
                }
            });
            self.search_input = Some(input);
            self._search_subscription = Some(sub);
        }

        // Kit rename input — same lazy-create rationale. Enter commits;
        // blur also commits (clicking away is an implicit confirm).
        if self.rename_input.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx));
            let sub = cx.subscribe_in(&input, window, |this, _input, event, _window, cx| match event {
                InputEvent::PressEnter {
                    secondary: false, ..
                } => this.commit_rename(cx),
                InputEvent::Blur => {
                    if this.renaming.is_some() {
                        this.commit_rename(cx);
                    }
                }
                _ => {}
            });
            self.rename_input = Some(input);
            self._rename_subscription = Some(sub);
        }

        // Seed the rename input once when entering rename mode (the menu
        // handler runs in `App` context with no `Window`, so the value/focus
        // are deferred to this render).
        if let Some(seed) = self.rename_seed.take() {
            if let Some(input) = &self.rename_input {
                input.update(cx, |s, cx| {
                    s.set_value(seed, window, cx);
                    s.focus(window, cx);
                });
            }
        }

        let visible_count = self.visible().len();
        let item_sizes = Rc::new(vec![Size::new(px(0.), px(ROW_HEIGHT)); visible_count]);
        let list = v_virtual_list(
            cx.entity(),
            "sessions-virtual-list",
            item_sizes,
            move |this, range, _window, cx| {
                let theme = *Theme::global(cx);
                let items = this.visible();
                range
                    .filter_map(|i| {
                        let item = items.get(i)?;
                        let id = item.record.id.clone();
                        let is_selected = this.selected.as_deref() == Some(id.as_str());
                        let is_renaming = this.renaming.as_deref() == Some(id.as_str());
                        let rename_entity = this.rename_input.clone();
                        let snapshot = ThreadMenuSnapshot {
                            id: id.clone(),
                            pinned: item.record.pinned,
                            archived: item.record.archived,
                        };

                        let select_listener = {
                            let id = id.clone();
                            cx.listener(move |this, _e: &ClickEvent, _w, cx| {
                                this.selected = Some(id.clone());
                                this.emit(SessionsEvent::SelectThread(id.clone()), cx);
                                cx.notify();
                            })
                        };
                        let menu_listener = {
                            let snapshot = snapshot.clone();
                            cx.listener(move |_this, event: &MouseDownEvent, window, cx| {
                                let weak = cx.weak_entity();
                                open_row_menu(
                                    cx,
                                    window.window_handle(),
                                    Bounds::new(
                                        event.position,
                                        Size::new(px(1.), px(1.)),
                                    ),
                                    snapshot.clone(),
                                    weak,
                                );
                            })
                        };
                        let dots_listener = {
                            let snapshot = snapshot.clone();
                            cx.listener(move |_this, event: &ClickEvent, window, cx| {
                                cx.stop_propagation();
                                let weak = cx.weak_entity();
                                open_row_menu(
                                    cx,
                                    window.window_handle(),
                                    Bounds::new(event.position(), Size::new(px(1.), px(1.))),
                                    snapshot.clone(),
                                    weak,
                                );
                            })
                        };

                        let left = if is_renaming {
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .child(
                                    Input::new(rename_entity.as_ref().expect("rename input created"))
                                        .appearance(false)
                                        .text_color(theme.text.primary)
                                        .text_size(px(13.)),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .min_w(px(0.))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.accent.primary)
                                        .child(if item.record.pinned { "📌" } else { "" }),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.text.primary)
                                        .truncate()
                                        .child(item.short_title()),
                                )
                                .into_any_element()
                        };

                        let right = div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .flex_none()
                            .child(
                                div()
                                    .text_size(theme.font_sizes.xs)
                                    .text_color(theme.text.muted)
                                    .child(format_timestamp(&item.record.updated_at)),
                            )
                            .child(
                                div()
                                    .id(("sessions-dots", i))
                                    .text_size(px(13.))
                                    .text_color(theme.text.muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.text.primary))
                                    .child("⋯")
                                    .on_click(dots_listener),
                            );

                        Some(
                            div()
                                .id(("sessions-row", i))
                                .w_full()
                                .h(px(ROW_HEIGHT))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .px(px(12.))
                                .cursor_pointer()
                                .when(is_selected, |el| el.bg(theme.interactive.active))
                                .hover(|s| s.bg(theme.interactive.hover))
                                .when(!is_renaming, |el| {
                                    el.on_click(select_listener)
                                        .on_mouse_down(MouseButton::Right, menu_listener)
                                })
                                .child(left)
                                .child(right),
                        )
                    })
                    .collect::<Vec<_>>()
            },
        );

        // Delete confirmation modal — rendered inline over the list when a
        // thread is awaiting confirmation (the kit `PopupMenu` is the action
        // entry point; this is the confirm surface for the destructive step).
        let confirm_modal: Option<gpui::AnyElement> = self.confirm_delete.clone().map(|id| {
            let title = self
                .threads
                .iter()
                .find(|t| t.record.id == id)
                .map(|t| t.display_title().to_string())
                .unwrap_or_default();
            let cancel = cx.listener(|this, _e: &ClickEvent, _w, cx| this.cancel_delete(cx));
            let confirm = cx.listener(|this, _e: &ClickEvent, _w, cx| this.do_delete(cx));
            div()
                .id("sessions-delete-confirm")
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .left_0()
                .bg(theme.bg.primary.opacity(0.7))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(300.))
                        .rounded(px(10.))
                        .bg(theme.bg.secondary)
                        .border_1()
                        .border_color(theme.border.subtle)
                        .px(px(14.))
                        .py(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text.primary)
                                .child("Delete thread?"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.text.secondary)
                                .child(format!("“{title}” will be permanently deleted.")),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .id("sessions-delete-cancel")
                                        .px(px(10.))
                                        .py(px(4.))
                                        .rounded(px(6.))
                                        .text_size(px(12.))
                                        .text_color(theme.text.secondary)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.border.subtle))
                                        .child("Cancel")
                                        .on_click(cancel),
                                )
                                .child(
                                    div()
                                        .id("sessions-delete-confirm-btn")
                                        .px(px(10.))
                                        .py(px(4.))
                                        .rounded(px(6.))
                                        .text_size(px(12.))
                                        .text_color(theme.text.primary)
                                        .bg(theme.status.error)
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.status.error))
                                        .child("Delete")
                                        .on_click(confirm),
                                ),
                        ),
                )
                .into_any_element()
        });

        let show_archived = self.show_archived;
        let search_input = self.search_input.clone();

        let list_or_empty: gpui::AnyElement = if visible_count == 0 {
            div()
                .id("sessions-empty")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(theme.text.muted)
                .child("No sessions")
                .into_any_element()
        } else {
            list.into_any_element()
        };

        div()
            .id("left-sessions-tab")
            .window_font(&theme)
            .size_full()
            .relative()
            .flex()
            .flex_col()
            // T266: the sessions tab's plate follows surface alpha.
            .bg(theme.surface_color(theme.bg.primary))
            // Header: title + new-thread button.
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text.primary)
                            .child("Sessions"),
                    )
                    .child(
                        div()
                            .id("sessions-new")
                            .text_size(px(13.))
                            .text_color(theme.text.secondary)
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.text.primary))
                            .child("+ New")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.emit(SessionsEvent::CreateThread, cx);
                            })),
                    ),
            )
            // Search input.
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .child(
                        div()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border.subtle)
                            .bg(theme.bg.secondary)
                            .px(px(8.))
                            .py(px(4.))
                            .child(
                                Input::new(search_input.as_ref().expect("search input created"))
                                    .appearance(false)
                                    .text_color(theme.text.primary)
                                    .text_size(px(13.)),
                            ),
                    ),
            )
            // Show-archived toggle.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(12.))
                    .pb(px(6.))
                    .child(
                        div()
                            .id("sessions-show-archived")
                            .w(px(13.))
                            .h(px(13.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(theme.border.subtle)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(10.))
                            .text_color(theme.text.primary)
                            .when(show_archived, |el| el.bg(theme.accent.primary))
                            .child(show_archived.then(|| "✓").unwrap_or(""))
                            .on_click(cx.listener(|this, _e: &ClickEvent, _w, cx| {
                                this.toggle_show_archived(cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.text.secondary)
                            .child("Show archived"),
                    ),
            )
            // Thread list (or empty state).
            .child(list_or_empty)
            // Delete confirmation overlay.
            .children(confirm_modal)
    }
}

// ── Row context menu (PopupMenu in an anchored popup window) ────────────
// Same engine as dock/tray/launcher menus: a `gpui_component::Root`-wrapped
// anchored popup hosting `PopupMenu`, `grab: false` (T264), and an Overlay
// click-catcher with a screen-space hole so click-away still dismisses.

struct SessionsRowMenuState {
    handle: Option<WindowHandle<Root>>,
    view: Option<WeakEntity<SessionsRowMenuView>>,
    snapshot: Option<ThreadMenuSnapshot>,
    tab: Option<WeakEntity<SessionsTab>>,
    close_generation: u64,
    window_closed_subscription: Option<Subscription>,
    click_catcher: Option<AnyWindowHandle>,
}

impl Default for SessionsRowMenuState {
    fn default() -> Self {
        Self {
            handle: None,
            view: None,
            snapshot: None,
            tab: None,
            close_generation: 0,
            window_closed_subscription: None,
            click_catcher: None,
        }
    }
}

impl Global for SessionsRowMenuState {}

struct SessionsRowMenuView {
    popup_menu: Option<Entity<PopupMenu>>,
    dismiss_subscription: Option<Subscription>,
}

impl SessionsRowMenuView {
    fn new(cx: &mut Context<Self>) -> Self {
        let _ = cx;
        Self {
            popup_menu: None,
            dismiss_subscription: None,
        }
    }
}

impl Render for SessionsRowMenuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.popup_menu.is_none() {
            let snapshot = cx
                .global::<SessionsRowMenuState>()
                .snapshot
                .clone()
                .unwrap_or(ThreadMenuSnapshot {
                    id: String::new(),
                    pinned: false,
                    archived: false,
                });
            let tab = cx.global::<SessionsRowMenuState>().tab.clone();
            let menu = build_row_menu(window, cx, snapshot, tab);
            self.dismiss_subscription = Some(cx.subscribe_in(
                &menu,
                window,
                |_this, _menu, _: &DismissEvent, window, cx| {
                    close_this(window, cx);
                },
            ));
            let handle = menu.read(cx).focus_handle(cx);
            handle.focus(window, cx);
            self.popup_menu = Some(menu);
        }
        let Some(menu) = self.popup_menu.clone() else {
            return div().into_any_element();
        };
        let theme = *Theme::global(cx);
        div()
            .size_full()
            .window_font(&theme)
            .child(menu.clone())
            .into_any_element()
    }
}

fn build_row_menu(
    window: &mut Window,
    cx: &mut App,
    snapshot: ThreadMenuSnapshot,
    tab: Option<WeakEntity<SessionsTab>>,
) -> Entity<PopupMenu> {
    PopupMenu::build(window, cx, |menu, _window, _cx| {
        let mut menu = menu.min_w(px(MENU_WIDTH)).max_w(px(MENU_WIDTH));

        {
            let snap = snapshot.clone();
            let tab = tab.clone();
            menu = menu.item(
                PopupMenuItem::new(if snap.pinned { "Unpin" } else { "Pin" }).on_click(
                    move |_e, _w, cx: &mut App| {
                        if let Some(t) = tab.as_ref().and_then(|w| w.upgrade()) {
                            t.update(cx, |s, cx| s.set_pinned(&snap.id, !snap.pinned, cx));
                        }
                    },
                ),
            );
        }
        {
            let snap = snapshot.clone();
            let tab = tab.clone();
            menu = menu.item(
                PopupMenuItem::new(if snap.archived { "Unarchive" } else { "Archive" }).on_click(
                    move |_e, _w, cx: &mut App| {
                        if let Some(t) = tab.as_ref().and_then(|w| w.upgrade()) {
                            t.update(cx, |s, cx| s.set_archived(&snap.id, !snap.archived, cx));
                        }
                    },
                ),
            );
        }
        {
            let snap = snapshot.clone();
            let tab = tab.clone();
            menu = menu.item(PopupMenuItem::new("Rename").on_click(move |_e, _w, cx: &mut App| {
                if let Some(t) = tab.as_ref().and_then(|w| w.upgrade()) {
                    t.update(cx, |s, cx| s.begin_rename(&snap.id, cx));
                }
            }));
        }
        {
            let snap = snapshot.clone();
            let tab = tab.clone();
            menu = menu.item(PopupMenuItem::new("Delete").on_click(move |_e, _w, cx: &mut App| {
                if let Some(t) = tab.as_ref().and_then(|w| w.upgrade()) {
                    t.update(cx, |s, cx| s.request_delete(&snap.id, cx));
                }
            }));
        }
        menu
    })
}

fn pick_display(cx: &App) -> Option<DisplayId> {
    crate::monitor::pult_display_id_or_primary(cx)
}

/// Convert a window-local right-click position into an output-local 1×1
/// anchor for the click-catcher's pass-through hole. The left content
/// surface is layer-shell anchored with margins (`left = RAIL_WIDTH`, `top =
/// bar gap`), so its client-side `window.bounds().origin` is `(0,0)` — the
/// real screen position must come from a live compositor query (the
/// `162798b4` lesson), exactly like the launcher's `catcher_anchor_for`.
fn catcher_anchor_for(cx: &App, local: Point<Pixels>) -> Bounds<Pixels> {
    let origin = content_output_local_origin(cx).unwrap_or_default();
    Bounds::new(origin + local, Size::new(px(1.), px(1.)))
}

fn content_output_local_origin(cx: &App) -> Option<Point<Pixels>> {
    let display = crate::monitor::pult_display_info(cx)?;
    let (wx, wy) =
        chronos_services::compositor::hyprland::window_position("chronos-side-panel-left-content")?;
    let display_origin = display.bounds().origin;
    Some(point(
        px(wx as f32) - display_origin.x,
        px(wy as f32) - display_origin.y,
    ))
}

fn open_click_catcher(
    cx: &mut App,
    anchor_rect: Bounds<Pixels>,
) -> anyhow::Result<AnyWindowHandle> {
    crate::popup_click_catcher::open_for_popup(
        cx,
        anchor_rect,
        Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        Rc::new(|window, cx| close_from_click_catcher(window, cx)),
    )
}

fn fallback_window_options(display_id: Option<DisplayId>) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        })),
        app_id: Some("chronos-sessions-row-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "sessions-row-menu".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP,
            exclusive_zone: None,
            margin: Some((px(36.), px(0.), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn window_options(anchor_rect: Bounds<Pixels>, parent: AnyWindowHandle) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(MENU_WIDTH), px(MENU_HEIGHT)),
        })),
        app_id: Some("chronos-sessions-row-menu".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::AnchoredPopup(PopupOptions {
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomLeft,
            gravity: PopupGravity::BottomRight,
            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::SLIDE_Y
                | PopupConstraintAdjustment::FLIP_X
                | PopupConstraintAdjustment::FLIP_Y,
            offset: point(px(0.), px(4.)),
            // T264: no compositor grab — the component dismisses on click-away.
            grab: false,
        }),
        ..Default::default()
    }
}

/// Open the row context menu for `snapshot`, anchored at `anchor_rect`
/// (parent-surface local) under `parent`. The `SessionsTab` weak handle is
/// carried so menu actions can reach the tab from `App` context.
fn open_row_menu(
    cx: &mut App,
    parent: AnyWindowHandle,
    anchor_rect: Bounds<Pixels>,
    snapshot: ThreadMenuSnapshot,
    tab: WeakEntity<SessionsTab>,
) {
    if !cx.has_global::<SessionsRowMenuState>() {
        cx.set_global(SessionsRowMenuState::default());
    }

    // Rebuild from scratch each time: the snapshot changes per row, and
    // `PopupMenu` has no set-items API.
    close(cx);

    {
        let state = cx.global_mut::<SessionsRowMenuState>();
        state.snapshot = Some(snapshot);
        state.tab = Some(tab);
        state.close_generation = state.close_generation.wrapping_add(1);
    }

    let click_catcher = open_click_catcher(cx, catcher_anchor_for(cx, anchor_rect.origin)).ok();
    let mut opened_view: Option<WeakEntity<SessionsRowMenuView>> = None;
    let mut open = |cx: &mut App, options: WindowOptions| {
        cx.open_window(options, |window, view_cx| {
            let view = view_cx.new(|view_cx| SessionsRowMenuView::new(view_cx));
            opened_view = Some(view.downgrade());
            view_cx.new(|view_cx| {
                Root::new(view, window, view_cx)
                    .bordered(false)
                    .bg(gpui::transparent_black())
            })
        })
    };
    let result = match open(cx, window_options(anchor_rect, parent)) {
        Err(err) => {
            if let Some(catcher) = click_catcher {
                let _ = catcher.update(cx, |_, window, _| window.remove_window());
            }
            if err.downcast_ref::<PopupNotSupportedError>().is_some() {
                tracing::warn!(
                    "sessions row menu: AnchoredPopup not supported, falling back to layer-shell"
                );
                let display_id = pick_display(cx);
                open(cx, fallback_window_options(display_id))
            } else {
                Err(err)
            }
        }
        ok => ok,
    };
    match result {
        Ok(new_handle) => {
            let window_id = new_handle.window_id();
            let window_closed_subscription = cx.on_window_closed(move |cx, closed_id| {
                let tracked = cx
                    .global::<SessionsRowMenuState>()
                    .handle
                    .as_ref()
                    .map(|handle| handle.window_id());
                if closed_id != window_id || tracked != Some(closed_id) {
                    return;
                }
                let catcher = {
                    let state = cx.global_mut::<SessionsRowMenuState>();
                    state.handle = None;
                    state.view = None;
                    state.snapshot = None;
                    state.tab = None;
                    state.close_generation = state.close_generation.wrapping_add(1);
                    state.window_closed_subscription = None;
                    state.click_catcher.take()
                };
                if let Some(catcher) = catcher {
                    let _ = catcher.update(cx, |_, window, _| window.remove_window());
                }
            });
            let state = cx.global_mut::<SessionsRowMenuState>();
            state.handle = Some(new_handle);
            state.view = opened_view;
            state.window_closed_subscription = Some(window_closed_subscription);
            state.click_catcher = click_catcher;
        }
        Err(err) => tracing::warn!("sessions row menu: failed to open: {err}"),
    }
}

/// Close from inside a callback that already holds `&mut Window` for this
/// popup (the `DismissEvent` subscription). Must not re-enter `handle.update`
/// on the same id — direct `remove_window` on the live reference.
fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<SessionsRowMenuState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    let catcher = {
        let state = cx.global_mut::<SessionsRowMenuState>();
        if tracked {
            state.handle.take();
            state.view = None;
            state.window_closed_subscription = None;
        }
        state.click_catcher.take()
    };
    {
        let state = cx.global_mut::<SessionsRowMenuState>();
        state.snapshot = None;
        state.tab = None;
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
    }
    if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
    window.remove_window();
}

/// Close both surfaces from the transparent click-catcher's own callback.
fn close_from_click_catcher(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let (popup, catcher) = {
        let state = cx.global_mut::<SessionsRowMenuState>();
        state.snapshot = None;
        state.tab = None;
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
        state.view = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(popup) = popup {
        let _ = popup.update(cx, |_, popup_window, _| popup_window.remove_window());
    }
    if catcher == Some(this) {
        window.remove_window();
    } else if let Some(catcher) = catcher {
        let _ = catcher.update(cx, |_, catcher_window, _| catcher_window.remove_window());
    }
}

/// Close both surfaces (clears state + destroys window).
fn close(cx: &mut App) {
    let (popup, catcher) = {
        let state = cx.global_mut::<SessionsRowMenuState>();
        state.snapshot = None;
        state.tab = None;
        state.close_generation = state.close_generation.wrapping_add(1);
        state.window_closed_subscription = None;
        state.view = None;
        (state.handle.take(), state.click_catcher.take())
    };
    if let Some(handle) = popup {
        let _ = handle.update(cx, |_, window: &mut gpui::Window, _| window.remove_window());
    }
    if let Some(handle) = catcher {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::threads::ThreadRecord;

    /// Sorting is pinned-first, then updated_at desc — the same policy
    /// `ChatTab::new` applies. This guards against an accidental reorder.
    #[test]
    fn sort_pins_first_then_recency() {
        let mut items = vec![
            ThreadListItem {
                record: ThreadRecord {
                    id: "a".into(),
                    pinned: false,
                    updated_at: "2026-01-03T00:00:00Z".into(),
                    ..record_fixture()
                },
                active: false,
            },
            ThreadListItem {
                record: ThreadRecord {
                    id: "b".into(),
                    pinned: true,
                    updated_at: "2026-01-01T00:00:00Z".into(),
                    ..record_fixture()
                },
                active: false,
            },
            ThreadListItem {
                record: ThreadRecord {
                    id: "c".into(),
                    pinned: false,
                    updated_at: "2026-01-02T00:00:00Z".into(),
                    ..record_fixture()
                },
                active: false,
            },
        ];
        SessionsTab::sort(&mut items);
        assert_eq!(items[0].record.id, "b", "pinned first");
        // Recency desc among non-pinned: a (2026-01-03) is newer than c (2026-01-02).
        assert_eq!(items[1].record.id, "a", "then recency desc — newest non-pinned");
        assert_eq!(items[2].record.id, "c", "then older non-pinned");
    }

    /// Source contract (T287-B): the `selected` field must be WRITTEN on row
    /// click and READ in render for the highlight — a write-only field was
    /// the round-1 reject ("highlight is a lie").
    #[test]
    fn selected_field_is_written_on_click_and_read_in_render() {
        let src = include_str!("sessions.rs");
        assert!(
            src.contains("this.selected = Some(id.clone())"),
            "row click must write `selected`"
        );
        assert!(
            src.contains("this.selected.as_deref() == Some(id.as_str())"),
            "render must read `selected` for the row highlight"
        );
        assert!(
            src.contains(".when(is_selected, |el| el.bg(theme.interactive.active))"),
            "render must paint the selected row background"
        );
    }

    /// T287-B: the search filter must keep using `short_title()` (the
    /// 30-char truncated title), NOT the full `display_title()`. A token in
    /// the truncated tail is therefore not searchable.
    #[test]
    fn search_filters_short_title_not_full_display_title() {
        let mut tab = SessionsTab::with_active_project(WeakEntity::<WorkspaceView>::new_invalid(), None);
        let long_title = format!("{}NEEDLE", "a".repeat(35));
        tab.threads = vec![ThreadListItem {
            record: ThreadRecord {
                id: "x".into(),
                title: long_title,
                ..record_fixture()
            },
            active: false,
        }];
        tab.search = "NEEDLE".to_string();
        assert!(
            tab.visible().is_empty(),
            "filter must use short_title, so the truncated tail is not searchable"
        );
    }

    /// T283 — removing the active project must reset the whole Sessions
    /// scope: project path, selection, AND the loaded list. The old
    /// project's threads must not stay on screen (the pre-T283 code only
    /// cleared the highlight while the list kept painting). Drives the
    /// real prod removal path `clear_for_project` on a live entity.
    #[gpui::test]
    fn clear_for_project_resets_scope(cx: &mut gpui::TestAppContext) {
        let coord = WeakEntity::<WorkspaceView>::new_invalid();
        let tab = cx.new(|_| SessionsTab {
            threads: vec![ThreadListItem {
                record: ThreadRecord {
                    id: "t1".into(),
                    ..record_fixture()
                },
                active: false,
            }],
            search: String::new(),
            selected: Some("t1".into()),
            project_path: Some(PathBuf::from("/proj")),
            coordinator: coord,
            store: None,
            show_archived: false,
            search_input: None,
            rename_input: None,
            _search_subscription: None,
            _rename_subscription: None,
            renaming: None,
            rename_seed: None,
            confirm_delete: None,
        });
        tab.update(cx, |tab, cx| tab.clear_for_project(cx));
        let (threads, selection, project) = tab.read_with(cx, |tab, _| {
            (
                tab.threads.is_empty(),
                tab.selected_thread().is_none(),
                tab.project_path.is_none(),
            )
        });
        assert!(threads, "removed project's threads must not stay on screen");
        assert!(selection, "selection must clear");
        assert!(project, "project scope must clear");
    }

    /// T283 — no active project → honest empty scope: 0 rows, no project
    /// path. The pre-T283 constructor fell back to the unscoped
    /// `list(None, ..)` and painted every project's threads. Uses the
    /// explicit-scope core (the `new` entry point delegates to it) so the
    /// test is deterministic — no process-global config cache, no user's
    /// on-disk store.
    #[test]
    fn new_without_project_loads_empty_scope() {
        let tab =
            SessionsTab::with_active_project(WeakEntity::<WorkspaceView>::new_invalid(), None);
        assert!(tab.threads.is_empty(), "no project → no rows");
        assert_eq!(tab.selected_thread(), None, "no project → no selection");
        assert!(tab.project_path.is_none(), "no project → no scope");
    }

    /// T283 — the unscoped whole-store `list(None, ...)` must never
    /// reappear in Sessions, and `new` must keep delegating to the
    /// explicit-scope core (so the no-project contract above is the path
    /// prod actually runs).
    #[test]
    fn no_unscoped_list_in_sessions() {
        let src = include_str!("sessions.rs");
        let needle = "list(None".to_owned() + ", false, false)";
        assert!(
            !src.contains(&needle),
            "Sessions must never list the whole store unscoped"
        );
        assert!(
            src.contains(
                "Self::with_active_project(coordinator, crate::project_switcher::cached().active)"
            ),
            "new must delegate to the explicit-scope core"
        );
    }

    /// T287-B — pin/archive/delete write through `ThreadStore` and the list
    /// re-sorts/hides/reloads accordingly, exercising the real prod paths.
    #[gpui::test]
    fn pin_archive_delete_roundtrip_through_store(cx: &mut gpui::TestAppContext) {
        let (store, _path) = temp_store();
        store.insert_for_project("a", "hermes", "/p", "/proj").unwrap();
        store.insert_for_project("b", "hermes", "/p", "/proj").unwrap();

        let tab = cx.new(|_| SessionsTab {
            threads: Vec::new(),
            search: String::new(),
            selected: None,
            project_path: Some(PathBuf::from("/proj")),
            coordinator: WeakEntity::<WorkspaceView>::new_invalid(),
            store: Some(store),
            show_archived: false,
            search_input: None,
            rename_input: None,
            _search_subscription: None,
            _rename_subscription: None,
            renaming: None,
            rename_seed: None,
            confirm_delete: None,
        });
        tab.update(cx, |tab, cx| tab.reload(cx));

        // Pin `a` → it re-sorts to the front and persists.
        tab.update(cx, |tab, cx| tab.set_pinned("a", true, cx));
        let ids = thread_ids(&tab, cx);
        assert_eq!(ids.first().map(String::as_str), Some("a"), "pinned first");
        assert!(tab.read_with(cx, |tab, _| {
            tab.threads
                .iter()
                .find(|t| t.record.id == "a")
                .map(|t| t.record.pinned)
                .unwrap_or(false)
        }));

        // Archive `b` → hidden by default.
        tab.update(cx, |tab, cx| tab.set_archived("b", true, cx));
        let ids = thread_ids(&tab, cx);
        assert!(!ids.contains(&"b".to_string()), "archived hidden by default");

        // Show archived → `b` visible again.
        tab.update(cx, |tab, cx| tab.toggle_show_archived(cx));
        let ids = thread_ids(&tab, cx);
        assert!(ids.contains(&"b".to_string()), "show archived reveals it");

        // Delete `a` via the confirm path.
        tab.update(cx, |tab, cx| {
            tab.confirm_delete = Some("a".into());
            tab.do_delete(cx);
        });
        let ids = thread_ids(&tab, cx);
        assert!(!ids.contains(&"a".to_string()), "deleted thread gone");
    }

    fn thread_ids(tab: &gpui::Entity<SessionsTab>, cx: &gpui::TestAppContext) -> Vec<String> {
        tab.read_with(cx, |tab, _| {
            tab.threads.iter().map(|t| t.record.id.clone()).collect()
        })
    }

    fn temp_store() -> (ThreadStore, PathBuf) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("chronos-sessions-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        (ThreadStore::open(&path).unwrap(), path)
    }

    fn record_fixture() -> ThreadRecord {
        ThreadRecord {
            id: String::new(),
            acp_session_id: None,
            agent_id: "test".into(),
            cwd: "/tmp".into(),
            project_path: "/tmp".into(),
            title: String::new(),
            title_override: None,
            last_model: None,
            pinned: false,
            archived: false,
            created_at: String::new(),
            updated_at: String::new(),
            transcript_json: None,
        }
    }
}
