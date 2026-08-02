//! Bar appearance — `[appearance]` section of `bar.toml` (T199).
//!
//! Schema v2 of `~/.config/chronos/bar.toml`. This module is **schema-only**:
//! parsing, defaults, sanitize (clamps + floating⇒!exclusive). Applying the
//! values to the layer-shell window (resize / re-anchor / margin) is T200 and
//! intentionally does NOT live here.
//!
//! Compat: v1 files (no `version`, flat widgets) keep loading — appearance
//! falls back to code defaults via the version gate in `layout_config::load`.

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::bar::BAR_HEIGHT;

/// Bar edge. `Left`/`Right` parse and store (vertical bar is later) — only
/// `Top`/`Bottom` are meaningful for T200 apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarEdge {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl FromStr for BarEdge {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "top" => Ok(Self::Top),
            "bottom" => Ok(Self::Bottom),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(()),
        }
    }
}

impl<'de> Deserialize<'de> for BarEdge {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_choice(d)
    }
}

/// Bar horizontal alignment (meaningful when `width != full`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarAlign {
    Start,
    #[default]
    Center,
    End,
}

impl FromStr for BarAlign {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "start" => Ok(Self::Start),
            "center" => Ok(Self::Center),
            "end" => Ok(Self::End),
            _ => Err(()),
        }
    }
}

impl<'de> Deserialize<'de> for BarAlign {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_choice(d)
    }
}

/// Bar depth tier — maps to `ElevationTokens` in T200.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarElevation {
    #[default]
    None,
    Soft,
    Strong,
}

impl FromStr for BarElevation {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "soft" => Ok(Self::Soft),
            "strong" => Ok(Self::Strong),
            _ => Err(()),
        }
    }
}

impl<'de> Deserialize<'de> for BarElevation {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_choice(d)
    }
}

/// Bar width mode. String form in TOML: `"full"` | `"hug"` | `"fraction:0.7"`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BarWidth {
    #[default]
    Full,
    /// Fraction of the display width (clamped to 0.2..=1.0 by sanitize).
    Fraction(f32),
    /// Content-measured width — schema accepts it; T200 may no-op.
    Hug,
}

impl Serialize for BarWidth {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            BarWidth::Full => s.serialize_str("full"),
            BarWidth::Hug => s.serialize_str("hug"),
            BarWidth::Fraction(f) => s.serialize_str(&format!("fraction:{f}")),
        }
    }
}

impl<'de> Deserialize<'de> for BarWidth {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        match raw.as_str() {
            "full" => Ok(BarWidth::Full),
            "hug" => Ok(BarWidth::Hug),
            _ => match raw.strip_prefix("fraction:") {
                Some(rest) => match rest.parse::<f32>() {
                    Ok(f) => Ok(BarWidth::Fraction(f)),
                    Err(_) => {
                        tracing::warn!("bar: bad fraction '{raw}', using full");
                        Ok(BarWidth::Full)
                    }
                },
                None => {
                    tracing::warn!("bar: unknown width '{raw}', using full");
                    Ok(BarWidth::Full)
                }
            },
        }
    }
}

/// Margins around the bar surface (used when floating / inset).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarMargin {
    pub x: f32,
    pub y: f32,
}

impl Default for BarMargin {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Sanitize bounds (documented here — keep in sync with tests).
const HEIGHT_MIN: f32 = 20.0;
const HEIGHT_MAX: f32 = 80.0;
const RADIUS_MIN: f32 = 0.0;
const RADIUS_MAX: f32 = 24.0;
const FRACTION_MIN: f32 = 0.2;
const FRACTION_MAX: f32 = 1.0;

/// Bar appearance — defaults mirror the hardcoded chrome (T198 table).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarAppearance {
    pub edge: BarEdge,
    /// Bar thickness in px.
    pub height: f32,
    pub width: BarWidth,
    pub align: BarAlign,
    pub margin: BarMargin,
    pub floating: bool,
    /// Reserve exclusive zone (forced off when floating — see sanitize).
    pub exclusive: bool,
    /// Corner radius in px; >0 implies clip (T200 render side).
    pub radius: f32,
    pub elevation: BarElevation,
}

impl Default for BarAppearance {
    fn default() -> Self {
        Self {
            edge: BarEdge::Top,
            height: BAR_HEIGHT,
            width: BarWidth::Full,
            align: BarAlign::Center,
            margin: BarMargin::default(),
            floating: false,
            exclusive: true,
            radius: 0.0,
            elevation: BarElevation::None,
        }
    }
}

