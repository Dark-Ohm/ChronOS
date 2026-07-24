//! Hot-reloadable render functions for ChronOS bar widgets.
//!
//! This crate is compiled as a `cdylib` + `rlib` and loaded at runtime by
//! `hot-lib-reloader` during development. Only pure render functions live
//! here — no state, no subscriptions, no `cx.observe`.

use gpui::{AnyElement, Hsla, div, prelude::*, px};

use chronos_ui::Theme;

/// Render the network widget element from pre-computed display data.
///
/// All state management (speed computation, connectivity polling) stays in
/// `crates/app`. This function receives the already-formatted strings and
/// colors and builds the GPUI element tree.
#[unsafe(no_mangle)]
pub fn render_network(
    dl: &str,
    ul: &str,
    dot_color: Hsla,
    speed_color: Hsla,
    theme: &Theme,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .w(px(6.))
                .h(px(6.))
                .rounded_full()
                .bg(dot_color)
                .flex_none(),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .child(
                    div()
                        .child(format!("\u{2193} {dl}"))
                        .text_color(speed_color)
                        .text_size(theme.font_sizes.xs)
                        .font_family(theme.font_mono),
                )
                .child(
                    div()
                        .child(format!("\u{2191} {ul}"))
                        .text_color(speed_color)
                        .text_size(theme.font_sizes.xs)
                        .font_family(theme.font_mono),
                ),
        )
        .into_any_element()
}
