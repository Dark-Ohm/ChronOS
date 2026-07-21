//! Static disk / removable section — replaces mockup Battery card.
//! Live udisks2 enumeration is a separate track; this is pixel-placeholder.
//!
//! Internal disk: usage bar only.
//! Removable USB: usage bar + mount / unmount / eject (no-op clicks).

use gpui::{IntoElement, div, prelude::*, px, relative, rgb};
use gpui_rsx::rsx;

fn usage_card(
    name: &'static str,
    usage_label: &'static str,
    fill_pct: f32,
    action_row: bool,
) -> impl IntoElement {
    let fill_frac = fill_pct.clamp(0.0, 1.0);

    div()
        .flex()
        .flex_col()
        .gap(px(if action_row { 8. } else { 9. }))
        .p(px(12.))
        .rounded(px(9.))
        .bg(rgb(0x1e_1e_2e))
        .border_1()
        .border_color(rgb(0x23_23_36))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(0xcd_d6_f4))
                        .child(name),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.5))
                        .text_color(rgb(0x6c_70_86))
                        .child(usage_label),
                ),
        )
        .child(
            div()
                .h(px(5.))
                .w_full()
                .rounded(px(3.))
                .bg(rgb(0x31_32_44))
                .child(
                    div()
                        .h_full()
                        .w(relative(fill_frac))
                        .rounded(px(3.))
                        .bg(rgb(0xa6_e3_a1)),
                ),
        )
        .when(action_row, |d| {
            d.child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(disk_action("disk-mount", "монтировать"))
                    .child(disk_action("disk-umount", "размонт."))
                    .child(disk_action("disk-eject", "извлечь")),
            )
        })
}

fn disk_action(id: &'static str, label: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .py(px(4.))
        .rounded(px(5.))
        .border_1()
        .border_color(rgb(0x45_47_5a))
        .text_size(px(10.))
        .text_color(rgb(0xa6_ad_c8))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x23_23_36)))
        .child(label)
        .on_click(move |_, _, _| {
            tracing::debug!("side_panel_right: disk action stub ({label})");
        })
}

/// Static list: 1 internal + 1 USB with action buttons.
pub fn render_disks_section() -> impl IntoElement {
    rsx! {
        <div class="flex flex-col" gap={px(10.)}>
            {usage_card("Disk", "318G / 512G", 0.62, false)}
            {usage_card("USB", "12G / 64G", 0.19, true)}
        </div>
    }
}
