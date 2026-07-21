//! MPRIS media-player service via session-bus D-Bus (`org.mpris.MediaPlayer2`).
//!
//! ASYNC TEMPLATE (spec §5.1): same connect+retry shape as UPower / tray.
//! Discovery is dynamic (no fixed well-known name) — `ListNames` +
//! `NameOwnerChanged`, filtered by the `org.mpris.MediaPlayer2.` prefix.
//!
//! **Player pick:** sticky `user_pinned` while that name is still live;
//! otherwise first `"Playing"`, else first name in list order. Scroll on the
//! bar widget cycles the sticky pin via `MprisCommand::CyclePlayer`.

use std::collections::HashMap;
use std::time::Duration;

use futures_signals::signal::{Mutable, Signal};
use futures_util::stream::StreamExt;
use tokio::runtime::Handle;
use tracing::{info, warn};
use zbus::fdo::DBusProxy;
use zbus::zvariant::{Array, OwnedValue, Str, Value};
use zbus::{Connection, proxy};

use crate::Service;
use crate::ServiceStatus;
pub use types::{CycleDirection, MprisCommand, MprisState};

pub mod types;

/// Well-known name prefix for MPRIS players on the session bus.
pub const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const PLAYER_PATH: &str = "/org/mpris/MediaPlayer2";
/// Fallback poll so track changes land even if a player skips property signals.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MprisPlayer {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    /// Playback position in microseconds. Must be polled — does not fire
    /// PropertiesChanged on most players.
    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;

    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct MprisSubscriber {
    data: Mutable<MprisState>,
    status: Mutable<ServiceStatus>,
    /// Bus name of the currently selected player (`org.mpris.MediaPlayer2.*`).
    active_name: Mutable<Option<String>>,
    /// User sticky override from CyclePlayer; cleared when the name leaves the bus.
    user_pinned: Mutable<Option<String>>,
    conn: Mutable<Option<Connection>>,
    runtime: Handle,
}

impl MprisSubscriber {
    /// Non-failing, synchronous constructor (spec §5.1).
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime — `Handle::current()` requires
    /// one. `init_all()` (spec §7) calls this inside `rt.block_on`.
    pub fn new() -> Self {
        let data = Mutable::new(MprisState::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        let active_name = Mutable::new(None);
        let user_pinned = Mutable::new(None);
        let conn = Mutable::new(None);
        let handle = Handle::current();
        tokio::spawn(run(
            data.clone(),
            status.clone(),
            active_name.clone(),
            user_pinned.clone(),
            conn.clone(),
        ));
        Self {
            data,
            status,
            active_name,
            user_pinned,
            conn,
            runtime: handle,
        }
    }

    /// Fire-and-forget command against the active player + immediate re-read.
    ///
    /// `CyclePlayer` only updates the sticky pin (no D-Bus method call).
    pub fn dispatch(&self, cmd: MprisCommand) {
        let data = self.data.clone();
        let status = self.status.clone();
        let active_name = self.active_name.clone();
        let user_pinned = self.user_pinned.clone();
        let conn = self.conn.clone();
        self.runtime.spawn(async move {
            let Some(conn) = conn.get_cloned() else {
                warn!("MprisSubscriber: no session connection, drop {cmd:?}");
                return;
            };

            match &cmd {
                MprisCommand::CyclePlayer(dir) => {
                    let names = match list_mpris_names(&conn).await {
                        Ok(n) => n,
                        Err(e) => {
                            warn!("MprisSubscriber list for cycle failed: {e:?}");
                            return;
                        }
                    };
                    let current = active_name.get_cloned();
                    if let Some(new_pin) =
                        cycle_player_names(&names, current.as_deref(), *dir)
                    {
                        info!(
                            "MprisSubscriber cycle {:?} → pin={new_pin} (n={})",
                            dir,
                            names.len()
                        );
                        user_pinned.set(Some(new_pin));
                    }
                }
                other => {
                    let Some(name) = active_name.get_cloned() else {
                        warn!("MprisSubscriber: no active player, drop {other:?}");
                        return;
                    };
                    if let Err(e) = apply_command(&conn, &name, other).await {
                        warn!("MprisSubscriber command failed ({other:?} on {name}): {e:?}");
                        return;
                    }
                }
            }

            match read_active_state(&conn, user_pinned.get_cloned()).await {
                Ok((state, active, pin_clear)) => {
                    data.set(state);
                    active_name.set(active);
                    if pin_clear {
                        user_pinned.set(None);
                    }
                    status.set(ServiceStatus::Available);
                }
                Err(e) => {
                    warn!("MprisSubscriber re-read after command failed: {e:?}");
                }
            }
        });
    }
}

impl Default for MprisSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for MprisSubscriber {
    type Data = MprisState;
    type Error = anyhow::Error;

