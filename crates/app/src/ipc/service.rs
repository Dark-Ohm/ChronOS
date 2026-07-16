use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use tokio::net::UnixListener as TokioUnixListener;
use tokio::sync::mpsc;

use super::messages::{encode_ping, is_ping, is_toggle_launcher};

pub type IpcReceiver = mpsc::UnboundedReceiver<()>;
pub type IpcToggleReceiver = mpsc::UnboundedReceiver<()>;

pub enum AcquireResult {
    Primary(IpcSubscriber),
    Secondary,
    Error(String),
}

/// Owns the bound-but-not-yet-accepting Unix socket for the primary instance.
///
/// The listener is stored as a std `UnixListener` so `init()` can run before
/// the tokio reactor is active (it only binds a socket + signals a peer).
/// `start_listener` converts it to a tokio listener, which requires a running
/// runtime.
pub struct IpcSubscriber {
    listener: Option<UnixListener>,
    socket_path: PathBuf,
}

impl IpcSubscriber {
    /// Returns `Some` when this process should continue as the primary
    /// instance, `None` when an existing instance was signaled instead.
    pub fn init() -> Option<IpcSubscriber> {
        match acquire_at(&socket_path(), &encode_ping()) {
            AcquireResult::Primary(subscriber) => Some(subscriber),
            AcquireResult::Secondary => None,
            AcquireResult::Error(err) => {
                tracing::error!("IPC service error: {}", err);
                None
            }
        }
    }

    /// Starts the accept loop. Must be called from within a tokio runtime.
    /// Returns receivers that yield `()` once per received ping / toggle request.
    pub fn start_listener(&mut self) -> (IpcReceiver, IpcToggleReceiver) {
        let (ping_sender, ping_receiver) = mpsc::unbounded_channel();
        let (toggle_sender, toggle_receiver) = mpsc::unbounded_channel();

        if let Some(std_listener) = self.listener.take() {
            // `from_std` requires a running tokio reactor, which is active here.
            match TokioUnixListener::from_std(std_listener) {
                Ok(listener) => {
                    tokio::spawn(async move {
                        accept_loop(listener, ping_sender, toggle_sender).await;
                    });
                }
                Err(e) => tracing::error!("Failed to convert IPC listener: {e}"),
            }
        }

        (ping_receiver, toggle_receiver)
    }
}

impl Drop for IpcSubscriber {
    fn drop(&mut self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

/// Try to become the primary instance at `path`, or signal an existing one.
///
/// Synchronous on the secondary path (no runtime needed to signal). When
/// becoming primary, the returned `IpcSubscriber` still needs a tokio
/// runtime active before `start_listener` is called.
pub fn acquire_at(path: &Path, payload: &str) -> AcquireResult {
    if let Ok(mut stream) = UnixStream::connect(path) {
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));

        if let Err(e) = stream.write_all(payload.as_bytes()) {
            return AcquireResult::Error(format!("Failed to signal existing instance: {}", e));
        }
        let _ = stream.flush();
        let _ = stream.shutdown(std::net::Shutdown::Write);

        return AcquireResult::Secondary;
    }

    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            return AcquireResult::Error(format!("Failed to remove stale socket: {}", e));
        }
    }

    let listener = match UnixListener::bind(path) {
        Ok(l) => l,
        Err(e) => return AcquireResult::Error(format!("Failed to create socket: {}", e)),
    };

    if let Err(e) = listener.set_nonblocking(true) {
        return AcquireResult::Error(format!("Failed to configure socket: {}", e));
    }

    AcquireResult::Primary(IpcSubscriber {
        listener: Some(listener),
        socket_path: path.to_path_buf(),
    })
}

fn get_user_id() -> String {
    std::env::var("UID")
        .or_else(|_| std::env::var("SUDO_UID"))
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn socket_path_in(runtime_dir: Option<&str>) -> PathBuf {
    match runtime_dir {
        Some(dir) => PathBuf::from(dir).join("chronos.sock"),
        None => PathBuf::from("/tmp").join(format!("chronos-{}.sock", get_user_id())),
    }
}

pub fn socket_path() -> PathBuf {
    socket_path_in(std::env::var("XDG_RUNTIME_DIR").ok().as_deref())
}

async fn accept_loop(
    listener: TokioUnixListener,
    ping_sender: mpsc::UnboundedSender<()>,
    toggle_sender: mpsc::UnboundedSender<()>,
) {
    use tokio::io::AsyncReadExt;

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let ping_sender = ping_sender.clone();
                let toggle_sender = toggle_sender.clone();
                tokio::spawn(async move {
                    let mut buffer = Vec::with_capacity(16);
                    let read = tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        stream.read_to_end(&mut buffer),
                    )
                    .await;

                    if let Ok(Ok(_)) = read {
                        let payload = String::from_utf8_lossy(&buffer).to_string();
                        if is_ping(&payload) {
                            let _ = ping_sender.send(());
                        } else if is_toggle_launcher(&payload) {
                            let _ = toggle_sender.send(());
                        }
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept connection: {}", e);
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_xdg_runtime_dir_when_set() {
        let path = socket_path_in(Some("/run/user/1000"));
        assert_eq!(path, PathBuf::from("/run/user/1000/chronos.sock"));
    }

    #[test]
    fn falls_back_to_tmp_when_unset() {
        let path = socket_path_in(None);
        assert!(path.starts_with("/tmp"));
        assert!(path.to_string_lossy().contains("chronos-"));
    }

    #[tokio::test]
    async fn second_acquire_on_same_path_becomes_secondary() {
        let dir = std::env::temp_dir().join(format!("chronos-ipc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.sock");
        let _ = std::fs::remove_file(&path);

        let first = acquire_at(&path, "ping");
        assert!(matches!(&first, AcquireResult::Primary(_)));

        let second = acquire_at(&path, "ping");
        assert!(matches!(&second, AcquireResult::Secondary));

        drop(first);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
