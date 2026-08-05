//! ACP agents tab — list agent backends (T196).
//!
//! Reads agents via `hermes_acp::registry::known_agents()`, which merges
//! built-in (Hermes) + `~/.config/chronos/agents.toml`.
//!
//! Open: views agents.toml in the Editor (View-only — plain-text `.toml`
//! is out of scope for the Preview/Edit dual toggle, T194c). Reload:
//! re-queries the registry. Add/remove is done by editing the TOML
//! directly with an external editor — pragmatic MVP.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, SharedString, Styled, Window, div, prelude::*, px,
};

use chronos_services::hermes_acp::registry::known_agents;
use chronos_ui::Theme;
use crate::side_panel_right::preview_target::{PreviewIntent, PreviewTarget};
use super::ui;

// T249/T255: the ACP header is `py(12)*2 + 13px title + gap(2) + text_xs
// subtitle (line 1) + gap(2) + text_xs subtitle (line 2) + 1px border`
// ≈ 78px. Used to floor the card height to the scroll viewport bottom
// (see `render`). Two mono subtitle lines: `{n} agent(s) · agents.toml`
// and `local only · no network · no telemetry`.
const HEADER_H_PX: f32 = 78.0;
/// T249: scroll container vertical padding `p(14)*2`.
const SCROLL_PADDING_TOTAL: f32 = 28.0;

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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let is_wide = ui::is_wide(window);

        // T249: stretch the elevated card to the bottom of the scroll
        // viewport so a short content set (few agents) doesn't leave a
        // naked void below it. GPUI measures scroll content at unbounded
        // height, so `relative()`/flex-grow minima don't resolve inside
        // overflow containers — the floor must be explicit px. Long
        // content outgrows the floor and scrolls as before.
        let min_card_h = (window.bounds().size.height.as_f32()
            - HEADER_H_PX
            - SCROLL_PADDING_TOTAL)
            .max(0.0);

        let open_file = cx.listener(move |this, _ev, _w, cx| {
            cx.set_global(PreviewTarget {
                path: Some(agents_path()),
                generation: 1,
                // `.toml` is Text-kind and T194c scopes the Preview/Edit
                // dual toggle to Markdown only — an Edit intent here would
                // silently downgrade to View anyway (`apply_intent`). Ask
                // for what will actually happen so the label below is true.
                //
                // T212 residual: a same-path re-open (this button clicked
                // twice) never re-reads disk regardless of `generation`
                // (deliberate T194c contract — see the comment on the fast
                // path in `preview.rs::on_target_changed`). If the file was
                // edited or deleted externally since the last View, this
                // shows the stale in-memory copy until a different path is
                // loaded first. Not fixed here — that contract is shared
                // with Files/Follow and is a bigger call than this tab.
                intent: PreviewIntent::View,
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

        // T231-pattern header: semibold title + mono subtitle (T255: two lines).
        let header = div()
            .id("acp-settings-header")
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
                    .child("ACP agents"),
            )
            .child(
                div()
                    .text_color(theme.text.muted)
                    .text_xs()
                    .font_family(theme.font_mono)
                    .child(format!("{} agent(s) · agents.toml", agents_snapshot.len())),
            )
            .child(
                div()
                    .text_color(theme.text.muted)
                    .text_xs()
                    .font_family(theme.font_mono)
                    .child("local only · no network · no telemetry"),
            );

        // Content — inside the elevated card (T231 §5 pattern).
        let mut card = ui::elevated_card(theme).id("acp-settings-card");

        // ── Configured agents ───────────────────────────────────────────
        card = card.child(ui::section_header(
            theme,
            "Configured agents",
            "ACP-compatible backends · built-in + ~/.config/chronos/agents.toml",
        ));
        let mut rows: Vec<AnyElement> = Vec::new();
        if agents_snapshot.is_empty() {
            rows.push(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(9.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border.subtle)
                    .text_color(theme.text.muted)
                    .text_xs()
                    .child("No agents configured. Click Open agents.toml below.")
                    .into_any_element(),
            );
        }
        for a in &agents_snapshot {
            rows.push(
                div()
                    .id(SharedString::from(format!("agent-{}", a.id)))
                    .w_full()
                    .flex_col()
                    .px(px(12.))
                    .py(px(9.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border.subtle)
                    .gap(px(6.))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_color(theme.text.primary)
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(a.display_name.clone()),
                                    )
                                    .when(a.builtin, |d| {
                                        d.child(
                                            div()
                                                .px(px(5.))
                                                .py(px(1.))
                                                .rounded(px(3.))
                                                .text_xs()
                                                .text_color(theme.accent.primary)
                                                .bg(theme.accent.primary.opacity(0.12))
                                                .child("built-in"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_color(theme.text.muted)
                                    .text_xs()
                                    .font_family(theme.font_mono)
                                    .child(a.id.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_color(theme.text.muted)
                            .text_xs()
                            .font_family(theme.font_mono)
                            .child(format!("{} {}", a.command, a.args.join(" "))),
                    )
                    .into_any_element(),
            );
        }
        card = card.child(
            div()
                .grid()
                .w_full()
                .gap(px(8.))
                .when(is_wide && rows.len() > 1, |d| d.grid_cols(2))
                .when(!is_wide || rows.len() <= 1, |d| d.grid_cols(1))
                .children(rows),
        );

        // ── Actions ─────────────────────────────────────────────────────
        card = card.child(ui::section_header(
            theme,
            "Actions",
            "View agents.toml (edit it externally), then Reload",
        ));
        card = card.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .id("acp-open-file")
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .justify_between()
                        .items_center()
                        .gap(px(8.))
                        .px(px(12.))
                        .py(px(9.))
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.interactive.hover))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .text_color(theme.text.primary)
                                        .text_size(px(12.))
                                        .child("Open agents.toml"),
                                )
                                .child(
                                    div()
                                        .text_color(theme.text.muted)
                                        .text_xs()
                                        .font_family(theme.font_mono)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child("~/.config/chronos/agents.toml"),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.accent.primary)
                                .text_size(px(12.))
                                .child("View"),
                        )
                        .on_click(open_file),
                )
                // Reload is `flex_none` so a long path in the sibling above can
                // never push it out of the 320px settings-width viewport (T212 —
                // S7 in the T209 live smoke read this as "missing"; it was clipped,
                // not absent — `flex_1` without `min_w(0.)` let the text grow
                // unbounded instead of eliding).
                .child(
                    div()
                        .id("acp-reload")
                        .flex_none()
                        .px(px(12.))
                        .py(px(9.))
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border.subtle)
                        .cursor_pointer()
                        .text_color(theme.text.secondary)
                        .text_size(px(12.))
                        .hover(|s| s.bg(theme.interactive.hover))
                        .child("Reload")
                        .on_click(reload_h),
                ),
        );

        // ── Error banner ────────────────────────────────────────────────
        let card = card.when_some(error_snapshot, |d, e| {
            d.child(
                div()
                    .w_full()
                    .px(px(10.))
                    .py(px(8.))
                    .rounded_md()
                    .border_1()
                    .border_color(theme.status.error)
                    .text_color(theme.status.error)
                    .text_xs()
                    .child(e),
            )
        });

        // ── Example ─────────────────────────────────────────────────────
        let card = card.child(
            div()
                .w_full()
                .flex_col()
                .gap(px(4.))
                .child(div().text_color(theme.text.muted).text_xs().child("Example entry:"))
                .child(
                    div()
                        .w_full()
                        .px(px(10.))
                        .py(px(8.))
                        .rounded_md()
                        .bg(theme.interactive.hover)
                        .text_color(theme.text.muted)
                        .text_xs()
                        .font_family(theme.font_mono)
                        .child(
                            "[[agents]]\nid = \"my-agent\"\ndisplay_name = \"My Agent\"\ncommand = \"/path/to/agent\"\nargs = [\"acp\"]",
                        ),
                ),
        );

        div()
            .id("acp-settings-tab")
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("acp-settings-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .p(px(14.))
                    .child(card.min_h(px(min_card_h))),
            )
    }
}

