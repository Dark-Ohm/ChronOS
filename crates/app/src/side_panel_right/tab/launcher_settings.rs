//! Launcher settings page (T265-G) — the OSD's grid/search/categories/
//! favorites/system-actions knobs in the right panel, backed by
//! `~/.config/chronos/launcher.toml` (read-modify-write via
//! `crate::launcher::launcher_config`).
//!
//! Every control writes through `launcher_config::update` (debounced RMW, not a
//! blind serde dump), so the open OSD hot-applies through the same
//! `subscribe()` signal the page itself uses to re-render — no restart, no
//! second config copy (spec: "переносить модель лаунчера во вторую копию —
//! нельзя").
//!
//! Controls reuse the established kit: `gpui-component::Switch` for toggles and
//! `bar_settings::slider_control` for the numeric grid sliders (spec: "свой
//! слайдер не писать").

use std::collections::HashMap;

use chronos_services::Service;
use chronos_ui::Theme;
use gpui::{
    App, AnyElement, DragMoveEvent, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::switch::Switch;

use super::bar_settings::slider_control;
use super::ui;
use crate::launcher::launcher_config::{self, LauncherConfig};
use crate::launcher::system_actions::{action_id, resolve_actions};
use crate::power::PowerAction;
use crate::state::{self, AppState};

// ── Slider geometry / ranges ────────────────────────────────────────────────

/// Drag markers — one per slider so the three grid knobs never cross-fire
/// (same pattern as bar_settings' Height/Radius markers).
struct ColumnsDrag;
struct RowsDrag;
struct IconDrag;

const COLUMNS_MIN: usize = 1;
const COLUMNS_MAX: usize = 12;
const ROWS_MIN: usize = 1;
const ROWS_MAX: usize = 10;
const ICON_MIN: usize = 16;
const ICON_MAX: usize = 64;

// ── Pure helpers (unit-tested, GPUI-free) ───────────────────────────────────

/// Pointer x relative to a track → 0..=1 fraction (T202 math; zero-width safe).
fn slider_frac<D>(ev: &DragMoveEvent<D>) -> f32 {
    let rel = f32::from(ev.event.position.x - ev.bounds.origin.x);
    let w = f32::from(ev.bounds.size.width);
    (rel / w.max(1.0)).clamp(0.0, 1.0)
}

fn usize_frac(v: usize, min: usize, max: usize) -> f32 {
    ((v.saturating_sub(min)) as f32 / (max - min) as f32).clamp(0.0, 1.0)
}

fn frac_to_usize(frac: f32, min: usize, max: usize) -> usize {
    (min as f32 + frac * (max - min) as f32).round().clamp(min as f32, max as f32) as usize
}

/// Move an action one slot up/down. Bounds-clamped; `false` when the move is
/// out of range (so the click is a no-op instead of a silent wrap).
fn move_action(order: &mut Vec<PowerAction>, index: usize, delta: isize) -> bool {
    let new = index as isize + delta;
    if new < 0 || new >= order.len() as isize {
        return false;
    }
    let action = order.remove(index);
    order.insert(new as usize, action);
    true
}

/// Remove `id` from a hidden list. Returns true when it was actually removed
/// (T265-G "hidden unhide вычёркивает id").
fn unhide(hidden: &mut Vec<String>, id: &str) -> bool {
    let before = hidden.len();
    hidden.retain(|h| h != id);
    hidden.len() != before
}

// ── Page state ──────────────────────────────────────────────────────────────

pub struct LauncherSettingsTab {
    config: LauncherConfig,
    scroll: ScrollHandle,
}

impl LauncherSettingsTab {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        // Hot-reload: re-read config on any mutation (own edits or the file
        // watcher's `reload()`) so the page always mirrors launcher.toml.
        state::watch(cx, launcher_config::subscribe(), |this, (), cx| {
            this.config = launcher_config::get();
            cx.notify();
        });
        Self {
            config: launcher_config::get(),
            scroll: ScrollHandle::new(),
        }
    }

    /// id → display name for the hidden-apps group. User-hidden ids are still
    /// in the service's listed set (hide is a launcher-level filter, not a
    /// `.desktop` edit — T265-D), so names resolve from there.
    fn app_names(cx: &App) -> HashMap<String, String> {
        AppState::applications(cx)
            .get()
            .entries
            .into_iter()
            .map(|e| (e.id, e.name))
            .collect()
    }

    /// One toggle row: label + kit `Switch`.
    fn switch_row(
        theme: Theme,
        id: &str,
        label: &str,
        path: &str,
        checked: bool,
        on_toggle: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        ui::setting_row(
            ui::setting_label(theme, label, path),
            Switch::new(id.to_string())
                .checked(checked)
                .on_click(on_toggle)
                .into_any_element(),
        )
    }
}

