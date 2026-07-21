//! Live disk / removable section — udisks2 inventory + mount/unmount/eject.
//!
//! Internal disk: usage bar only.
//! Removable: usage bar + mount / unmount / eject.

use chronos_services::{DiskInfo, DisksCommand};
use gpui::{App, ElementId, IntoElement, SharedString, div, prelude::*, px, relative, rgb};

use crate::state::AppState;

fn usage_card(disk: &DiskInfo) -> impl IntoElement {
    let fill_frac = disk.fraction.clamp(0.0, 1.0);
    let action_row = disk.removable;
    let label = SharedString::from(disk.label.clone());
    let size_label = SharedString::from(disk.size_label.clone());

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
                        .child(label),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.5))
                        .text_color(rgb(0x6c_70_86))
                        .child(size_label),
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
            let block = disk.block_path.clone();
            let drive = disk.drive_path.clone();
            let mounted = disk.mount_point.is_some();
            d.child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(disk_action(
                        ElementId::Name(
                            format!("disk-mount-{}", disk.block_path).into(),
                        ),
                        "монтировать",
                        !mounted,
                        {
                            let block = block.clone();
                            move |_, _, cx: &mut App| {
                                tracing::info!("side_panel_right: disk mount {block}");
                                AppState::disks(cx).dispatch(DisksCommand::Mount {
                                    block_path: block.clone(),
                                });
                            }
                        },
                    ))
                    .child(disk_action(
                        ElementId::Name(
                            format!("disk-umount-{}", disk.block_path).into(),
                        ),
                        "размонт.",
                        mounted,
                        {
                            let block = block.clone();
                            move |_, _, cx: &mut App| {
                                tracing::info!("side_panel_right: disk unmount {block}");
                                AppState::disks(cx).dispatch(DisksCommand::Unmount {
                                    block_path: block.clone(),
                                });
                            }
                        },
                    ))
                    .child(disk_action(
                        ElementId::Name(
                            format!("disk-eject-{}", disk.block_path).into(),
                        ),
                        "извлечь",
                        drive.is_some(),
                        {
                            let drive = drive.clone();
                            move |_, _, cx: &mut App| {
                                let Some(drive_path) = drive.clone() else {
                                    tracing::debug!(
                                        "side_panel_right: eject ignored — no drive path"
                                    );
                                    return;
                                };
                                tracing::info!("side_panel_right: disk eject {drive_path}");
                                AppState::disks(cx).dispatch(DisksCommand::Eject { drive_path });
                            }
                        },
                    )),
            )
        })
}

fn disk_action(
    id: ElementId,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .py(px(4.))
        .rounded(px(5.))
        .border_1()
        .border_color(rgb(0x45_47_5a))
        .text_size(px(10.))
        .text_color(if enabled {
            rgb(0xa6_ad_c8)
        } else {
            rgb(0x6c_70_86)
        })
        .flex()
        .items_center()
        .justify_center()
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(rgb(0x23_23_36)))
                .on_click(on_click)
        })
        .child(label)
}

/// Live list from `DisksSubscriber`. Empty → "нет дисков".
pub fn render_disks_section(
    disks: &[DiskInfo],
    _cx: &mut gpui::Context<crate::side_panel_right::view::SidePanelRightView>,
) -> impl IntoElement {
    let cards: Vec<_> = disks.iter().map(usage_card).collect();
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .when(disks.is_empty(), |d| {
            d.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x6c_70_86))
                    .child("нет дисков"),
            )
        })
        .children(cards)
}
