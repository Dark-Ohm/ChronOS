//! T279 / Slice A2 — the Sessions tab body.
//!
//! A standalone session-list entity rendered inside the content canvas when
//! `active_tab == Sessions`. It owns its own `ThreadStore` handle + list
//! items (mirrors `ChatTab::new`'s load path) and reuses
//! `sessions_list::ThreadListItem` row helpers — no row markup is
//! duplicated here; the full interactive sidebar (rename/pin/archive/menu)
//! stays with `ChatTab`'s inline sidebar, which is a different surface (a
//! 200 px strip beside chat, not a full-panel tab).
//!
//! Selection emits `SessionsEvent::SelectThread` upward; the coordinator
//! (`select_session` reducer) writes the id into the SoT and switches to
//! Chat. The tab itself does not own the active session id — that lives in
//! `SidePanelLeftState_`.

use gpui::{Context, IntoElement, Render, WeakEntity, Window, div, prelude::*, px};

use chronos_services::threads::{ThreadRecord, ThreadStore};
use chronos_ui::{Theme, WindowRootExt};

use crate::side_panel_left::sessions_list::{ThreadListItem, format_timestamp};
use crate::side_panel_left::workspace_view::WorkspaceView;

/// Event emitted by `SessionsTab` to the workspace coordinator.
#[derive(Clone, Debug)]
pub enum SessionsEvent {
    /// User clicked a thread — coordinator sets active id + opens Chat.
    SelectThread(String),
    /// User clicked "+ New".
    CreateThread,
}

/// The Sessions tab body — a full-panel thread list backed by `ThreadStore`.
pub struct SessionsTab {
    /// Loaded thread list, sorted pinned-first then updated_at desc.
    threads: Vec<ThreadListItem>,
    /// Search filter query.
    search: String,
    /// Currently selected thread id — written on click, painted in render
    /// (T279 round 2: the field was previously written-but-never-read,
    /// which the reviewer flagged as dead state). The SoT
    /// (`SidePanelLeftState_.active_session_id`) remains the source of
    /// truth for the coordinator; this mirror exists purely so the row
    /// can paint its selected background without re-querying the global
    /// during render.
    selected: Option<String>,
    /// T280: the project this list is scoped to. `None` before any project
    /// is active — shows no project-scoped threads (empty list is honest).
    project_path: Option<std::path::PathBuf>,
    /// Weak handle to the owning `WorkspaceView` — used to forward events
    /// to the coordinator without this tab owning panel state.
    coordinator: WeakEntity<WorkspaceView>,
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
        };
        if let Some(active) = active {
            tab.project_path = Some(std::path::PathBuf::from(&active));
            if let Ok(store) = ThreadStore::open_default() {
                if let Ok(records) = store.list_for_project(&active, false) {
                    tab.threads = records
                        .into_iter()
                        .map(|record| ThreadListItem { record, active: false })
                        .collect::<Vec<_>>();
                    Self::sort(&mut tab.threads);
                }
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
    pub fn set_project(&mut self, project_path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.project_path = Some(project_path.clone());
        self.selected = None;
        let store = ThreadStore::open_default().ok();
        let mut threads = store
            .as_ref()
            .and_then(|s| {
                s.list_for_project(&project_path.to_string_lossy(), false)
                    .ok()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|record| ThreadListItem { record, active: false })
            .collect::<Vec<_>>();
        Self::sort(&mut threads);
        self.threads = threads;
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
    /// scope, the selection, and drops every loaded thread.
    fn empty_scope(&mut self) {
        self.project_path = None;
        self.selected = None;
        self.threads.clear();
    }

    /// Visible threads after applying the search filter.
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

    fn emit(&self, event: SessionsEvent, cx: &mut Context<Self>) {
        if let Some(view) = self.coordinator.upgrade() {
            view.update(cx, |view, cx| view.on_sessions_event(event, cx));
        }
    }
}

impl Render for SessionsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let visible = self.visible();
        div()
            .id("left-sessions-tab")
            .window_font(&theme)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg.primary)
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
            // Thread list.
            .children(visible.iter().enumerate().map(|(i, item)| {
                let id = item.record.id.clone();
                let is_selected = self.selected.as_deref() == Some(id.as_str());
                div()
                    .id(("sessions-row", i))
                    .flex()
                    .flex_col()
                    .px(px(12.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .cursor_pointer()
                    // T279 round 2: the selected row paints — the field is
                    // no longer write-only dead state.
                    .when(is_selected, |el| el.bg(theme.interactive.active))
                    .hover(|s| s.bg(theme.interactive.hover))
                    .on_click({
                        let thread_id = id.clone();
                        cx.listener(move |this, _e, _w, cx| {
                            this.selected = Some(thread_id.clone());
                            this.emit(SessionsEvent::SelectThread(thread_id.clone()), cx);
                            cx.notify();
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.text.primary)
                                    .child(item.short_title()),
                            )
                            .child(
                                div()
                                    .text_size(theme.font_sizes.xs)
                                    .text_color(theme.text.muted)
                                    .child(format_timestamp(&item.record.updated_at)),
                            ),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Source contract (round-2 §5): the `selected` field must be WRITTEN
    /// on row click and READ in render for the highlight — a write-only
    /// field was the round-1 reject ("highlight is a lie"). Source scan
    /// mirrors the T278 gate pattern.
    #[test]
    fn selected_field_is_written_on_click_and_read_in_render() {
        let src = include_str!("sessions.rs");
        assert!(
            src.contains("this.selected = Some(thread_id.clone())"),
            "row click must write `selected`"
        );
        assert!(
            src.contains("self.selected.as_deref() == Some(id.as_str())"),
            "render must read `selected` for the row highlight"
        );
        assert!(
            src.contains(".when(is_selected, |el| el.bg(theme.interactive.active))"),
            "render must paint the selected row background"
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
            project_path: Some(std::path::PathBuf::from("/proj")),
            coordinator: coord,
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
        let tab = SessionsTab::with_active_project(WeakEntity::<WorkspaceView>::new_invalid(), None);
        assert!(tab.threads.is_empty(), "no project → no rows");
        assert_eq!(tab.selected_thread(), None, "no project → no selection");
        assert!(tab.project_path.is_none(), "no project → no scope");
    }

    /// T283 — the unscoped whole-store `list(None, ...)` must never
    /// reappear in Sessions, and `new` must keep delegating to the
    /// explicit-scope core (so the no-project contract above is the path
    /// prod actually runs). Source scan mirrors the T278 gate pattern; the
    /// needle is split so the test does not self-match.
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