//! Updates tab — pending official + AUR update list with "Upgrade all" /
//! "Upgrade selected" (T294).
//!
//! Replaces the deleted updates popup. Contract (T294): apply is ALWAYS
//! pacman (official repos, `pkexec pacman -Syu` / `-Sy -- <official>`);
//! AUR rows are display-only — hover reveals a `yay` hint, clicking them does
//! nothing, and they never enter the selection that feeds "Upgrade selected".
//! Sections in the list ("Repos" / "AUR") make the source visible without a
//! hover.
//!
//! The tab hosts its OWN service subscription (the popup's global watcher is
//! gone — same pattern as `DisplayTab`), so the list repaints on updates.

use std::collections::HashSet;

use chronos_services::{AurCommand, PackageUpdate, Service, UpdateSource, UpdatesState, UpgradeState};
use chronos_ui::{Theme, WindowRootExt};
use gpui::{
    AnyElement, App, Context, IntoElement, InteractiveElement, Render, ScrollHandle, Styled, Window,
    div, prelude::*, px, svg,
};

use crate::state::{self, AppState};
use crate::updates_list as list;

pub struct UpdatesTab {
    scroll: ScrollHandle,
    /// Ephemeral UI-only selection of OFFICIAL package names (toggled by
    /// row clicks). Lives on the view, NOT the service — selection vanishes
    /// when the tab is closed, and a `Running` upgrade freezes further
    /// toggles so the dispatched package set can't be scrambled mid-flight.
    selection: HashSet<String>,
    /// Name of the AUR row currently showing its inline `yay` hint.
    hovered_aur: Option<String>,
}

impl UpdatesTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let signal = AppState::aur(cx).subscribe();
        state::watch(cx, signal, |_this: &mut Self, _s: UpdatesState, cx| {
            cx.notify();
        });
        Self {
            scroll: ScrollHandle::new(),
            selection: HashSet::new(),
            hovered_aur: None,
        }
    }

    fn toggle_selection(&mut self, name: String) {
        if self.selection.contains(&name) {
            self.selection.remove(&name);
        } else {
            self.selection.insert(name);
        }
    }
}

