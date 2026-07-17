//! MPRIS media-player service via session-bus D-Bus (`org.mpris.MediaPlayer2`).
//!
//! ASYNC TEMPLATE (spec §5.1): same connect+retry shape as UPower / tray.
//! Discovery is dynamic (no fixed well-known name) — `ListNames` +
//! `NameOwnerChanged`, filtered by the `org.mpris.MediaPlayer2.` prefix.
//!
//! **MVP player pick:** first with `PlaybackStatus == "Playing"`, else first
//! name in list order. Multi-player switcher is out of scope.

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
pub use types::{MprisCommand, MprisState};

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
        let conn = Mutable::new(None);
        let handle = Handle::current();
        tokio::spawn(run(
            data.clone(),
            status.clone(),
            active_name.clone(),
            conn.clone(),
        ));
        Self {
            data,
            status,
            active_name,
            conn,
            runtime: handle,
        }
    }

    /// Fire-and-forget command against the active player + immediate re-read.
    pub fn dispatch(&self, cmd: MprisCommand) {
        let data = self.data.clone();
        let status = self.status.clone();
        let active_name = self.active_name.clone();
        let conn = self.conn.clone();
        self.runtime.spawn(async move {
            let Some(conn) = conn.get_cloned() else {
                warn!("MprisSubscriber: no session connection, drop {cmd:?}");
                return;
            };
            let Some(name) = active_name.get_cloned() else {
                warn!("MprisSubscriber: no active player, drop {cmd:?}");
                return;
            };
            if let Err(e) = apply_command(&conn, &name, &cmd).await {
                warn!("MprisSubscriber command failed ({cmd:?} on {name}): {e:?}");
                return;
            }
            match read_active_state(&conn).await {
                Ok((state, active)) => {
                    data.set(state);
                    active_name.set(active);
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

/// MVP selection: first `"Playing"`, else first entry. Empty list → `None`.
pub fn select_active_player(players: &[(String, String)]) -> Option<String> {
    players
        .iter()
        .find(|(_, status)| status.eq_ignore_ascii_case("Playing"))
        .or_else(|| players.first())
        .map(|(name, _)| name.clone())
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

fn extract_string(map: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let owned = map.get(key)?;
    let value = unwrap_variant(Value::from(owned.try_clone().ok()?));
    match value {
        Value::Str(s) => Some(s.to_string()),
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

/// Read PlaybackStatus for each known name; pick active; load metadata.
async fn read_active_state(
    conn: &Connection,
) -> anyhow::Result<(MprisState, Option<String>)> {
    let names = list_mpris_names(conn).await?;
    if names.is_empty() {
        return Ok((
            MprisState {
                has_player: false,
                ..MprisState::default()
            },
            None,
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

    let Some(active) = select_active_player(&with_status) else {
        return Ok((
            MprisState {
                has_player: false,
                ..MprisState::default()
            },
            None,
        ));
    };

    let proxy = player_proxy(conn, &active).await?;
    let status = proxy
        .playback_status()
        .await
        .unwrap_or_else(|_| "Stopped".into());
    let metadata = proxy.metadata().await.unwrap_or_default();
    let (title, artist) = parse_metadata(&metadata);
    let playing = status.eq_ignore_ascii_case("Playing");

    Ok((
        MprisState {
            title,
            artist,
            playing,
            has_player: true,
        },
        Some(active),
    ))
}

async fn publish(
    data: &Mutable<MprisState>,
    status: &Mutable<ServiceStatus>,
    active_name: &Mutable<Option<String>>,
    conn: &Connection,
) {
    match read_active_state(conn).await {
        Ok((state, active)) => {
            if data.get_cloned() != state {
                data.set(state);
            }
            if active_name.get_cloned() != active {
                active_name.set(active);
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
                publish(&data, &status, &active_name, &conn).await;
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
                                        publish(&data, &status, &active_name, &conn).await;
                                    }
                                }
                                None => {
                                    warn!("MprisSubscriber NameOwnerChanged ended");
                                    break;
                                }
                            }
                        }
                        _ = tick.tick() => {
                            publish(&data, &status, &active_name, &conn).await;
                        }
                    }
                }

                status.set(ServiceStatus::Unavailable);
                conn_slot.set(None);
                active_name.set(None);
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
