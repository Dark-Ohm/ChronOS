use gpui::{div, prelude::*, px, rgb};

pub struct ToolCard<'a> {
    pub name: &'a str,
    pub status: &'a str,
    pub args: Option<&'a str>,
    pub result: Option<&'a str>,
    pub expanded: bool,
}

impl<'a> ToolCard<'a> {
    pub fn render<F>(&self, on_click: Option<F>) -> impl IntoElement
    where
        F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    {
        let status_color = match self.status {
            "running" => rgb(0xf9_e2_af),
            "done" => rgb(0xa6_e3_a1),
            "error" => rgb(0xf3_8b_a8),
            _ => rgb(0x58_5b_70),
        };

        let toggle_icon = if self.expanded { "▾" } else { "▸" };
        let status_label = match self.status {
            "running" => "Running",
            "done" => "Done",
            "error" => "Error",
            other => other,
        };

        let mut header = div()
            .id(format!("tool-card-header-{}", self.name))
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.))
            .py(px(4.))
            .rounded(px(6.))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x2a_2a_3d)));

        if let Some(handler) = on_click {
            header = header.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        let header = header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(status_color),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xcd_d6_f4))
                            .child(self.name.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(rgb(0x6c_70_86))
                            .child(status_label.to_string()),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(0x58_5b_70))
                    .child(toggle_icon),
            );

        let mut card = div()
            .mx(px(4.))
            .rounded(px(6.))
            .bg(rgb(0x1e_1e_2e))
            .border_1()
            .border_color(rgb(0x31_32_44))
            .child(header);

        if self.expanded {
            let mut details = div()
                .px(px(8.))
                .py(px(4.))
                .border_t_1()
                .border_color(rgb(0x31_32_44))
                .flex()
                .flex_col()
                .gap(px(4.));

            if let Some(args) = self.args {
                if !args.is_empty() {
                    details = details.child(
                        div().flex().flex_col().gap(px(2.)).child(
                            div()
                                .text_size(px(9.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(0x89_b4_fa))
                                .child("Arguments"),
                        )
                        .child(
                            div()
                                .w_full()
                                .px(px(6.))
                                .py(px(4.))
                                .rounded(px(4.))
                                .bg(rgb(0x18_18_25))
                                .text_size(px(9.))
                                .text_color(rgb(0xa6_ad_c8))
                                .child(args.to_string()),
                        ),
                    );
                }
            }

            if let Some(result) = self.result {
                if !result.is_empty() {
                    details = details.child(
                        div().flex().flex_col().gap(px(2.)).child(
                            div()
                                .text_size(px(9.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(0xa6_e3_a1))
                                .child("Result"),
                        )
                        .child(
                            div()
                                .w_full()
                                .px(px(6.))
                                .py(px(4.))
                                .rounded(px(4.))
                                .bg(rgb(0x18_18_25))
                                .text_size(px(9.))
                                .text_color(rgb(0xa6_ad_c8))
                                .child(result.to_string()),
                        ),
                    );
                }
            }

            card = card.child(details);
        }

        card
    }
}
