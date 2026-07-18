mod bar;
mod desktop_terminal;
mod dock;
mod ipc;
mod launcher;
mod notifications;
mod osd;
mod plugin_bridge;
pub mod state;
mod wallpaper_ctl;

use chronos_luau::PluginManager;
use chronos_services;
use gpui_platform::application;
use ipc::IpcSubscriber;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Chronos starting");

    let Some(subscriber) = IpcSubscriber::init() else {
        tracing::info!("Another Chronos instance is running, signaled it and exiting");
        return;
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Run the entire app lifecycle inside the tokio runtime so the reactor
    // stays active for the whole session. This is required because the IPC
    // listener (started from within `app.run`) uses `tokio::spawn`, which
    // needs a live runtime. `block_on` drives the runtime while `app.run`
    // blocks on the Wayland event loop.
    rt.block_on(async {
        // block_on enters the runtime context so Handle::current() resolves
        // inside NetworkSubscriber::new() / UPowerSubscriber::new().
        let services = chronos_services::init_all();

        // services is Send + Sync (Mutable + zbus::Connection) -> crosses to GPUI thread
        let app = application();
        app.run(move |cx| {
            tracing::info!("GPUI application context ready");

            // Initialize global AppState so watch() / AppState::compositor() etc. work
            state::AppState::init(services, cx);

            subscriber.start(cx);
            chronos_ui::Theme::init(cx);
            bar::init(cx);
            dock::init(cx);
            notifications::init(cx);
            osd::init(cx);
            desktop_terminal::init(cx);

            // Initialize launcher global state (desktop entries come from AppState::applications)
            launcher::init(cx);

            let plugin_dirs = vec![
                dirs::config_dir().unwrap().join("chronos/plugins"),
                std::path::PathBuf::from("/usr/share/chronos/plugins"),
            ];
            let mut plugin_manager = PluginManager::new(plugin_dirs);
            plugin_manager.load_all();
            plugin_bridge::register_plugin_widgets(&plugin_manager, cx);
            cx.set_global(plugin_manager);
            PluginManager::start_tick_loop(cx);
            PluginManager::start_watcher(cx);
        });
    });

    tracing::info!("Chronos exited");
}
