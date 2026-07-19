//! Project-switcher pill for the bar — folder icon + active project name +
//! git branch (mono), click toggles the project popup.
//!
//! Branch text refreshes on the bar's 1-second ticker: `current_branch` is a
//! ~30-byte `.git/HEAD` read, cheap enough to do per render.

use std::path::Path;

use gpui::{AnyElement, App, Window, div, prelude::*, px, svg};

use chronos_luau::bar::{BarSection, BarWidget};
use chronos_ui::Theme;

use crate::project_switcher::{cached, current_branch};

pub struct ProjectWidget;

impl BarWidget for ProjectWidget {
    fn name(&self) -> &str {
        "project"
    }

    fn section(&self) -> BarSection {
        BarSection::Right
    }

    fn render(&self, _window: &mut Window, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let config = cached();

        let (label, branch) = match config.active_entry() {
            Some(entry) => (
                entry.name.clone(),
                current_branch(Path::new(&entry.path)),
            ),
            None => ("проект".to_string(), None),
        };
        let color = if config.active_entry().is_some() {
            theme.text.secondary
        } else {
            theme.text.muted
        };

        let mut pill = div()
            .id("bar-project")
            .flex()
            .items_center()
            .gap(px(5.))
            .cursor_pointer()
            .px(px(7.))
            .py(px(2.))
            .rounded(theme.radius)
            .hover(|s| s.bg(theme.interactive.hover))
            .child(svg().path("icons/folder.svg").size(px(12.)).text_color(color))
            .child(
                div()
                    .child(label)
                    .text_color(color)
                    .font_family(theme.font_mono)
                    .text_size(theme.font_sizes.sm),
            );
        if let Some(branch) = branch {
            pill = pill.child(
                div()
                    .child(branch)
                    .text_color(theme.text.muted)
                    .font_family(theme.font_mono)
                    .text_xs(),
            );
        }
        pill.child(
            svg()
                .path("icons/chevron-down.svg")
                .size(px(9.))
                .text_color(theme.text.muted),
        )
        .on_click(|_event, window, cx: &mut App| {
            crate::project_switcher::toggle(window, cx);
        })
        .into_any_element()
    }
}

/// Register the project-switcher pill with the global bar registry.
pub fn register(cx: &mut App) {
    cx.global_mut::<chronos_luau::bar::BarWidgetRegistry>()
        .register(Box::new(ProjectWidget));
}