impl BarAppearance {
    /// Clamp ranges and force invariants. Never panics; `tracing::warn!` on
    /// every deviation. Idempotent — safe to re-run on a cached value.
    pub fn sanitized(self) -> Self {
        let mut out = self;

        if !(HEIGHT_MIN..=HEIGHT_MAX).contains(&out.height) {
            tracing::warn!(
                height = out.height,
                min = HEIGHT_MIN,
                max = HEIGHT_MAX,
                "bar: appearance height out of range, clamping"
            );
            out.height = out.height.clamp(HEIGHT_MIN, HEIGHT_MAX);
        }

        if !(RADIUS_MIN..=RADIUS_MAX).contains(&out.radius) {
            tracing::warn!(
                radius = out.radius,
                min = RADIUS_MIN,
                max = RADIUS_MAX,
                "bar: appearance radius out of range, clamping"
            );
            out.radius = out.radius.clamp(RADIUS_MIN, RADIUS_MAX);
        }

        match &mut out.width {
            BarWidth::Fraction(f) if !(FRACTION_MIN..=FRACTION_MAX).contains(f) => {
                tracing::warn!(
                    fraction = *f,
                    min = FRACTION_MIN,
                    max = FRACTION_MAX,
                    "bar: appearance width fraction out of range, clamping"
                );
                *f = f.clamp(FRACTION_MIN, FRACTION_MAX);
            }
            _ => {}
        }

        if out.margin.x < 0.0 {
            tracing::warn!(x = out.margin.x, "bar: appearance margin.x negative, zeroing");
            out.margin.x = 0.0;
        }
        if out.margin.y < 0.0 {
            tracing::warn!(y = out.margin.y, "bar: appearance margin.y negative, zeroing");
            out.margin.y = 0.0;
        }

        // A floating bar must not reserve an exclusive zone — otherwise the
        // compositor reserves space from the (non-edge) anchor incorrectly.
        if out.floating && out.exclusive {
            tracing::warn!("bar: floating=true forces exclusive=false");
            out.exclusive = false;
        }

        out
    }

