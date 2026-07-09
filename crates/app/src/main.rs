mod bar;
mod ipc;
mod plugin_bridge;

use chronos_luau::PluginManager;
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
        bar::init(cx);

        let plugin_dirs = vec![
            dirs::config_dir().unwrap().join("chronos/plugins"),
            std::path::PathBuf::from("/usr/share/chronos/plugins"),
        ];
        let mut plugin_manager = PluginManager::new(plugin_dirs);
        plugin_manager.load_all();
        plugin_bridge::register_plugin_widgets(&plugin_manager, cx);
        cx.set_global(plugin_manager);
        PluginManager::start_tick_loop(cx);
    });

    tracing::info!("Chronos exited");
}
