//! Updates popup view — pending-update list + "Upgrade all" button.
//!
//! Pixel-faithful to `design/Updates Popup.dc.html` (dark reference + light
//! Light C variant). Every hex, padding, radius, font-size, font-weight here
//! comes from that mockup — do not re-derive by eye.

use std::collections::HashSet;

use gpui::{
    AnyElement, App, Context, Corners, InteractiveElement, IntoElement, Render, ScrollHandle,
    Styled, Window, canvas, div, prelude::*, px, svg,
};

use chronos_services::{PackageUpdate, Service, UpdateSource, UpgradeState};

use crate::state::AppState;
use crate::updates_popup::{MAX_LIST_H, refresh, upgrade_all, upgrade_selected};

use chronos_ui::{Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer};
use crate::motion;

// ── Geometry from mockup ────────────────────────────────────────────
const HEADER_PY: f32 = 12.;
const HEADER_PX: f32 = 14.;
const ROW_PY: f32 = 9.;
const ROW_PX: f32 = 14.;
const FOOTER_PY: f32 = 12.;
const FOOTER_PX: f32 = 14.;
const BTN_PY: f32 = 8.;

/// Width of the left gutter that carries the selection indicator (px).
/// Mockup-fixed: smaller reads as noise, larger eats the name column.
/// Task constraint: <= 18px (T119 §1).
const SELECTION_GUTTER: f32 = 16.;

pub struct UpdatesPopupView {
    scroll: ScrollHandle,
    /// Ephemeral UI-only selection of package names (toggled by row
    /// clicks). Lives on the view, NOT the service — selection vanishes
    /// whenever the popup closes (mirrors how a combobox's highlight is
    /// per-session), and a `Running` upgrade disables further toggles so
    /// the user can't scramble the in-flight package set.
    selection: HashSet<String>,
    /// View-driven enter progress 0..=1 (T129).
    enter_t: f32,
}

impl UpdatesPopupView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        motion::arm_enter_progress(cx, |this, t| {
            this.enter_t = t;
        });
        Self {
            scroll: ScrollHandle::new(),
            selection: HashSet::new(),
            enter_t: 0.0,
        }
    }
}

impl Render for UpdatesPopupView {
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
        let radius = theme.radius; // 6px
        let radius_lg = theme.radius_lg; // 12px
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

        // ── Selection hygiene ───────────────────────────────────────
        // Drop any names that no longer appear in the list (e.g. after a
        // `Refresh` shrunk the pending set) so the footer label doesn't
        // mysteriously stay on "Upgrade selected" with zero visible rows.
        // Mutating `self.selection` inside `render` is fine — it's a
        // generic `&mut self` borrow, `cx.listener` is reserved for
        // event-driven mutations only.
        self.selection
            .retain(|n| visible_updates.iter().any(|u| &u.name == n));
        let is_running = matches!(state.upgrade_state, UpgradeState::Running(_));
        let is_checking = state.checking;
        // Snapshot the selection for the render pass — `self` is borrowed
        // by `render` for the entire frame, but `cx.listener` closures
        // borrow `&mut this` only at click time, so a captured-into-closure
        // borrowed snapshot of `self.selection` is unnecessary.
        let selection_snapshot: HashSet<String> = self.selection.clone();

        // Check button is inert during upgrade or mid-refresh.
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