impl Render for UpdatesTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = AppState::aur(cx).get();
        let updates = state.updates.clone();
        let count = updates.len();

        let theme = *Theme::global(cx);
        let bg = theme.bg.primary;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let border = theme.border.default;
        let radius = theme.radius;
        let accent = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let hover = theme.interactive.hover;
        let font_mono = theme.font_mono;

        // ── Visible updates (filter completed during upgrade) ─────
        let completed: Vec<String> = match &state.upgrade_state {
            UpgradeState::Running(p) => p.completed_names.clone(),
            _ => Vec::new(),
        };
        let visible_updates: Vec<_> = updates
            .iter()
            .filter(|u| !completed.contains(&u.name))
            .collect();
        let visible_count = visible_updates.len();

        // ── Selection hygiene ──────────────────────────────────────
        // Drop names that no longer appear; keep the set official-only (AUR
        // is never selectable, and a package may flip source between checks).
        self.selection.retain(|n| {
            visible_updates
                .iter()
                .any(|u| u.name == *n && u.source != UpdateSource::Aur)
        });
        let selection_snapshot: HashSet<String> = self.selection.clone();
        let is_running = matches!(state.upgrade_state, UpgradeState::Running(_));
        let is_checking = state.checking;

        // Shared handle for row-level click/hover mutation (launcher
        // pattern: capture `Entity<Self>`, call `.update(cx, …)`).
        let handle = cx.entity();

        // ── Header ────────────────────────────────────────────────
        let check_enabled = !is_running && !is_checking;
        let check_label: &'static str = if is_checking {
            "Checking…"
        } else {
            "Check updates"
        };
        let check_color = if check_enabled {
            text_secondary
        } else {
            text_muted
        };
        let mut check_btn = div()
            .id("updates-tab-check")
            .flex_none()
            .h(px(22.))
            .px(px(8.))
            .rounded(radius)
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(if check_enabled { border } else { hover })
            .child(
                div()
                    .text_color(check_color)
                    .font_family(font_mono)
                    .text_size(px(11.))
                    .child(check_label),
            );
        if check_enabled {
            check_btn = check_btn
                .cursor_pointer()
                .hover(|s| s.bg(hover).border_color(text_muted))
                .on_click(move |_event, _window, cx| {
                    AppState::aur(cx).dispatch(AurCommand::Refresh);
                });
        }

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .px(px(list::HEADER_PX))
            .py(px(list::HEADER_PY))
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .text_color(text_primary)
                    .font_family(font_mono)
                    .text_size(theme.font_sizes.sm)
                    .child(if visible_count > 0 {
                        format!("Updates ({visible_count})")
                    } else if count > 0 {
                        format!("Updates ({count})")
                    } else {
                        "Updates".to_string()
                    }),
            )
            .child(div().flex_1())
            .child(check_btn);

        // ── List (sections: Repos / AUR) ─────────────────────────
        let list_el: AnyElement = if visible_updates.is_empty() {
            div()
                .w_full()
                .px(px(list::ROW_PX))
                .py(px(list::ROW_PY))
                .text_color(text_muted)
                .font_family(font_mono)
                .text_size(theme.font_sizes.sm)
                .child("System is up to date")
                .into_any_element()
        } else {
            let mut children: Vec<AnyElement> = Vec::new();

            // Section: Repos (official — selectable/upgradeable).
            children.push(section_label("Repos", text_muted, font_mono));
            for u in visible_updates.iter().filter(|u| !list::is_aur(u)) {
                let is_selected = selection_snapshot.contains(&u.name);
                children.push(row(
                    *u,
                    is_running,
                    list::is_aur(u),
                    is_selected,
                    self.hovered_aur.as_deref() == Some(u.name.as_str()),
                    text_primary,
                    text_secondary,
                    text_muted,
                    hover,
                    accent,
                    border,
                    font_mono,
                    radius,
                    &handle,
                ));
            }

            // Section: AUR (display-only, hover hint).
            children.push(section_label("AUR", text_muted, font_mono));
            for u in visible_updates.iter().filter(|u| list::is_aur(u)) {
                children.push(row(
                    *u,
                    is_running,
                    true,
                    false,
                    self.hovered_aur.as_deref() == Some(u.name.as_str()),
                    text_primary,
                    text_secondary,
                    text_muted,
                    hover,
                    accent,
                    border,
                    font_mono,
                    radius,
                    &handle,
                ));
            }

            div()
                .id("updates-tab-list")
                .w_full()
                .flex_col()
                .children(children)
                .into_any_element()
        };

        // ── Footer ────────────────────────────────────────────────
        let upgrade_state = state.upgrade_state.clone();
        let footer: AnyElement = if updates.is_empty() && matches!(upgrade_state, UpgradeState::Idle)
        {
            div().into_any_element()
        } else {
            let status_line: AnyElement = match &upgrade_state {
                UpgradeState::Idle => div().into_any_element(),
                UpgradeState::Running(_) => div().into_any_element(),
                UpgradeState::Done => div()
                    .w_full()
                    .px(px(list::FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.success)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade complete")
                    .into_any_element(),
                UpgradeState::Failed => div()
                    .w_full()
                    .px(px(list::FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.error)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade failed")
                    .into_any_element(),
            };
            let button: AnyElement = if let UpgradeState::Running(ref progress) = upgrade_state {
                let pct = progress.percent();
                let pct_text = format!("{pct}%");
                let progress_frac = if progress.total > 0 {
                    progress.current as f32 / progress.total as f32
                } else {
                    0.0
                };
                div()
                    .w_full()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(8.))
                            .child(
                                svg()
                                    .path("icons/arrows-clockwise.svg")
                                    .size(px(14.))
                                    .text_color(accent),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(4.))
                                    .rounded(px(2.))
                                    .bg(theme.border.subtle)
                                    .child(
                                        div()
                                            .h_full()
                                            .rounded(px(2.))
                                            .bg(accent)
                                            .w(px(progress_frac * 320.)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(text_secondary)
                                    .font_family(font_mono)
                                    .text_size(px(11.))
                                    .child(pct_text),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_color(text_muted)
                            .font_family(font_mono)
                            .text_size(px(10.5))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(progress.last_line.clone()),
                    )
                    .into_any_element()
            } else if !updates.is_empty() {
                // Flips per selection: empty → "Upgrade all", non-empty →
                // "Upgrade selected" (official names only — AUR never reaches
                // this argv; T294).
                let has_selection = !selection_snapshot.is_empty();
                let label: &'static str = if has_selection {
                    "Upgrade selected"
                } else {
                    "Upgrade all"
                };
                let selected_pkgs: Vec<String> = if has_selection {
                    selection_snapshot.iter().cloned().collect()
                } else {
                    Vec::new()
                };
                div()
                    .id("updates-tab-upgrade-action")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .py(px(list::BTN_PY))
                    .rounded(radius)
                    .border_1()
                    .border_color(accent)
                    .text_color(accent)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .hover(|s| s.border_color(accent_hover).text_color(accent_hover))
                    .child(label)
                    .on_click(move |_event, _window, cx: &mut App| {
                        if selected_pkgs.is_empty() {
                            AppState::aur(cx).dispatch(AurCommand::UpgradeAll);
                        } else {
                            AppState::aur(cx).dispatch(AurCommand::UpgradeSelected {
                                packages: selected_pkgs.clone(),
                            });
                        }
                    })
                    .into_any_element()
            } else {
                div().into_any_element()
            };

            div()
                .w_full()
                .flex_col()
                .child(status_line)
                .child(
                    div()
                        .w_full()
                        .px(px(list::FOOTER_PX))
                        .py(px(list::FOOTER_PY))
                        .child(button),
                )
                .into_any_element()
        };

        // ── Assemble the tab body ─────────────────────────────────
        div()
            .id("updates-tab")
            .window_font(&theme)
            .size_full()
            .bg(bg)
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("updates-tab-scroll")
                    .w_full()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(list_el),
            )
            .child(footer)
            .into_any_element()
    }
}

