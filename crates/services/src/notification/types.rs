//! Data model for the org.freedesktop.Notifications service.
//!
//! Pure ChronOS types — written from scratch against the FDO spec
//! (https://specifications.freedesktop.org/notification-spec/). No donor code.
//!
//! Note: the `Data` type contains only integer/string/bool fields, so it
//! derives `Eq`/`Hash` freely. If a fractional field (e.g. a float position)
//! is ever added, *drop* `Eq`/`Hash` (per project float-rule) rather than
//! weakening comparisons — `Mutable`/`Service` do not require `Eq` here.

/// How the user should be disturbed by a notification.
///
/// Mirrors the FDO `urgency` hint values (0=low, 1=normal, 2=critical).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Urgency {
    Low = 0,
    Normal = 1,
    Critical = 2,
}

impl Default for Urgency {
    fn default() -> Self {
        Urgency::Normal
    }
}

impl Urgency {
    /// Parse from the raw FDO `urgency` hint (a `u8`).
    pub fn from_u8(v: u8) -> Urgency {
        match v {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }
}

/// A single live notification, as tracked by our daemon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: Urgency,
    /// `(action_key, action_label)` pairs (FDO passes them 2-by-2).
    pub actions: Vec<(String, String)>,
    /// Absolute expiry timestamp (epoch milliseconds), `None` = sticky.
    pub expire_at: Option<u64>,
}

impl Notification {
    pub fn is_expired(&self, now_ms: u64) -> bool {
        match self.expire_at {
            Some(t) => now_ms >= t,
            None => false,
        }
    }
}

/// Reason codes emitted with the `NotificationClosed` signal (FDO spec).
#[derive(Clone, Copy, Debug)]
pub enum CloseReason {
    /// The notification expired.
    Expired = 1,
    /// The user dismissed it via `CloseNotification`.
    DismissedByCall = 2,
    /// The daemon dismissed it (e.g. `DismissAll`).
    DismissedByDaemon = 3,
}

/// Reason codes emitted with `DismissAll` / `InvokeAction` bookkeeping.
///
/// FDO has no explicit `DismissAll` method, but our `ImperativeCommand`
/// surface needs a grouping discriminator; we reuse `CloseReason` semantics.
pub type DismissReason = CloseReason;

/// The reactive snapshot exposed via the `Service` trait (`Data`).
///
/// Contains every active notification keyed by id. The UI layer subscribes
/// to this and renders the popup stack; the daemon mutates it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotificationState {
    /// Monotonic next id for `Notify` calls with `replaces_id == 0`.
    pub next_id: u32,
    /// Active notifications, newest last.
    pub notifications: Vec<Notification>,
    /// Whether any notification is currently in a critical/urgent state.
    pub any_critical: bool,
}

impl NotificationState {
    pub fn by_id(&self, id: u32) -> Option<&Notification> {
        self.notifications.iter().find(|n| n.id == id)
    }

    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        let idx = self.notifications.iter().position(|n| n.id == id)?;
        Some(self.notifications.remove(idx))
    }

    pub fn recompute_flags(&mut self) {
        self.any_critical = self
            .notifications
            .iter()
            .any(|n| n.urgency == Urgency::Critical);
    }
}
