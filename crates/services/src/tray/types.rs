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

/// Raw RGBA icon pixmap carried by `StatusNotifierItem.IconPixmap`
/// (wire signature `(iiay)`): width, height, and pixel data. ARGB→RGBA is
/// already applied in [`crate::tray::TraySubscriber::add_item`]; the widget
/// converts RGBA→BGRA when building a GPUI `RenderImage` (gpui stores decoded
/// images in BGRA — see `Source/gpui/src/assets.rs`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrayPixmap {
    /// Pixmap width in pixels.
    pub width: u32,
    /// Pixmap height in pixels.
    pub height: u32,
    /// RGBA pixel data (`width * height * 4` bytes).
    pub data: Vec<u8>,
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
    /// Icon pixmap (`StatusNotifierItem.IconPixmap`), if provided. Holds the
    /// largest available pixmap as ready-for-GPU RGBA (`TrayPixmap`: ARGB→RGBA
    /// conversion already done in `TraySubscriber::add_item`).
    pub icon_pixmap: Option<TrayPixmap>,
    /// Convenience derived label for the text fallback badge.
    pub label: String,
    /// Object path to the `com.canonical.dbusmenu` interface
    /// (`StatusNotifierItem.Menu` property), if the application exposes one.
    pub menu_path: Option<String>,
    /// Fetched menu tree (populated by `FetchMenu` command).
    pub menu: Option<Vec<MenuNode>>,
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

/// Strip mnemonic underscores from a D-Bus menu label. The spec says labels
/// may contain underscores as keyboard mnemonic markers (e.g. "_Quit",
/// "E_xit"); the UI should display them without the leading underscore. A
/// double underscore `__` represents a literal underscore.
pub fn strip_mnemonic(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Toggle type for checkable menu items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuToggleType {
    /// Standard checkbox.
    Checkmark,
    /// Radio button (mutually exclusive within a group).
    Radio,
}

/// A parsed node in a DBusMenu tree (result of `GetLayout`).
///
/// The tree is recursive: leaf nodes have an empty `children` vec.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuNode {
    /// Menu item id (positive integer, unique within the menu).
    pub id: i32,
    /// Display label (mnemonic underscores already stripped).
    pub label: String,
    /// Whether the item can be activated.
    pub enabled: bool,
    /// Whether the item should be shown.
    pub visible: bool,
    /// If true, this item is a visual separator (label/children are ignored).
    pub separator: bool,
    /// Toggle state for checkable items: `(toggle_type, checked)`.
    pub toggle: Option<(MenuToggleType, bool)>,
    /// Child items (empty for leaf nodes).
    pub children: Vec<MenuNode>,
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
    /// Fetch the menu tree from the item's `com.canonical.dbusmenu` interface.
    /// The result is written to `TrayItem.menu` in reactive state.
    FetchMenu { service: String },
    /// Activate a menu item by id (`com.canonical.dbusmenu.Event(clicked)`).
    MenuClicked { service: String, id: i32 },
}