mod assets;
mod bar;
mod desktop_terminal;
mod dock;
mod edit_mode;
mod workspace_mode;
mod ipc;
mod launcher;
mod monitor;
mod motion;
mod notifications;
mod osd;
mod plugin_bridge;
mod project_switcher;
mod side_panel_left;
mod side_panel_right;
pub mod state;
mod system_popup;
mod theme_config;
mod tray_menu;
mod updates_popup;
mod volume_popup;
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

    // Enter the runtime instead of blocking on it. `enter()` is enough for what
    // we actually need — `Handle::current()` inside NetworkSubscriber::new() /
    // UPowerSubscriber::new(), and `tokio::spawn` from the IPC listener — while
    // the multi-thread runtime is driven by its own workers.
    //
    // Do NOT go back to `rt.block_on(async { app.run(..) })`. `block_on` grants
    // one coop budget per poll of its future (tokio runtime/park.rs:284), and
    // `app.run` never returns from its first poll: the whole Wayland event loop
    // lives inside it. So the main thread got ONE budget of 128 operations
    // (`Budget::initial()`, tokio task/coop/mod.rs:115) for the entire process
    // lifetime, never replenished. Once spent, every poll of any tokio resource
    // on the main thread returns Pending *and* wakes itself
    // (task/coop/mod.rs:372-407) — a self-rescheduling storm that pinned the
    // main thread at 100% CPU. T143, measured live: the agent panel froze at
    // exactly streaming event #125, the poll at which the budget flipped.
    let _rt_guard = rt.enter();

    let services = chronos_services::init_all();

    // services is Send + Sync (Mutable + zbus::Connection) -> crosses to GPUI thread
    let app = application().with_assets(assets::Assets);
    app.run(move |cx| {
        tracing::info!("GPUI application context ready");

        // Initialize global AppState so watch() / AppState::compositor() etc. work
        state::AppState::init(services, cx);

        subscriber.start(cx);
        theme_config::init(cx);
        gpui_component::init(cx);
        edit_mode::init(cx);
        workspace_mode::init(cx);
        bar::init(cx);
        notifications::init(cx);
        notifications::history_popup::init(cx);
        osd::init(cx);
        tray_menu::init(cx);
        updates_popup::init(cx);
        project_switcher::init(cx);
        volume_popup::init(cx);
        system_popup::init(cx);
        side_panel_right::init(cx);
        side_panel_left::init(cx);
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

    tracing::info!("Chronos exited");
}