    /// Serialization guard: skip writing `[appearance]` when it equals
    /// defaults — keeps v1 files byte-stable on save.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Generic enum-from-string with warn-on-unknown → default. Used by the
/// choice enums so a bad **value** degrades per-field instead of failing the
/// file. A bad **type** (e.g. `width = 5`) still hard-fails the whole file
/// parse — `layout_config::load` then warns and falls back to defaults
/// (mirrors the existing `BarLayoutConfig::load` contract).
fn deserialize_choice<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr<Err = ()> + Default,
{
    let raw = String::deserialize(d)?;
    match T::from_str(&raw) {
        Ok(v) => Ok(v),
        Err(()) => {
            tracing::warn!("bar: unknown value '{raw}', using default");
            Ok(T::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_hardcoded_chrome() {
        let a = BarAppearance::default();
        assert_eq!(a.edge, BarEdge::Top);
        assert_eq!(a.height, BAR_HEIGHT);
        assert_eq!(a.width, BarWidth::Full);
        assert_eq!(a.align, BarAlign::Center);
        assert_eq!(a.margin, BarMargin { x: 0.0, y: 0.0 });
        assert!(!a.floating);
        assert!(a.exclusive);
        assert_eq!(a.radius, 0.0);
        assert_eq!(a.elevation, BarElevation::None);
    }

    #[test]
    fn sanitize_clamps_height() {
        let low = BarAppearance { height: 10.0, ..Default::default() }.sanitized();
        assert_eq!(low.height, HEIGHT_MIN);
        let high = BarAppearance { height: 200.0, ..Default::default() }.sanitized();
        assert_eq!(high.height, HEIGHT_MAX);
        let ok = BarAppearance { height: 30.0, ..Default::default() }.sanitized();
        assert_eq!(ok.height, 30.0);
    }

    #[test]
    fn sanitize_clamps_fraction() {
        let low = BarAppearance { width: BarWidth::Fraction(0.05), ..Default::default() }
            .sanitized();
        assert_eq!(low.width, BarWidth::Fraction(FRACTION_MIN));
        let high = BarAppearance { width: BarWidth::Fraction(2.0), ..Default::default() }
            .sanitized();
        assert_eq!(high.width, BarWidth::Fraction(FRACTION_MAX));
        let ok = BarAppearance { width: BarWidth::Fraction(0.7), ..Default::default() }.sanitized();
        assert_eq!(ok.width, BarWidth::Fraction(0.7));
    }

    #[test]
    fn sanitize_clamps_radius() {
        let high = BarAppearance { radius: 99.0, ..Default::default() }.sanitized();
        assert_eq!(high.radius, RADIUS_MAX);
        let neg = BarAppearance { radius: -3.0, ..Default::default() }.sanitized();
        assert_eq!(neg.radius, RADIUS_MIN);
    }

    #[test]
    fn sanitize_zeroes_negative_margin() {
        let a = BarAppearance {
            margin: BarMargin { x: -4.0, y: 12.0 },
            ..Default::default()
        }
        .sanitized();
        assert_eq!(a.margin.x, 0.0);
        assert_eq!(a.margin.y, 12.0);
    }

    #[test]
    fn sanitize_floating_forces_exclusive_off() {
        let a = BarAppearance {
            floating: true,
            exclusive: true,
            ..Default::default()
        }
        .sanitized();
        assert!(a.floating);
        assert!(!a.exclusive);
        // Not floating → exclusive untouched.
        let a = BarAppearance {
            floating: false,
            exclusive: true,
            ..Default::default()
        }
        .sanitized();
        assert!(a.exclusive);
    }

    #[test]
    fn parse_full_toml_appearance() {
        let toml_str = r#"
            edge = "bottom"
            height = 40
            width = "fraction:0.7"
            align = "end"
            margin = { x = 12, y = 8 }
            floating = true
            radius = 12
            elevation = "soft"
        "#;
        let a: BarAppearance = toml::from_str(toml_str).unwrap();
        assert_eq!(a.edge, BarEdge::Bottom);
        assert_eq!(a.height, 40.0);
        assert_eq!(a.width, BarWidth::Fraction(0.7));
        assert_eq!(a.align, BarAlign::End);
        assert_eq!(a.margin.x, 12.0);
        assert_eq!(a.margin.y, 8.0);
        assert!(a.floating);
        assert_eq!(a.radius, 12.0);
        assert_eq!(a.elevation, BarElevation::Soft);
    }

    #[test]
    fn parse_hug_width_accepted() {
        let a: BarAppearance = toml::from_str(r#"width = "hug""#).unwrap();
        assert_eq!(a.width, BarWidth::Hug);
    }

    #[test]
    fn parse_edge_left_right_accepted_for_future() {
        let a: BarAppearance = toml::from_str(r#"edge = "left""#).unwrap();
        assert_eq!(a.edge, BarEdge::Left);
        let a: BarAppearance = toml::from_str(r#"edge = "RIGHT""#).unwrap();
        assert_eq!(a.edge, BarEdge::Right);
    }

    #[test]
    fn unknown_values_fall_back_to_defaults() {
        let a: BarAppearance = toml::from_str(r#"edge = "middle""#).unwrap();
        assert_eq!(a.edge, BarEdge::Top);
        let a: BarAppearance = toml::from_str(r#"align = "north""#).unwrap();
        assert_eq!(a.align, BarAlign::Center);
        let a: BarAppearance = toml::from_str(r#"elevation = "hyper""#).unwrap();
        assert_eq!(a.elevation, BarElevation::None);
        let a: BarAppearance = toml::from_str(r#"width = "half""#).unwrap();
        assert_eq!(a.width, BarWidth::Full);
        let a: BarAppearance = toml::from_str(r#"width = "fraction:abc""#).unwrap();
        assert_eq!(a.width, BarWidth::Full);
    }

    #[test]
    fn empty_appearance_is_defaults() {
        let a: BarAppearance = toml::from_str("").unwrap();
        assert_eq!(a, BarAppearance::default());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let a: BarAppearance = toml::from_str("future_option = 42").unwrap();
        assert_eq!(a, BarAppearance::default());
    }

    #[test]
    fn width_serialize_roundtrip() {
        for w in [BarWidth::Full, BarWidth::Hug, BarWidth::Fraction(0.7)] {
            let s = toml::to_string(&BarAppearance {
                width: w,
                ..Default::default()
            })
            .unwrap();
            let back: BarAppearance = toml::from_str(&s).unwrap();
            assert_eq!(back.width, w);
        }
    }

    #[test]
    fn type_error_fails_whole_parse_value_error_degrades() {
        // Type errors (wrong TOML type) hard-fail the file parse → `load()`
        // warns + defaults (mirrors existing load contract). Value errors
        // degrade per-field with a warn.
        assert!(toml::from_str::<BarAppearance>(r#"width = 5"#).is_err());
        assert!(toml::from_str::<BarAppearance>(r#"margin = 7"#).is_err());
        assert!(toml::from_str::<BarAppearance>(r#"edge = "bogus""#).is_ok());
    }

    #[test]
    fn sanitize_is_idempotent() {
        let dirty = BarAppearance {
            height: 200.0,
            radius: -2.0,
            width: BarWidth::Fraction(0.05),
            floating: true,
            exclusive: true,
            ..Default::default()
        };
        let once = dirty.sanitized();
        let twice = once.sanitized();
        assert_eq!(once, twice);
    }
}
