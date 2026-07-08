mod messages;
mod service;

use gpui::App;

pub use service::IpcSubscriber;

impl IpcSubscriber {
    /// Starts listening for pings and logs each one. Keeps `self` alive for
    /// the lifetime of the listener so the socket file isn't removed early.
    pub fn start(mut self, cx: &mut App) {
        let mut receiver = self.start_listener();

        cx.spawn(async move |cx| {
            let _ipc_guard = self;
            tracing::info!("IPC listener started");

            while receiver.recv().await.is_some() {
                let _ = cx.update(|_cx| {
                    tracing::info!("Received ping from a secondary instance");
                });
            }

            tracing::warn!("IPC listener ended unexpectedly");
        })
        .detach();
    }
}
