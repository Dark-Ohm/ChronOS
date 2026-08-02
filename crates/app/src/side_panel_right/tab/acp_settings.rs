//! ACP agents tab — list agent backends (T196).
//!
//! Reads agents via `hermes_acp::registry::known_agents()`, which merges
//! built-in (Hermes) + `~/.config/chronos/agents.toml`.
//!
//! Edit: opens agents.toml in Editor. Reload: re-queries the registry.
//! Add/remove is done by editing the TOML directly — pragmatic MVP.

use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollHandle, SharedString, Styled, Window, div, prelude::*, px,
};

use chronos_services::hermes_acp::registry::known_agents;
use chronos_ui::Theme;
use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};

// ---------------------------------------------------------------------------
// Agent display wrapper
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AgentRow {
    id: String,
    display_name: String,
    command: String,
    args: Vec<String>,
    builtin: bool,
}

fn agents_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("chronos/agents.toml")
}

fn is_builtin(id: &str) -> bool {
    id == "hermes"
}

// ---------------------------------------------------------------------------
// Tab entity
// ---------------------------------------------------------------------------

pub struct AcpSettingsTab {
    agents: Vec<AgentRow>,
    error: Option<String>,
    scroll: ScrollHandle,
}

impl AcpSettingsTab {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        let (agents, error) = Self::load();
        Self { agents, error, scroll: ScrollHandle::new() }
    }

    fn load() -> (Vec<AgentRow>, Option<String>) {
        let descriptors = known_agents();
        if descriptors.is_empty() {
            return (Vec::new(), Some("No agents available — check hermes installation".to_string()));
        }
        let agents: Vec<AgentRow> = descriptors
            .into_iter()
            .map(|d| {
                let builtin = is_builtin(&d.id);
                AgentRow { id: d.id, display_name: d.display_name, command: d.config.command, args: d.config.args, builtin }
            })
            .collect();
        (agents, None)
    }

    fn reload(&mut self) {
        let (agents, error) = Self::load();
        self.agents = agents;
        self.error = error;
    }
}

impl Render for AcpSettingsTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);

        let open_file = cx.listener(move |this, _ev, _w, cx| {
            cx.set_global(PreviewTarget {
                path: Some(agents_path()),
                generation: 1,
                intent: PreviewIntent::Edit,
            });
            this.error = None;
            cx.notify();
        });

        let reload_h = cx.listener(move |this, _ev, _w, cx| {
            this.reload();
            cx.notify();
        });

        let agents_snapshot = self.agents.clone();
        let error_snapshot = self.error.clone();

        div().id("acp-settings-tab").size_full().flex().flex_col()
            .child(div().w_full().px(px(14.)).py(px(12.)).border_b_1().border_color(theme.border.default).flex().flex_col().gap(px(2.))
                .child(div().text_color(theme.text.primary).text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("ACP agents"))
                .child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(format!("{} agent(s) · agents.toml", agents_snapshot.len()))))
            .child(
                div().id("acp-settings-scroll").flex_1().min_h(px(0.)).overflow_y_scroll().track_scroll(&self.scroll).flex().flex_col().gap(px(14.)).p(px(14.))
                    .child(div().w_full().flex_col().gap(px(2.))
                        .child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child("Configured agents"))
                        .child(div().text_color(theme.text.muted).text_xs().child("ACP-compatible backends. Edit agents.toml to add or remove.")))
                    .child({
                        let mut rows: Vec<gpui::AnyElement> = Vec::new();
                        if agents_snapshot.is_empty() {
                            rows.push(div().w_full().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).text_color(theme.text.muted).text_xs().child("No agents configured. Click Open agents.toml below.").into_any_element());
                        }
                        for a in &agents_snapshot {
                            rows.push(
                                div().id(SharedString::from(format!("agent-{}", a.id)))
                                    .w_full().flex_col().px(px(12.)).py(px(9.)).rounded_md()
                                    .border_1().border_color(theme.border.subtle).gap(px(6.))
                                    .child(div().w_full().flex().justify_between().items_center()
                                        .child(div().flex().items_center().gap(px(8.))
                                            .child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(FontWeight::MEDIUM).child(a.display_name.clone()))
                                            .when(a.builtin, |d| d.child(div().px(px(5.)).py(px(1.)).rounded(px(3.)).text_xs().text_color(theme.accent.primary).bg(theme.accent.primary.opacity(0.12)).child("built-in"))))
                                        .child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(a.id.clone())))
                                    .child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child(format!("{} {}", a.command, a.args.join(" "))))
                                    .into_any_element(),
                            );
                        }
                        div().w_full().flex_col().gap(px(4.)).children(rows)
                    })
                    .child(div().w_full().flex_col().gap(px(2.))
                        .child(div().text_color(theme.text.primary).text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child("Actions"))
                        .child(div().text_color(theme.text.muted).text_xs().child("Edit agents.toml to add/remove agents, then reload.")))
                    .child(div().w_full().flex().gap(px(8.))
                        .child(div().id("acp-open-file").flex_1().flex().justify_between().items_center().px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).cursor_pointer().hover(|s| s.bg(theme.interactive.hover))
                            .child(div().flex_col().gap(px(1.)).child(div().text_color(theme.text.primary).text_size(px(12.)).child("Open agents.toml")).child(div().text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("~/.config/chronos/agents.toml")))
                            .child(div().text_color(theme.accent.primary).text_size(px(12.)).child("Edit"))
                            .on_click(open_file))
                        .child(div().id("acp-reload").px(px(12.)).py(px(9.)).rounded_md().border_1().border_color(theme.border.subtle).cursor_pointer().text_color(theme.text.secondary).text_size(px(12.)).hover(|s| s.bg(theme.interactive.hover)).child("Reload").on_click(reload_h)))
                    .when_some(error_snapshot, |d, e| {
                        d.child(div().w_full().px(px(10.)).py(px(8.)).rounded_md().border_1().border_color(theme.status.error).text_color(theme.status.error).text_xs().child(e))
                    })
                    .child(div().w_full().flex_col().gap(px(2.))
                        .child(div().text_color(theme.text.muted).text_xs().child("Example entry:"))
                        .child(div().w_full().px(px(10.)).py(px(8.)).rounded_md().bg(theme.interactive.hover).text_color(theme.text.muted).text_xs().font_family(theme.font_mono).child("[[agents]]\nid = \"my-agent\"\ndisplay_name = \"My Agent\"\ncommand = \"/path/to/agent\"\nargs = [\"acp\"]")))
            )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
