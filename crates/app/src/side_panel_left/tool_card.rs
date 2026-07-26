use gpui::{div, prelude::*, px};
use chronos_ui::Theme;

pub struct ToolCard<'a> {
    pub name: &'a str,
    pub status: &'a str,
    pub args: Option<&'a str>,
    pub result: Option<&'a str>,
    pub expanded: bool,
    pub theme: &'a Theme,
}

impl<'a> ToolCard<'a> {
    pub fn render<F>(&self, on_click: Option<F>) -> impl IntoElement
    where
        F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    {
        let theme = self.theme;
        let status_color = match self.status {
            "running" => theme.status.warning,
            "done" => theme.status.success,
            "error" => theme.status.error,
            _ => theme.interactive.active,
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
            .px(px(10.))
            .py(px(7.))
            .rounded(px(8.))
            .cursor_pointer()
            .hover(|s| s.bg(theme.bg.elevated));

        if let Some(handler) = on_click {
            header = header.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        let header = header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(status_color))
                    .child(
                        div()
                            .font_family(theme.font_mono)
                            .text_size(px(10.5))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text.primary)
                            .child(self.name.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme.text.muted)
                            .child(status_label.to_string()),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme.interactive.active)
                    .child(toggle_icon),
            );

        let mut card = div()
            .rounded(px(8.))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .child(header);

        if self.expanded {
            let mut details = div()
                .px(px(10.))
                .py(px(7.))
                .border_t_1()
                .border_color(theme.border.default)
                .flex()
                .flex_col()
                .gap(px(4.));

            if let Some(args) = self.args {
                if !args.is_empty() {
                    details = details.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(9.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.status.info)
                                    .child("Arguments"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(theme.bg.tertiary)
                                    .font_family(theme.font_mono)
                                    .text_size(px(10.))
                                    .text_color(theme.text.secondary)
                                    .child(args.to_string()),
                            ),
                    );
                }
            }

            if let Some(result) = self.result {
                if !result.is_empty() {
                    details = details.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(9.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.status.success)
                                    .child("Result"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .px(px(6.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(theme.bg.tertiary)
                                    .font_family(theme.font_mono)
                                    .text_size(px(10.))
                                    .text_color(theme.text.secondary)
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
