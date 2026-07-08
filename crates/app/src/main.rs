mod bar;
mod ipc;

use gpui_platform::application;
use ipc::IpcSubscriber;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Chronos starting");

    let Some(subscriber) = IpcSubscriber::init() else {
        tracing::info!("Another Chronos instance is running, signaled it and exiting");
        return;
    };

    let app = application();
    app.run(move |cx| {
        tracing::info!("GPUI application context ready");
        subscriber.start(cx);
    });

    tracing::info!("Chronos exited");
}
