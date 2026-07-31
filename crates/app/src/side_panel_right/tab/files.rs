//! Files tab — read-oriented project tree (spec §4.1).
//!
//! Listing layer: `chronos_services::files` (ported from Chronos-FM).
//! UI: narrow single-column view for `preferred_content_width = 440`.

use std::path::PathBuf;

use chronos_services::files::{
    DIR_LISTING_LIMIT, FileEntryDto, ListParams, ListResult, SortKey, list_dir_sync, sort_entries,
};
use chronos_ui::Theme;
use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, svg,
};

/// Load lifecycle for honest empty / error / truncated states (§13).
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadState {
    Loading,
    Ready,
    Error(String),
}

pub struct FilesTab {
    cwd: PathBuf,
    entries: Vec<FileEntryDto>,
    /// Total names in the directory before the page limit.
    total: usize,
    /// `true` when `next_cursor` was set — listing is truncated by limit.
    truncated: bool,
    load: LoadState,
    /// Monotonic generation so stale background results are discarded.
    generation: u64,
    scroll: ScrollHandle,
}

impl FilesTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut this = Self {
            cwd,
            entries: Vec::new(),
            total: 0,
            truncated: false,
            load: LoadState::Loading,
            generation: 0,
            scroll: ScrollHandle::new(),
        };
        this.request_reload(cx);
        this
    }

    fn request_reload(&mut self, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        let load_gen = self.generation;
        let path = self.cwd.to_string_lossy().to_string();
        self.load = LoadState::Loading;
        self.entries.clear();
        self.truncated = false;
        self.total = 0;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let path_for_io = path.clone();
            let result = cx
                .background_spawn(async move {
                    list_dir_sync(ListParams {
                        path: &path_for_io,
                        limit: DIR_LISTING_LIMIT,
                        cursor: None,
                    })
                })
                .await;

            this.update(cx, |this, cx| {
                if this.generation != load_gen {
                    return;
                }
                match result {
                    Ok(list) => this.apply_list(list),
                    Err(e) => {
                        this.load = LoadState::Error(format!(
                            "Cannot read '{}': {e}",
                            this.cwd.display()
                        ));
                        this.entries.clear();
                        this.total = 0;
                        this.truncated = false;
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn apply_list(&mut self, mut list: ListResult) {
        sort_entries(&mut list.entries, SortKey::Name, true);
        self.truncated = list.next_cursor.is_some();
        self.total = list.total;
        self.entries = list.entries;
        self.load = LoadState::Ready;
    }

    fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path == self.cwd {
            return;
        }
        self.cwd = path;
        self.scroll = ScrollHandle::new();
        self.request_reload(cx);
    }

    fn go_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.cwd.parent() {
            if parent.as_os_str().is_empty() {
                return;
            }
            self.navigate_to(parent.to_path_buf(), cx);
        }
    }

    fn open_entry(&mut self, entry: &FileEntryDto, cx: &mut Context<Self>) {
        if entry.kind == "dir" {
            self.navigate_to(PathBuf::from(&entry.path), cx);
        }
        // Files: read-only v1 — no open/preview (later capability / Editor tab).
    }
}

impl Render for FilesTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let cwd_label: SharedString = self.cwd.to_string_lossy().into_owned().into();

        let path_bar = div()
            .id("files-path-bar")
            .flex()
            .flex_col()
            .gap(px(4.))
            .px(px(12.))
            .py(px(10.))
            .border_b_1()
            .border_color(theme.border.subtle)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("files-up")
                            .px(px(8.))
                            .py(px(4.))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.interactive.hover))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.go_up(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.text.primary)
                                    .child(".."),
                            ),
                    )
                    .child(
                        div()
                            .id("files-reload")
                            .px(px(8.))
                            .py(px(4.))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.interactive.hover))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.request_reload(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text.muted)
                                    .child("reload"),
                            ),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .font_family(theme.font_mono)
                    .text_color(theme.text.muted)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(cwd_label),
            );

        let status = match &self.load {
            LoadState::Loading => Some(("Loading…".to_string(), theme.text.muted)),
            LoadState::Error(msg) => Some((msg.clone(), theme.status.error)),
            LoadState::Ready if self.entries.is_empty() => {
                Some(("Directory is empty".to_string(), theme.text.muted))
            }
            LoadState::Ready => None,
        };

        let truncated_banner = if self.truncated && matches!(self.load, LoadState::Ready) {
            Some(format!(
                "Showing {} of {} entries (limit {})",
                self.entries.len(),
                self.total,
                DIR_LISTING_LIMIT
            ))
        } else {
            None
        };

        let mut list = div()
            .id("files-list")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .flex()
            .flex_col()
            .px(px(6.))
            .py(px(4.));

        if let Some((msg, color)) = status {
            list = list.child(
                div()
                    .px(px(10.))
                    .py(px(16.))
                    .text_size(px(12.))
                    .text_color(color)
                    .child(msg),
            );
        }

        if let Some(banner) = truncated_banner {
            list = list.child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .mb(px(4.))
                    .rounded_md()
                    .bg(theme.bg.elevated)
                    .text_size(px(11.))
                    .text_color(theme.text.muted)
                    .child(banner),
            );
        }

        for (ix, entry) in self.entries.iter().enumerate() {
            let entry = entry.clone();
            let is_dir = entry.kind == "dir";
            let name = entry.name.clone();
            let size_label = if is_dir {
                String::new()
            } else {
                human_bytes(entry.size)
            };

            let row = div()
                .id(SharedString::from(format!("files-row-{ix}")))
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(8.))
                .py(px(5.))
                .rounded_md()
                .cursor_pointer()
                .hover(|s| s.bg(theme.interactive.hover))
                .on_click(cx.listener({
                    let entry = entry.clone();
                    move |this, _ev, _w, cx| {
                        // Single click navigates into directories (narrow panel).
                        this.open_entry(&entry, cx);
                    }
                }))
                .child(if is_dir {
                    svg()
                        .path("icons/folder.svg")
                        .size(px(14.))
                        .text_color(theme.accent.primary)
                        .into_any_element()
                } else {
                    div()
                        .w(px(14.))
                        .h(px(14.))
                        .rounded_sm()
                        .bg(theme.bg.elevated)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_size(px(9.))
                                .text_color(theme.text.muted)
                                .child(
                                    name.chars()
                                        .next()
                                        .unwrap_or('?')
                                        .to_uppercase()
                                        .to_string(),
                                ),
                        )
                        .into_any_element()
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_size(px(12.))
                        .text_color(theme.text.primary)
                        .font_weight(if is_dir {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.text.muted)
                        .font_family(theme.font_mono)
                        .child(size_label),
                );

            list = list.child(row);
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(path_bar)
            .child(list)
    }
}

