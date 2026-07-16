//! Base16 <-> [`Theme`].
//!
//! Base16 определяет 16 семантических слотов (`base00`..`base0f`): первые
//! восемь — фон/текст/границы (от тёмного к светлому), остальные восемь —
//! акценты и статусы. Здесь только статический конвертер: без matugen,
//! без вызова внешних процессов и без serde_json (отложено по заданию).

use anyhow::Result;
use gpui::Hsla;

use super::{Theme, parse_hex};

/// 16 слотов Base16-палитры.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Base16Colors {
    pub base00: Hsla,
    pub base01: Hsla,
    pub base02: Hsla,
    pub base03: Hsla,
    pub base04: Hsla,
    pub base05: Hsla,
    pub base06: Hsla,
    pub base07: Hsla,
    pub base08: Hsla,
    pub base09: Hsla,
    pub base0a: Hsla,
    pub base0b: Hsla,
    pub base0c: Hsla,
    pub base0d: Hsla,
    pub base0e: Hsla,
    pub base0f: Hsla,
}

impl Base16Colors {
    /// Строит палитру из 16 hex-строк (ровно 16: 6 или 8 hex-цифр каждая).
    pub fn from_hex(colors: &[&str; 16]) -> Result<Self> {
        let c = colors
            .iter()
            .map(|s| parse_hex(s))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            base00: c[0],
            base01: c[1],
            base02: c[2],
            base03: c[3],
            base04: c[4],
            base05: c[5],
            base06: c[6],
            base07: c[7],
            base08: c[8],
            base09: c[9],
            base0a: c[10],
            base0b: c[11],
            base0c: c[12],
            base0d: c[13],
            base0e: c[14],
            base0f: c[15],
        })
    }

    /// Маппит слоты Base16 на семантические группы [`Theme`].
    ///
    /// base00..base07 -> bg/text/border (от тёмного фона к светлому тексту),
    /// base08..base0f -> accent/status/interactive по общепринятой семантике
    /// Base16 (08 — error, 0b — success, 0c — info/cyan, 0d — info/blue,
    /// 0e — accent/violet, 09 — warning, 0a — warning/alt, 0f — active/emph).
    pub fn to_theme(&self) -> Theme {
        let mut theme = Theme::default();
        theme.bg.primary = self.base00;
        theme.bg.secondary = self.base01;
        theme.bg.tertiary = self.base02;
        theme.bg.elevated = self.base03;
        theme.text.primary = self.base07;
        theme.text.secondary = self.base06;
        theme.text.muted = self.base05;
        theme.text.disabled = self.base04;
        theme.text.placeholder = self.base04;
        theme.border.default = self.base02;
        theme.border.subtle = self.base03;
        theme.border.focused = self.base0d;
        theme.accent.primary = self.base0d;
        theme.accent.selection = self.base0d;
        theme.accent.hover = self.base0e;
        theme.interactive.default = self.base02;
        theme.interactive.hover = self.base03;
        theme.interactive.active = self.base04;
        theme.interactive.toggle_on = self.base0d;
        theme.interactive.toggle_on_hover = self.base0e;
        theme.status.error = self.base08;
        theme.status.warning = self.base09;
        theme.status.success = self.base0b;
        theme.status.info = self.base0c;
        theme.transparent.a = 0.0;
        theme
    }
}
