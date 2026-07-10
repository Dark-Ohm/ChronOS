//! Minimal GPUI app proving the full reactive chain (spec §9).
//! Run: `cargo run -p chronos --example status-printer`

use chronos_app::state::AppState;
use chronos_services::Service;
use futures_signals::signal::SignalExt;
use futures_util::stream::StreamExt;
use gpui_platform::application;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let services = rt.block_on(async { chronos_services::init_all() });

    let app = application();
    app.run(move |cx| {
        AppState::init(services, cx);

        let comp = AppState::compositor(cx).clone();
        let net = AppState::network(cx).clone();
        let up = AppState::upower(cx).clone();

        // Subscribe to compositor updates
        let comp_signal = comp.subscribe();
        cx.spawn(async move |_cx| {
            let mut stream = comp_signal.to_stream();
            while let Some(data) = stream.next().await {
                tracing::info!("compositor: {:?} (status via get)", data);
            }
        })
        .detach();

        // Subscribe to network updates
        let net_signal = net.subscribe();
        cx.spawn(async move |_cx| {
            let mut stream = net_signal.to_stream();
            while let Some(data) = stream.next().await {
                tracing::info!("network: {:?}", data);
            }
        })
        .detach();

        // Subscribe to upower updates
        let up_signal = up.subscribe();
        cx.spawn(async move |_cx| {
            let mut stream = up_signal.to_stream();
            while let Some(data) = stream.next().await {
                tracing::info!("upower: {:?}", data);
            }
        })
        .detach();

        tracing::info!("status-printer subscribed; logging updates");
    });
}