    fn subscribe(&self) -> impl Signal<Item = MprisState> + Unpin + 'static {
        self.data.signal_cloned()
    }

    fn get(&self) -> MprisState {
        self.data.get_cloned()
    }

    fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

/// True when `name` is an MPRIS well-known player name.
pub fn is_mpris_name(name: &str) -> bool {
    name.starts_with(MPRIS_PREFIX)
}

/// Short id for UI: suffix after `org.mpris.MediaPlayer2.`, else full name.
pub fn short_player_id(name: &str) -> String {
    name.strip_prefix(MPRIS_PREFIX)
        .unwrap_or(name)
        .to_string()
}

/// Auto selection: first `"Playing"`, else first entry. Empty list → `None`.
pub fn select_active_player(players: &[(String, String)]) -> Option<String> {
    players
        .iter()
        .find(|(_, status)| status.eq_ignore_ascii_case("Playing"))
        .or_else(|| players.first())
        .map(|(name, _)| name.clone())
}

/// Sticky resolve: pin wins while still live; otherwise auto-select.
///
/// Returns `(active_name, clear_pin)` — `clear_pin` is true when a pin was set
/// but that name is no longer on the bus (caller should drop the pin).
pub fn resolve_active_player(
    players: &[(String, String)],
    user_pinned: Option<&str>,
) -> (Option<String>, bool) {
    if players.is_empty() {
        return (None, user_pinned.is_some());
    }
    if let Some(pin) = user_pinned {
        if players.iter().any(|(n, _)| n == pin) {
            return (Some(pin.to_string()), false);
        }
        // Pin gone — fall back to auto and ask caller to clear sticky.
        return (select_active_player(players), true);
    }
    (select_active_player(players), false)
}

/// Cycle sticky selection through live bus names (wrap-around).
///
/// 0–1 players → `None` (no-op). Uses `current` as the pivot when present in
/// the list; otherwise starts from index 0.
pub fn cycle_player_names(
    names: &[String],
    current: Option<&str>,
    direction: CycleDirection,
) -> Option<String> {
    if names.len() <= 1 {
        return None;
    }
    let idx = current
        .and_then(|a| names.iter().position(|n| n == a))
        .unwrap_or(0);
    let next = match direction {
        CycleDirection::Next => (idx + 1) % names.len(),
        CycleDirection::Prev => {
            if idx == 0 {
                names.len() - 1
            } else {
                idx - 1
            }
        }
    };
    Some(names[next].clone())
}

/// Strip one layer of D-Bus variant wrapping (`v` → inner), as in tray/menu.
fn unwrap_variant(v: Value<'static>) -> Value<'static> {
    match v {
        Value::Value(boxed) => *boxed,
        other => other,
    }
}

/// Extract `xesam:title` (Str) + first `xesam:artist` (Array of Str).
pub fn parse_metadata(map: &HashMap<String, OwnedValue>) -> (String, String) {
    let title = extract_string(map, "xesam:title").unwrap_or_default();
    let artist = extract_artist(map).unwrap_or_default();
    (title, artist)
}

/// `mpris:artUrl` (Str). Empty string → `None`.
pub fn extract_art_url(map: &HashMap<String, OwnedValue>) -> Option<String> {
    let s = extract_string(map, "mpris:artUrl")?;
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `mpris:length` in microseconds. Missing / non-positive → `None`.
///
/// Players emit `x` (i64) or sometimes `t`/`i`/`u` wrapped in a variant.
pub fn extract_length_us(map: &HashMap<String, OwnedValue>) -> Option<i64> {
    let n = extract_i64(map, "mpris:length")?;
    if n > 0 { Some(n) } else { None }
}

/// True when the UI can render a meaningful progress bar.
pub fn has_position(position_us: Option<i64>, length_us: Option<i64>) -> bool {
    matches!((position_us, length_us), (Some(_), Some(len)) if len > 0)
}

fn extract_string(map: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let owned = map.get(key)?;
    let value = unwrap_variant(Value::from(owned.try_clone().ok()?));
    match value {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

fn extract_i64(map: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    let owned = map.get(key)?;
    let value = unwrap_variant(Value::from(owned.try_clone().ok()?));
    match value {
        Value::I64(n) => Some(n),
        Value::U64(n) => i64::try_from(n).ok(),
        Value::I32(n) => Some(i64::from(n)),
        Value::U32(n) => Some(i64::from(n)),
        _ => None,
    }
}

fn extract_artist(map: &HashMap<String, OwnedValue>) -> Option<String> {
    let owned = map.get("xesam:artist")?;
    let value = unwrap_variant(Value::from(owned.try_clone().ok()?));
    match value {
        Value::Str(s) => Some(s.to_string()),
        Value::Array(arr) => {
            for item in arr.iter() {
                let item = unwrap_variant(item.try_to_owned().ok()?.into());
                if let Value::Str(s) = item {
                    return Some(s.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

async fn apply_command(conn: &Connection, name: &str, cmd: &MprisCommand) -> anyhow::Result<()> {
    let proxy = player_proxy(conn, name).await?;
    match cmd {
        MprisCommand::PlayPause => proxy.play_pause().await?,
        MprisCommand::Next => proxy.next().await?,
        MprisCommand::Previous => proxy.previous().await?,
        MprisCommand::CyclePlayer(_) => {
            // Handled in dispatch before apply_command is called.
        }
    }
    Ok(())
}

async fn player_proxy<'a>(
    conn: &'a Connection,
    name: &'a str,
) -> anyhow::Result<MprisPlayerProxy<'a>> {
    let builder = MprisPlayerProxy::builder(conn)
        .destination(name)?
        .path(PLAYER_PATH)?;
    Ok(builder.build().await?)
}

/// List MPRIS bus names on the session connection.
async fn list_mpris_names(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let dbus = DBusProxy::new(conn).await?;
    let names = dbus.list_names().await?;
    Ok(names
        .into_iter()
        .map(|n| n.to_string())
        .filter(|n| is_mpris_name(n))
        .collect())
}

/// Read PlaybackStatus for each known name; pick active (sticky or auto); load metadata.
///
/// Returns `(state, active_name, clear_pin)`.
async fn read_active_state(
    conn: &Connection,
    user_pinned: Option<String>,
) -> anyhow::Result<(MprisState, Option<String>, bool)> {
    let names = list_mpris_names(conn).await?;
    if names.is_empty() {
        return Ok((
            MprisState {
                has_player: false,
                ..MprisState::default()
            },
            None,
            user_pinned.is_some(),
        ));
    }

    let mut with_status: Vec<(String, String)> = Vec::with_capacity(names.len());
    for name in &names {
        let status = match player_proxy(conn, name).await {
            Ok(p) => p.playback_status().await.unwrap_or_else(|_| "Stopped".into()),
            Err(_) => "Stopped".into(),
        };
        with_status.push((name.clone(), status));
    }

    let (active, clear_pin) =
        resolve_active_player(&with_status, user_pinned.as_deref());
    let Some(active) = active else {
        return Ok((
            MprisState {
                has_player: false,
                player_count: with_status.len(),
                ..MprisState::default()
            },
            None,
            clear_pin,
        ));
    };

    let player_count = with_status.len();
    let player_index = with_status
        .iter()
        .position(|(n, _)| n == &active)
        .map(|i| i + 1)
        .unwrap_or(0);
    let player_id = short_player_id(&active);

    let proxy = player_proxy(conn, &active).await?;
    let status = proxy
        .playback_status()
        .await
        .unwrap_or_else(|_| "Stopped".into());
    let metadata = proxy.metadata().await.unwrap_or_default();
    let (title, artist) = parse_metadata(&metadata);
    let art_url = extract_art_url(&metadata);
    let length_us = extract_length_us(&metadata);
    // Position must be polled (no PropertiesChanged on most players).
    let position_us = match proxy.position().await {
        Ok(p) if p >= 0 => Some(p),
        Ok(_) => None,
        Err(_) => None,
    };
    let playing = status.eq_ignore_ascii_case("Playing");

    Ok((
        MprisState {
            title,
            artist,
            playing,
            has_player: true,
            player_count,
            player_index,
            player_id,
            art_url,
            position_us,
            length_us,
        },
        Some(active),
        clear_pin,
    ))
}

async fn publish(
    data: &Mutable<MprisState>,
    status: &Mutable<ServiceStatus>,
    active_name: &Mutable<Option<String>>,
    user_pinned: &Mutable<Option<String>>,
    conn: &Connection,
) {
    match read_active_state(conn, user_pinned.get_cloned()).await {
        Ok((state, active, clear_pin)) => {
            if data.get_cloned() != state {
                data.set(state);
            }
            if active_name.get_cloned() != active {
                active_name.set(active);
            }
            if clear_pin {
                user_pinned.set(None);
            }
            status.set(ServiceStatus::Available);
        }
        Err(e) => {
            warn!("MprisSubscriber read failed: {e:?}");
        }
    }
}

async fn run(
    data: Mutable<MprisState>,
    status: Mutable<ServiceStatus>,
    active_name: Mutable<Option<String>>,
    user_pinned: Mutable<Option<String>>,
    conn_slot: Mutable<Option<Connection>>,
) {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
    let mut backoff = Duration::from_secs(1);

    loop {
        let session = async {
            let conn = Connection::session().await?;
            let _ = list_mpris_names(&conn).await?;
            Ok::<_, anyhow::Error>(conn)
        };

        match tokio::time::timeout(CONNECT_TIMEOUT, session).await {
            Ok(Ok(conn)) => {
                conn_slot.set(Some(conn.clone()));
                status.set(ServiceStatus::Available);
                info!("MprisSubscriber connected (session bus)");
                publish(&data, &status, &active_name, &user_pinned, &conn).await;
                backoff = Duration::from_secs(1);

                let dbus = match DBusProxy::new(&conn).await {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("MprisSubscriber DBusProxy failed: {e:?}");
                        status.set(ServiceStatus::Unavailable);
                        conn_slot.set(None);
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };
                let mut name_stream = match dbus.receive_name_owner_changed().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("MprisSubscriber NameOwnerChanged stream failed: {e:?}");
                        status.set(ServiceStatus::Unavailable);
                        conn_slot.set(None);
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                let mut tick = tokio::time::interval(POLL_INTERVAL);
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        maybe = name_stream.next() => {
                            match maybe {
                                Some(sig) => {
                                    let Ok(args) = sig.args() else { continue };
                                    if is_mpris_name(args.name.as_str()) {
                                        publish(
                                            &data,
                                            &status,
                                            &active_name,
                                            &user_pinned,
                                            &conn,
                                        )
                                        .await;
                                    }
                                }
                                None => {
                                    warn!("MprisSubscriber NameOwnerChanged ended");
                                    break;
                                }
                            }
                        }
                        _ = tick.tick() => {
                            publish(
                                &data,
                                &status,
                                &active_name,
                                &user_pinned,
                                &conn,
                            )
                            .await;
                        }
                    }
                }

                status.set(ServiceStatus::Unavailable);
                conn_slot.set(None);
                active_name.set(None);
                user_pinned.set(None);
                data.set(MprisState::default());
            }
            Ok(Err(e)) => {
                warn!("MprisSubscriber connect failed, retrying: {e:?}");
                status.set(ServiceStatus::Unavailable);
            }
            Err(_) => {
                warn!("MprisSubscriber connect timed out, retrying");
                status.set(ServiceStatus::Unavailable);
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_mpris_name_prefix() {
        assert!(is_mpris_name("org.mpris.MediaPlayer2.mpv"));
        assert!(is_mpris_name("org.mpris.MediaPlayer2.spotify"));
        assert!(!is_mpris_name("org.mpris.MediaPlayer2"));
        assert!(!is_mpris_name("org.kde.StatusNotifierItem"));
        assert!(!is_mpris_name(""));
    }

    #[test]
    fn short_player_id_strips_prefix() {
        assert_eq!(
            short_player_id("org.mpris.MediaPlayer2.chronos_a"),
            "chronos_a"
        );
        assert_eq!(short_player_id("not-a-prefix"), "not-a-prefix");
    }

    #[test]
    fn select_prefers_playing() {
        let players = vec![
            ("org.mpris.MediaPlayer2.a".into(), "Paused".into()),
            ("org.mpris.MediaPlayer2.b".into(), "Playing".into()),
            ("org.mpris.MediaPlayer2.c".into(), "Stopped".into()),
        ];
        assert_eq!(
            select_active_player(&players).as_deref(),
            Some("org.mpris.MediaPlayer2.b")
        );
    }

    #[test]
    fn select_falls_back_to_first() {
        let players = vec![
            ("org.mpris.MediaPlayer2.a".into(), "Paused".into()),
            ("org.mpris.MediaPlayer2.b".into(), "Stopped".into()),
        ];
        assert_eq!(
            select_active_player(&players).as_deref(),
            Some("org.mpris.MediaPlayer2.a")
        );
    }

    #[test]
    fn select_empty_is_none() {
        assert_eq!(select_active_player(&[]), None);
    }

    #[test]
    fn sticky_pin_holds_while_alive() {
        let players = vec![
            ("org.mpris.MediaPlayer2.a".into(), "Paused".into()),
            ("org.mpris.MediaPlayer2.b".into(), "Playing".into()),
        ];
        // Pin A even though B is Playing.
        let (active, clear) =
            resolve_active_player(&players, Some("org.mpris.MediaPlayer2.a"));
        assert_eq!(active.as_deref(), Some("org.mpris.MediaPlayer2.a"));
        assert!(!clear);
    }

    #[test]
    fn sticky_pin_clears_when_player_gone() {
        let players = vec![
            ("org.mpris.MediaPlayer2.b".into(), "Playing".into()),
            ("org.mpris.MediaPlayer2.c".into(), "Paused".into()),
        ];
        let (active, clear) =
            resolve_active_player(&players, Some("org.mpris.MediaPlayer2.a"));
        assert_eq!(active.as_deref(), Some("org.mpris.MediaPlayer2.b"));
        assert!(clear);
    }

    #[test]
    fn sticky_absent_uses_auto() {
        let players = vec![
            ("org.mpris.MediaPlayer2.a".into(), "Paused".into()),
            ("org.mpris.MediaPlayer2.b".into(), "Playing".into()),
        ];
        let (active, clear) = resolve_active_player(&players, None);
        assert_eq!(active.as_deref(), Some("org.mpris.MediaPlayer2.b"));
        assert!(!clear);
    }

    #[test]
    fn sticky_empty_list_clears_pin() {
        let (active, clear) =
            resolve_active_player(&[], Some("org.mpris.MediaPlayer2.a"));
        assert!(active.is_none());
        assert!(clear);
    }

    #[test]
    fn cycle_next_wraps() {
        let names = vec![
            "org.mpris.MediaPlayer2.a".into(),
            "org.mpris.MediaPlayer2.b".into(),
            "org.mpris.MediaPlayer2.c".into(),
        ];
        assert_eq!(
            cycle_player_names(&names, Some("org.mpris.MediaPlayer2.a"), CycleDirection::Next)
                .as_deref(),
            Some("org.mpris.MediaPlayer2.b")
        );
        assert_eq!(
            cycle_player_names(&names, Some("org.mpris.MediaPlayer2.c"), CycleDirection::Next)
                .as_deref(),
            Some("org.mpris.MediaPlayer2.a")
        );
    }

    #[test]
    fn cycle_prev_wraps() {
        let names = vec![
            "org.mpris.MediaPlayer2.a".into(),
            "org.mpris.MediaPlayer2.b".into(),
        ];
        assert_eq!(
            cycle_player_names(&names, Some("org.mpris.MediaPlayer2.a"), CycleDirection::Prev)
                .as_deref(),
            Some("org.mpris.MediaPlayer2.b")
        );
        assert_eq!(
            cycle_player_names(&names, Some("org.mpris.MediaPlayer2.b"), CycleDirection::Prev)
                .as_deref(),
            Some("org.mpris.MediaPlayer2.a")
        );
    }

    #[test]
    fn cycle_noop_with_zero_or_one() {
        assert!(cycle_player_names(&[], None, CycleDirection::Next).is_none());
        let one = vec!["org.mpris.MediaPlayer2.a".into()];
        assert!(
            cycle_player_names(&one, Some("org.mpris.MediaPlayer2.a"), CycleDirection::Next)
                .is_none()
        );
    }

    #[test]
    fn cycle_unknown_current_starts_from_zero() {
        let names = vec![
            "org.mpris.MediaPlayer2.a".into(),
            "org.mpris.MediaPlayer2.b".into(),
        ];
        // current not in list → pivot 0 → Next → b
        assert_eq!(
            cycle_player_names(&names, Some("org.mpris.MediaPlayer2.x"), CycleDirection::Next)
                .as_deref(),
            Some("org.mpris.MediaPlayer2.b")
        );
    }

    #[test]
    fn parse_metadata_plain_strings() {
        let mut map = HashMap::new();
        map.insert(
            "xesam:title".into(),
            Str::from("Song Title").try_into().unwrap(),
        );
        // artist as array of strings (spec)
        let artists: Array<'_> = Array::from(vec![Value::from("Artist One"), Value::from("B")]);
        map.insert("xesam:artist".into(), OwnedValue::try_from(artists).unwrap());
        let (title, artist) = parse_metadata(&map);
        assert_eq!(title, "Song Title");
        assert_eq!(artist, "Artist One");
    }

    #[test]
    fn parse_metadata_variant_wrapped() {
        // Real a{sv} values often arrive as variants (v).
        let mut map = HashMap::new();
        let title_v = Value::Value(Box::new(Value::from("Wrapped Title")));
        map.insert("xesam:title".into(), OwnedValue::try_from(title_v).unwrap());
        let artist_arr = Array::from(vec![Value::from("Wrapped Artist")]);
        let artist_v = Value::Value(Box::new(Value::from(artist_arr)));
        map.insert(
            "xesam:artist".into(),
            OwnedValue::try_from(artist_v).unwrap(),
        );
        let (title, artist) = parse_metadata(&map);
        assert_eq!(title, "Wrapped Title");
        assert_eq!(artist, "Wrapped Artist");
    }

    #[test]
    fn parse_metadata_missing_keys() {
        let map = HashMap::new();
        let (title, artist) = parse_metadata(&map);
        assert!(title.is_empty());
        assert!(artist.is_empty());
    }

    #[test]
    fn parse_metadata_artist_as_plain_string() {
        let mut map = HashMap::new();
        map.insert(
            "xesam:artist".into(),
            Str::from("Solo").try_into().unwrap(),
        );
        let (_, artist) = parse_metadata(&map);
        assert_eq!(artist, "Solo");
    }

    /// Fixture shape from live idle Vivaldi on this host (2026-07-21):
    /// `busctl --user get-property … Metadata` → `a{sv} 1 "mpris:length" x 0`
    /// (no art, zero length). Plus a full synthetic frame matching a real
    /// playing track (file:// art + positive length).
    #[test]
    fn extract_art_and_length_from_metadata_fixture() {
        let mut idle = HashMap::new();
        idle.insert(
            "mpris:length".into(),
            OwnedValue::try_from(Value::I64(0)).unwrap(),
        );
        assert_eq!(extract_length_us(&idle), None);
        assert_eq!(extract_art_url(&idle), None);

        let mut playing = HashMap::new();
        // Real-shape values: i64 length in μs, file:// artUrl, title/artist.
        // length = 3m 42.2s = 222_200_000 μs (typical track).
        playing.insert(
            "mpris:length".into(),
            OwnedValue::try_from(Value::I64(222_200_000)).unwrap(),
        );
        playing.insert(
            "mpris:artUrl".into(),
            Str::from("file:///usr/share/pixmaps/archlinux-logo.png")
                .try_into()
                .unwrap(),
        );
        playing.insert(
            "xesam:title".into(),
            Str::from("Colour Temperature").try_into().unwrap(),
        );
        let art_arr_artist = Array::from(vec![Value::from("Ambient Systems")]);
        playing.insert(
            "xesam:artist".into(),
            OwnedValue::try_from(art_arr_artist).unwrap(),
        );

        assert_eq!(
            extract_art_url(&playing).as_deref(),
            Some("file:///usr/share/pixmaps/archlinux-logo.png")
        );
        assert_eq!(extract_length_us(&playing), Some(222_200_000));
        let (title, artist) = parse_metadata(&playing);
        assert_eq!(title, "Colour Temperature");
        assert_eq!(artist, "Ambient Systems");
    }

    #[test]
    fn extract_length_variant_wrapped_and_u64() {
        let mut map = HashMap::new();
        let wrapped = Value::Value(Box::new(Value::I64(90_000_000)));
        map.insert("mpris:length".into(), OwnedValue::try_from(wrapped).unwrap());
        assert_eq!(extract_length_us(&map), Some(90_000_000));

        let mut map_u = HashMap::new();
        map_u.insert(
            "mpris:length".into(),
            OwnedValue::try_from(Value::U64(15_000_000)).unwrap(),
        );
        assert_eq!(extract_length_us(&map_u), Some(15_000_000));
    }

    #[test]
    fn has_position_requires_positive_length_and_readable_pos() {
        assert!(!has_position(None, Some(100)));
        assert!(!has_position(Some(10), None));
        assert!(!has_position(Some(10), Some(0)));
        assert!(has_position(Some(10), Some(100)));
        assert!(has_position(Some(0), Some(100)));
    }

    #[test]
    fn mpris_new_panics_outside_runtime() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = MprisSubscriber::new();
        }));
        assert!(
            result.is_err(),
            "MprisSubscriber::new() must panic outside a tokio runtime"
        );
    }
}
