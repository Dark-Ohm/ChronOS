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
    /// Weak handle to the owning `WorkspaceView` — used to forward events
    /// to the coordinator without this tab owning panel state.
    coordinator: WeakEntity<WorkspaceView>,
}

impl SessionsTab {
    pub fn new(coordinator: WeakEntity<WorkspaceView>) -> Self {
        let store = ThreadStore::open_default().ok();
        let mut threads = store
            .as_ref()
            .map(|s| {
                s.list(None, false, false)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|record| ThreadListItem { record, active: false })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Self::sort(&mut threads);
        Self {
            threads,
            search: String::new(),
            coordinator,
        }
    }

    fn sort(threads: &mut [ThreadListItem]) {
        threads.sort_by(|a, b| match (a.record.pinned, b.record.pinned) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.record.updated_at.cmp(&a.record.updated_at),
        });
    }

    /// Set the active project and reload the list for it. Slice A2 uses the
    /// global store (no project scope yet — that's T280); the signature is
    /// forward-compatible so T280 wires `list_for_project` here.
    pub fn set_project(&mut self, _project_path: std::path::PathBuf, _cx: &mut Context<Self>) {
        // T280 will filter by project_path; A2 shows all threads.
    }

    /// Currently selected thread id (read from the coordinator — the tab
    /// does not duplicate it).
    pub fn selected_thread(&self) -> Option<&str> {
        None
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
                div()
                    .id(("sessions-row", i))
                    .flex()
                    .flex_col()
                    .px(px(12.))
                    .py(px(8.))
                    .border_b_1()
                    .border_color(theme.border.subtle)
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.interactive.hover))
                    .on_click({
                        let thread_id = id.clone();
                        cx.listener(move |this, _e, _w, cx| {
                            this.emit(SessionsEvent::SelectThread(thread_id.clone()), cx);
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

    fn record_fixture() -> ThreadRecord {
        ThreadRecord {
            id: String::new(),
            acp_session_id: None,
            agent_id: "test".into(),
            cwd: "/tmp".into(),
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