fn human_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = size as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{size}{}", UNITS[0])
    } else {
        format!("{v:.1}{}", UNITS[u])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn install_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(Theme::default());
        });
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0B");
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(2048), "2.0K");
    }

    #[gpui::test]
    fn files_tab_settles_after_background_list(cx: &mut TestAppContext) {
        install_theme(cx);
        let view = cx.new(|cx| FilesTab::new(cx));
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| {
            assert!(
                matches!(this.load, LoadState::Ready | LoadState::Error(_)),
                "after park, load must settle, got {:?}",
                this.load
            );
        });
    }

    #[gpui::test]
    fn files_tab_error_on_missing_path(cx: &mut TestAppContext) {
        install_theme(cx);
        let view = cx.new(|cx| {
            let mut t = FilesTab::new(cx);
            t.cwd = PathBuf::from("/no/such/chronos/files/tab/path");
            t.request_reload(cx);
            t
        });
        cx.background_executor.run_until_parked();
        cx.update_entity(&view, |this, _cx| match &this.load {
            LoadState::Error(msg) => {
                assert!(
                    msg.contains("Cannot read"),
                    "error must be honest, got: {msg}"
                );
            }
            other => panic!("expected Error, got {other:?}"),
        });
    }

    #[gpui::test]
    fn truncated_state_when_limit_hit(cx: &mut TestAppContext) {
        install_theme(cx);
        let dir = std::env::temp_dir().join(format!(
            "chronos-files-tab-trunc-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in 0..15 {
            std::fs::write(dir.join(format!("f{n:02}")), "x").unwrap();
        }
        let path = dir.clone();

        let view = cx.new(|cx| FilesTab::new(cx));
        cx.update_entity(&view, |this, _cx| {
            this.cwd = path;
            let list = list_dir_sync(ListParams {
                path: &this.cwd.to_string_lossy(),
                limit: 5,
                cursor: None,
            })
            .unwrap();
            assert!(list.next_cursor.is_some());
            this.apply_list(list);
            assert!(this.truncated);
            assert_eq!(this.entries.len(), 5);
            assert_eq!(this.total, 15);
            assert!(matches!(this.load, LoadState::Ready));
        });
        let _ = std::fs::remove_dir_all(&dir);
    }
}
