//! Data model for the system tray service (StatusNotifierWatcher).
//!
//! Pure ChronOS types — written from scratch against the FDO StatusNotifier
//! spec (https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/).
//! No donor code. The FDO method/signal *signatures* are the spec (not
//! copyrightable); the *logic* is ours.
//!
//! NOTE: `TrayState` intentionally does NOT derive `Eq`/`Hash`. Even though no
//! float is present today, the pixmap buffer is opaque and future fractional
//! fields would silently break `Eq`; per project float-rule we keep it
//! `PartialEq`-only. `Mutable`/`Service` do not require `Eq` here.

/// A tray item's icon source.
#[derive(Clone, Debug, PartialEq)]
pub enum TrayIcon {
    /// Icon name for lookup via the freedesktop icon theme.
    Name(String),
    /// Raw RGBA pixel data (MVP does not render pixmaps; see OPENCODE report).
    Pixmap {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
}

impl TrayIcon {
    /// Icon name if this is a [`TrayIcon::Name`].
    pub fn name(&self) -> Option<&str> {
        match self {
            TrayIcon::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Pixmap dimensions + bytes if this is a [`TrayIcon::Pixmap`].
    pub fn pixmap(&self) -> Option<(u32, u32, &[u8])> {
        match self {
            TrayIcon::Pixmap {
                width,
                height,
                data,
            } => Some((*width, *height, data)),
            _ => None,
        }
    }
}

/// A single live system tray item (a registered `StatusNotifierItem`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrayItem {
    /// Unique id = the registered service string (D-Bus destination, e.g.
    /// `:1.234` or `org.kde.StatusNotifierItem-1234-1`). Used as the key and
    /// to build the item proxy for activation.
    pub id: String,
    /// Display title (`StatusNotifierItem.Title`), if provided.
    pub title: Option<String>,
    /// Icon name (`StatusNotifierItem.IconName`), if provided.
    pub icon_name: Option<String>,
    /// Icon pixmap (`StatusNotifierItem.IconPixmap`), if provided (render
    /// deferred — see OPENCODE report). Holds the largest available pixmap.
    pub icon_pixmap: Option<Vec<u8>>,
    /// Convenience derived label for the text fallback badge.
    pub label: String,
}

impl TrayItem {
    /// Derive the text-fallback badge label: first letter of title, else of
    /// icon_name, else `?`. Uppercased.
    pub fn derive_label(title: &Option<String>, icon_name: &Option<String>) -> String {
        let src = title
            .as_ref()
            .filter(|s| !s.is_empty())
            .or_else(|| icon_name.as_ref().filter(|s| !s.is_empty()));
        match src {
            Some(s) => s.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into()),
            None => "?".into(),
        }
    }
}

/// The reactive snapshot exposed via the `Service` trait (`Data`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayState {
    /// Live tray items, newest last.
    pub items: Vec<TrayItem>,
}

impl TrayState {
    /// Find a tray item by id.
    pub fn find(&self, id: &str) -> Option<&TrayItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Number of tray items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are any tray items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Imperative commands the ChronOS-side API can issue (never part of the
/// `Service` trait — concrete methods on the subscriber, per our convention).
#[derive(Debug, Clone, PartialEq)]
pub enum TrayCommand {
    /// Left-click activation: `StatusNotifierItem.Activate(0, 0)`.
    ActivateItem { service: String },
}
