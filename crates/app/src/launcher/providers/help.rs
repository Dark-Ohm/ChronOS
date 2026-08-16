//! Help provider (`?`): the fixed list of prefix modes.

use super::{ProviderAction, ProviderResult};

pub fn results() -> Vec<ProviderResult> {
    vec![
        row("apps", "no prefix", "search applications"),
        row("shell", "> cmd", "run a command in $SHELL"),
        row("files", "/ or ~", "browse paths, Enter opens"),
        row("calc", "= expr", "evaluate, Enter copies result"),
        row("sys", "i:", "hostname / kernel / compositor"),
        row("close", "esc", "close the launcher"),
    ]
}

fn row(id: &str, label: &str, detail: &str) -> ProviderResult {
    ProviderResult {
        id: format!("help-{id}"),
        label: label.to_string(),
        detail: Some(detail.to_string()),
        glyph: '?',
        action: ProviderAction::None,
    }
}