/// Build one list row. Official rows are selectable (click toggles the
/// selection indicator); AUR rows are display-only — hover reveals the
/// inline `yay` hint, and clicking is intentionally inert (T294). While an
/// upgrade is `Running` the whole row is frozen (no toggle/hover change) so
/// the dispatched package set can't be scrambled mid-flight.
#[allow(clippy::too_many_arguments)]
fn row(
    update: &PackageUpdate,
    is_running: bool,
    is_aur: bool,
    is_selected: bool,
    hovered: bool,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    hover: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    font_mono: &'static str,
    radius: gpui::Pixels,
    handle: &gpui::Entity<UpdatesTab>,
) -> AnyElement {
    let name = update.name.clone();
    let indicator = list::selection_indicator(is_selected, accent, text_muted);
    let name_el = list::name_cell(update, text_primary, font_mono);
    let badge = list::aur_badge(is_aur, radius, font_mono);
    let versions = list::versions(update, text_muted, text_secondary, font_mono);

    // Row id must be stable across re-renders so hover/click keep state.
    let row_id = format!("updates-tab-row-{name}");

    // Top line: [indicator] [name] [AUR?] ──gap── [old → new].
    let top_line = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(10.))
        .px(px(list::ROW_PX))
        .py(px(list::ROW_PY))
        .child(indicator)
        .child(name_el)
        .child(badge)
        .child(versions);

    // AUR rows reveal the hint as a second line while hovered (inline hover
    // card — not a floating surface, which a scrolling list would clip).
    let mut container = div().id(row_id).w_full().flex_col();
    if is_aur && hovered {
        container = container.child(top_line).child(aur_hint(text_muted, font_mono));
    } else {
        container = container.child(top_line);
    }
    container = container.border_b_1().border_color(border);
    if !is_running {
        container = container.hover(|s| s.bg(hover));
    }

    if is_aur {
        // Display-only: hover toggles the hint, click is a no-op.
        container
            .on_hover({
                let handle = handle.clone();
                let name = name.clone();
                move |hovered, _window, cx: &mut App| {
                    let cur = if *hovered { Some(name.clone()) } else { None };
                    handle.update(cx, |this, cx| {
                        this.hovered_aur = cur;
                        cx.notify();
                    });
                }
            })
            .into_any_element()
    } else if is_running {
        // Freeze toggles while an upgrade streams; still keep the id/hover
        // chrome but no click handler.
        container.into_any_element()
    } else {
        let handle = handle.clone();
        let name = name.clone();
        container
            .on_click(move |_event, _window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    this.toggle_selection(name.clone());
                    cx.notify();
                });
            })
            .into_any_element()
    }
}
/// Section separator in the list — "Repos" / "AUR" (T294: the source must be
/// visible without a hover).
fn section_label(label: &str, text_muted: gpui::Hsla, font_mono: &'static str) -> AnyElement {
    div()
        .w_full()
        .px(px(list::ROW_PX))
        .pt(px(10.))
        .pb(px(4.))
        .text_color(text_muted)
        .font_family(font_mono)
        .text_size(px(9.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(label.to_string())
        .into_any_element()
}

/// Inline AUR hover card: two-line `yay` hint, indented under the name.
fn aur_hint(text_muted: gpui::Hsla, font_mono: &'static str) -> AnyElement {
    div()
        .w_full()
        .flex_col()
        .gap(px(2.))
        .px(px(list::ROW_PX + list::SELECTION_GUTTER))
        .pb(px(8.))
        .child(
            div()
                .text_color(text_muted)
                .font_family(font_mono)
                .text_size(px(10.5))
                .child(list::AUR_HINT_LINE1.to_string()),
        )
        .child(
            div()
                .text_color(text_muted)
                .font_family(font_mono)
                .text_size(px(10.5))
                .child(list::AUR_HINT_LINE2.to_string()),
        )
        .into_any_element()
}