        // ── Header ──────────────────────────────────────────────────
        // Title (left) ── spacer ── [Check updates] mono text only.
        // No icon. No in-popup ✕ — dismiss is bar toggle (user 2026-07-24).
        let mut check_btn = div()
            .id("updates-popup-check")
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
                .on_click(|_event, _window, cx: &mut App| {
                    refresh(cx);
                });
        }

        let header = div()
            .w_full()
            .flex()
            .items_center()
            .px(px(HEADER_PX))
            .py(px(HEADER_PY))
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

        // ── List ────────────────────────────────────────────────────
        let list: AnyElement = if visible_updates.is_empty() && completed.is_empty() {
            div()
                .w_full()
                .px(px(ROW_PX))
                .py(px(ROW_PY))
                .text_color(text_muted)
                .font_family(font_mono)
                .text_size(theme.font_sizes.sm)
                .child("System is up to date")
                .into_any_element()
        } else {
            let rows: Vec<AnyElement> = visible_updates
                .iter()
                .map(|u| {
                    let is_selected = selection_snapshot.contains(&u.name);
                    render_row(
                        u,
                        is_selected,
                        is_running,
                        text_primary,
                        text_secondary,
                        text_muted,
                        hover,
                        accent,
                        border,
                        font_mono,
                        radius,
                        cx,
                    )
                })
                .collect();
            div()
                .id("updates-popup-list")
                .w_full()
                .max_h(px(MAX_LIST_H))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .flex_col()
                .children(rows)
                .into_any_element()
        };

        // ── Footer ──────────────────────────────────────────────────
        let upgrade_state = state.upgrade_state.clone();
        let footer: AnyElement = if updates.is_empty()
            && matches!(upgrade_state, UpgradeState::Idle)
        {
            div().into_any_element()
        } else {
            let status_line: AnyElement = match &upgrade_state {
                UpgradeState::Idle => div().into_any_element(),
                UpgradeState::Running(_) => div().into_any_element(),
                UpgradeState::Done => div()
                    .w_full()
                    .px(px(FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.success)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade complete")
                    .into_any_element(),
                UpgradeState::Failed => div()
                    .w_full()
                    .px(px(FOOTER_PX))
                    .pb(px(2.))
                    .text_color(theme.status.error)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .child("Upgrade failed")
                    .into_any_element(),
            };

            let button: AnyElement = if let UpgradeState::Running(ref progress) = upgrade_state {
                // Spinner + progress bar + live output
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
                        // Spinner row
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
                                    .text_color(text_muted)
                                    .font_family(font_mono)
                                    .text_size(px(12.))
                                    .child(format!(
                                        "Upgrading… {}/{}",
                                        progress.current, progress.total
                                    )),
                            ),
                    )
                    .child(
                        // Progress bar row
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                // Track
                                div().flex_1().h(px(4.)).rounded(px(2.)).bg(hover).child(
                                    // Fill
                                    div().h_full().rounded(px(2.)).bg(accent).w(
                                        gpui::Length::Definite(gpui::DefiniteLength::Fraction(
                                            progress_frac,
                                        )),
                                    ),
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
                        // Live output line
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
                // Footer label flips per selection: empty selection →
                // "Upgrade all" (full sysupgrade), non-empty → "Upgrade
                // selected" (targeted `-S` install of those names). The
                // button's `id` keeps the same shape T118 relied on so the
                // running-mode display path is untouched.
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
                    .id("updates-popup-upgrade-action")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .py(px(BTN_PY))
                    .rounded(radius)
                    .border_1()
                    .border_color(accent)
                    .text_color(accent)
                    .font_family(font_mono)
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .hover(|s| s.border_color(accent_hover).text_color(accent_hover))
                    .child(label)
                    .on_click(move |_event, window, cx: &mut App| {
                        if selected_pkgs.is_empty() {
                            upgrade_all(window, cx);
                        } else {
                            upgrade_selected(selected_pkgs.clone(), window, cx);
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
                        .px(px(FOOTER_PX))
                        .py(px(FOOTER_PY))
                        .child(button),
                )
                .into_any_element()
        };

        // ── Card ────────────────────────────────────────────────────
        // Радиус карточки оставляем `radius` (6px) — НЕ меняем геометрию.
        // Тени / blur / glow берём из `theme.elevation_popup()` (T128):
        // light-схема получает тот же Light-C рецепт, что volume/system.
        let elev = theme.elevation_popup();
        // Keep mockup corner radius 6px (not elev.radius / radius_lg).
        let blur_layer = elevation_blur_layer(&elev, radius);

        let card = div()
            .window_font(&theme)
            .relative()
            .flex_col()
            .rounded(radius) // 6px, not 10px
            .bg(bg)
            .border_1()
            .border_color(border)
            .shadow(elev.shadows.to_vec())
            .child(blur_layer)
            .overflow_hidden();
        let mut card = elevation_apply_light_chrome(&elev, card);
        let card = card.child(header).child(list).child(footer);
        motion::apply_enter_rise(card, self.enter_t)
    }
}

// ── Row ─────────────────────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
fn render_row(
    update: &PackageUpdate,
    is_selected: bool,
    is_running: bool,
    text_primary: gpui::Hsla,
    text_secondary: gpui::Hsla,
    text_muted: gpui::Hsla,
    hover: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    font_mono: &'static str,
    radius: gpui::Pixels,
    cx: &mut Context<UpdatesPopupView>,
) -> AnyElement {
    let is_aur = matches!(update.source, UpdateSource::Aur);
    let name = update.name.clone();

    // ── Selection gutter: 16px column, no SVG asset needed ───
    // Selected → 10px accent-filled rounded square. Unselected → 10px
    // transparent square with a 1px muted border outline. Same outer
    // footprint in both states — pixel layout stays identical whether or
    // not a row is selected, so the right-column version string never
    // shifts left/right as you toggle.
    let indicator = div()
        .flex_none()
        .w(px(SELECTION_GUTTER))
        .flex()
        .items_center()
        .justify_center()
        .child(if is_selected {
            div()
                .w(px(10.))
                .h(px(10.))
                .rounded(px(2.))
                .bg(accent)
                .into_any_element()
        } else {
            div()
                .w(px(10.))
                .h(px(10.))
                .rounded(px(2.))
                .border_1()
                .border_color(text_muted)
                .into_any_element()
        });

    // ── Row layout: [indicator] [name] [AUR?] ──gap── [old → new] ──
    let name_el = div()
        .flex_1()
        .min_w(px(0.))
        .text_color(text_primary)
        .font_family(font_mono)
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(update.name.clone());

    let aur_badge: AnyElement = if is_aur {
        div()
            .flex_none()
            .rounded(radius)
            .px(px(5.))
            .py(px(1.))
            .border_1()
            .border_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f74d)))
            .bg(gpui::Hsla::from(gpui::rgba(0xcb_a6_f71f)))
            .text_color(gpui::Hsla::from(gpui::rgba(0xcb_a6_f7ff)))
            .font_family(font_mono)
            .text_size(px(9.5))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child("AUR")
            .into_any_element()
    } else {
        div().into_any_element()
    };

    let versions = div()
        .flex_none()
        .font_family(font_mono)
        .text_size(px(11.))
        .flex()
        .items_center()
        .gap(px(5.))
        .child(
            div()
                .text_color(text_muted)
                .child(update.old_version.clone()),
        )
        .child(div().text_color(text_muted).child("→"))
        .child(
            div()
                .text_color(text_secondary)
                .child(update.new_version.clone()),
        );

    // Clicking a row toggles its membership in the selection — but only
    // when no upgrade is in flight. During `Running` the user can still
    // scroll/read the list; we just freeze the selection so the dispatched
    // package set (captured at the moment the user clicked "Upgrade
    // selected") stays honest. `cx.listener` lets us reach `&mut this`
    // from the click handler without re-entering `handle.update`.
    if is_running {
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(ROW_PX))
            .py(px(ROW_PY))
            .border_b_1()
            .border_color(border)
            .child(indicator)
            .child(name_el)
            .child(aur_badge)
            .child(versions)
            .into_any_element()
    } else {
        // Stable id for the row so toggles keep state across re-renders;
        // GPUI takes `Into<SharedString>` hierea, so `format!` is fine —
        // no `.leak()` (which would leak every render frame).
        let row_id = format!("updates-popup-row-{name}");
        div()
            .id(row_id)
            .w_full()
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(ROW_PX))
            .py(px(ROW_PY))
            .border_b_1()
            .border_color(border)
            .hover(|s| s.bg(hover))
            .child(indicator)
            .child(name_el)
            .child(aur_badge)
            .child(versions)
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if this.selection.contains(&name) {
                    this.selection.remove(&name);
                    tracing::debug!(target: "chronos::updates_popup", "deselected {name}");
                } else {
                    this.selection.insert(name.clone());
                    tracing::debug!(target: "chronos::updates_popup", "selected {name}");
                }
                cx.notify();
            }))
            .into_any_element()
    }
}