// ── Render ──────────────────────────────────────────────────────────────────

impl Render for LauncherSettingsTab {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let cfg = self.config.clone();
        let grid = cfg.grid.sanitized();

        let header = div()
            .id("launcher-settings-header")
            .w_full()
            .px(px(14.))
            .py(px(12.))
            .border_b_1()
            .border_color(theme.border.default)
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_color(theme.text.primary)
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Launcher"),
            )
            .child(
                div()
                    .text_color(theme.text.muted)
                    .text_xs()
                    .font_family(theme.font_mono)
                    .child("~/.config/chronos/launcher.toml · OSD grid + search"),
            );

        let mut card = ui::elevated_card(theme).id("launcher-settings-card");

        // ── 1. Appearance ────────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Appearance", "[appearance]"));
        card = card.child(Self::switch_row(
            theme,
            "launcher-compact-default",
            "Compact by default",
            "appearance.compact_default",
            cfg.appearance.compact_default,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.appearance.compact_default = *checked);
            },
        ));
        card = card.child(Self::switch_row(
            theme,
            "launcher-hide-labels",
            "Hide grid labels",
            "appearance.hide_labels",
            cfg.appearance.hide_labels,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.appearance.hide_labels = *checked);
            },
        ));

        // ── 2. Grid ──────────────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Grid", "[grid] columns / rows / icon_size"));

        card = card.child(ui::setting_row(
            ui::setting_label(theme, &format!("Columns · {}", grid.columns), "grid.columns"),
            slider_control(
                theme,
                usize_frac(grid.columns, COLUMNS_MIN, COLUMNS_MAX),
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.columns = g.columns.saturating_sub(1).clamp(COLUMNS_MIN, COLUMNS_MAX);
                    });
                },
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.columns = (g.columns + 1).clamp(COLUMNS_MIN, COLUMNS_MAX);
                    });
                },
                ColumnsDrag,
                move |ev, _w, _cx| {
                    launcher_config::update(|c| {
                        c.grid.columns = frac_to_usize(slider_frac(ev), COLUMNS_MIN, COLUMNS_MAX);
                    });
                },
                "launcher-cols-minus",
                "launcher-cols-track",
                "launcher-cols-plus",
            ),
        ));

        card = card.child(ui::setting_row(
            ui::setting_label(theme, &format!("Rows · {}", grid.rows), "grid.rows"),
            slider_control(
                theme,
                usize_frac(grid.rows, ROWS_MIN, ROWS_MAX),
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.rows = g.rows.saturating_sub(1).clamp(ROWS_MIN, ROWS_MAX);
                    });
                },
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.rows = (g.rows + 1).clamp(ROWS_MIN, ROWS_MAX);
                    });
                },
                RowsDrag,
                move |ev, _w, _cx| {
                    launcher_config::update(|c| {
                        c.grid.rows = frac_to_usize(slider_frac(ev), ROWS_MIN, ROWS_MAX);
                    });
                },
                "launcher-rows-minus",
                "launcher-rows-track",
                "launcher-rows-plus",
            ),
        ));

        card = card.child(ui::setting_row(
            ui::setting_label(theme, &format!("Icon size · {}px", grid.icon_size), "grid.icon_size"),
            slider_control(
                theme,
                usize_frac(grid.icon_size as usize, ICON_MIN, ICON_MAX),
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.icon_size = (g.icon_size.saturating_sub(4)).clamp(ICON_MIN as u32, ICON_MAX as u32);
                    });
                },
                move |_ev, _w, _cx| {
                    launcher_config::update(|c| {
                        let g = c.grid.sanitized();
                        c.grid.icon_size = (g.icon_size + 4).clamp(ICON_MIN as u32, ICON_MAX as u32);
                    });
                },
                IconDrag,
                move |ev, _w, _cx| {
                    launcher_config::update(|c| {
                        c.grid.icon_size = frac_to_usize(slider_frac(ev), ICON_MIN, ICON_MAX) as u32;
                    });
                },
                "launcher-icon-minus",
                "launcher-icon-track",
                "launcher-icon-plus",
            ),
        ));

        // ── 3. Search ────────────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Search", "[search]"));
        card = card.child(Self::switch_row(
            theme,
            "launcher-include-hidden",
            "Include hidden apps",
            "search.include_hidden",
            cfg.search.include_hidden,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.search.include_hidden = *checked);
            },
        ));
        card = card.child(Self::switch_row(
            theme,
            "launcher-inline-completion",
            "Inline completion",
            "search.inline_completion",
            cfg.search.inline_completion,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.search.inline_completion = *checked);
            },
        ));

        // ── 4. Categories ────────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Categories", "[categories] hide"));
        card = card.child(
            div()
                .w_full()
                .text_color(theme.text.muted)
                .text_xs()
                .child("Empty categories are hidden automatically."),
        );
        let hidden_cats = cfg.categories.hide.clone();
        card = card.child(if hidden_cats.is_empty() {
            div()
                .w_full()
                .text_color(theme.text.muted)
                .text_xs()
                .child("No hidden categories.")
                .into_any_element()
        } else {
            div()
                .w_full()
                .flex_col()
                .gap(px(6.))
                .children(hidden_cats.into_iter().map(|cat| {
                    let cat_for_click = cat.clone();
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(10.))
                        .py(px(6.))
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .bg(theme.bg.secondary.opacity(0.5))
                        .child(
                            div()
                                .text_color(theme.text.secondary)
                                .text_size(px(11.))
                                .child(cat.clone()),
                        )
                        .child(
                            div()
                                .id(format!("launcher-cat-show-{cat}"))
                                .text_color(theme.accent.primary)
                                .text_size(px(11.))
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.accent.secondary))
                                .child("Show")
                                .on_click(move |_ev, _w, _cx| {
                                    launcher_config::update(|c| {
                                        c.categories.hide.retain(|h| h != &cat_for_click);
                                    });
                                }),
                        )
                }))
                .into_any_element()
        });

        // ── 5. Favorites ─────────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Favorites", "[favorites]"));
        card = card.child(Self::switch_row(
            theme,
            "launcher-fav-sort-alpha",
            "Sort alphabetically",
            "favorites.sort_alpha",
            cfg.favorites.sort_alpha,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.favorites.sort_alpha = *checked);
            },
        ));
        card = card.child(Self::switch_row(
            theme,
            "launcher-fav-hide-labels",
            "Hide labels",
            "favorites.hide_labels",
            cfg.favorites.hide_labels,
            |checked, _w, _cx| {
                launcher_config::update(|c| c.favorites.hide_labels = *checked);
            },
        ));

        // ── 6. System actions ────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "System actions", "[system_actions] order"));
        let order = resolve_actions(&cfg);
        card = card.child(
            div()
                .w_full()
                .flex_col()
                .gap(px(6.))
                .children(order.iter().enumerate().map(|(i, action)| {
                    let action = *action;
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(10.))
                        .py(px(6.))
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .bg(theme.bg.secondary.opacity(0.5))
                        .child(
                            div()
                                .text_color(theme.text.secondary)
                                .text_size(px(11.))
                                .child(action.label()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .id(format!("launcher-sys-up-{i}"))
                                        .text_color(theme.text.muted)
                                        .text_size(px(11.))
                                        .cursor_pointer()
                                        .hover(|s| s.text_color(theme.accent.primary))
                                        .child("↑")
                                        .on_click(move |_ev, _w, _cx| {
                                            launcher_config::update(|c| {
                                                let mut actions = resolve_actions(c);
                                                if move_action(&mut actions, i, -1) {
                                                    c.system_actions.order = actions
                                                        .iter()
                                                        .map(|a| action_id(*a).to_string())
                                                        .collect();
                                                }
                                            });
                                        }),
                                )
                                .child(
                                    div()
                                        .id(format!("launcher-sys-down-{i}"))
                                        .text_color(theme.text.muted)
                                        .text_size(px(11.))
                                        .cursor_pointer()
                                        .hover(|s| s.text_color(theme.accent.primary))
                                        .child("↓")
                                        .on_click(move |_ev, _w, _cx| {
                                            launcher_config::update(|c| {
                                                let mut actions = resolve_actions(c);
                                                if move_action(&mut actions, i, 1) {
                                                    c.system_actions.order = actions
                                                        .iter()
                                                        .map(|a| action_id(*a).to_string())
                                                        .collect();
                                                }
                                            });
                                        }),
                                ),
                        )
                }))
                .child(
                    div()
                        .id("launcher-sys-reset")
                        .w_full()
                        .text_color(theme.accent.primary)
                        .text_size(px(11.))
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.accent.secondary))
                        .child("Reset to default")
                        .on_click(|_ev, _w, _cx| {
                            launcher_config::update(|c| c.system_actions.order.clear());
                        }),
                ),
        );

        // ── 7. Hidden apps ───────────────────────────────────────────────
        card = card.child(ui::section_header(theme, "Hidden apps", "[hidden]"));
        let names = Self::app_names(cx);
        let hidden = cfg.hidden.clone();
        card = card.child(if hidden.is_empty() {
            div()
                .w_full()
                .text_color(theme.text.muted)
                .text_xs()
                .child("No hidden apps.")
                .into_any_element()
        } else {
            div()
                .w_full()
                .flex_col()
                .gap(px(6.))
                .children(hidden.into_iter().map(|id| {
                    let label = names.get(&id).cloned().unwrap_or_else(|| id.clone());
                    let id_for_click = id.clone();
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(10.))
                        .py(px(6.))
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .bg(theme.bg.secondary.opacity(0.5))
                        .child(
                            div()
                                .text_color(theme.text.secondary)
                                .text_size(px(11.))
                                .child(label),
                        )
                        .child(
                            div()
                                .id(format!("launcher-unhide-{id_for_click}"))
                                .text_color(theme.accent.primary)
                                .text_size(px(11.))
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.accent.secondary))
                                .child("Unhide")
                                .on_click(move |_ev, _w, _cx| {
                                    launcher_config::update(|c| {
                                        unhide(&mut c.hidden, &id_for_click);
                                    });
                                }),
                        )
                }))
                .into_any_element()
        });

        // ── Frame ────────────────────────────────────────────────────────
        // Rough header height offset so a short card does not force a scroll
        // (same T249 floor idea as acp_settings, minus the fixed header).
        let min_card_h = (window.bounds().size.height.as_f32() - 56.0).max(0.0);
        div()
            .id("launcher-settings-tab")
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("launcher-settings-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .p(px(14.))
                    .child(card.min_h(px(min_card_h))),
            )
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unhide_removes_the_id() {
        let mut hidden = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert!(unhide(&mut hidden, "b"));
        assert_eq!(hidden, vec!["a", "c"]);
        assert!(!unhide(&mut hidden, "zzz"), "missing id must not report a change");
    }

    #[test]
    fn move_action_reorders_and_clamps() {
        let mut order = vec![
            PowerAction::Lock,
            PowerAction::LogOut,
            PowerAction::Sleep,
        ];
        assert!(move_action(&mut order, 1, -1));
        assert_eq!(order, vec![PowerAction::LogOut, PowerAction::Lock, PowerAction::Sleep]);
        // Clamp: moving the top item up is a no-op.
        assert!(!move_action(&mut order, 0, -1));
        assert!(!move_action(&mut order, 2, 1));
    }

    #[test]
    fn slider_frac_maps_and_clamps() {
        // Exercise the pure mapping helpers, which carry the slider math.
        assert_eq!(frac_to_usize(0.0, COLUMNS_MIN, COLUMNS_MAX), COLUMNS_MIN);
        assert_eq!(frac_to_usize(1.0, COLUMNS_MIN, COLUMNS_MAX), COLUMNS_MAX);
        assert_eq!(frac_to_usize(2.0, ICON_MIN, ICON_MAX), ICON_MAX, "clamps at top");
        assert_eq!(usize_frac(7, 1, 12), (6.0 / 11.0));
    }
}
