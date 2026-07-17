mod messages;
mod service;

use gpui::App;

pub use service::IpcSubscriber;

impl IpcSubscriber {
    /// Starts listening for pings, launcher-toggle, and wallpaper requests.
    /// Keeps `self` alive for the lifetime of the listener so the socket
    /// file isn't removed early.
    pub fn start(mut self, cx: &mut App) {
        let (mut ping_receiver, mut toggle_receiver, mut wallpaper_receiver) =
            self.start_listener();

        cx.spawn(async move |cx| {
            let _ipc_guard = self;
            tracing::info!("IPC listener started");

            let mut last_toggle_at = std::time::Instant::now() - std::time::Duration::from_secs(1);

            loop {
                tokio::select! {
                    ping = ping_receiver.recv() => {
                        if ping.is_some() {
                            let _ = cx.update(|_cx| {
                                tracing::info!("Received ping from a secondary instance");
                            });
                        } else {
                            break;
                        }
                    }
                    toggle = toggle_receiver.recv() => {
                        if toggle.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_toggle_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_toggle_at = now;
                                tracing::info!("IPC toggle received, calling launcher::toggle");
                                let _ = cx.update(|_cx| {
                                    crate::launcher::toggle(_cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    wallpaper = wallpaper_receiver.recv() => {
                        if let Some(cmd) = wallpaper {
                            let _ = cx.update(|cx| {
                                match cmd {
                                    crate::ipc::messages::WallpaperIpcCmd::Next => {
                                        tracing::info!("IPC wallpaper-next received");
                                        crate::wallpaper_ctl::next(cx);
                                    }
                                    crate::ipc::messages::WallpaperIpcCmd::Set(path) => {
                                        tracing::info!("IPC wallpaper-set received: {}", path.display());
                                        crate::wallpaper_ctl::set(cx, &path);
                                    }
                                }
                            });
                        } else {
                            break;
                        }
                    }
                }
            }

            tracing::warn!("IPC listener ended unexpectedly");
        })
        .detach();
    }
}
