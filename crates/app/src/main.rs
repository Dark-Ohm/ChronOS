mod agent_follow;
mod assets;
mod bar;
pub mod bar_settings;
mod calendar_popup;
mod desktop_terminal;
mod dock;
mod edit_mode;
mod frame;
mod games_config;
mod icon_resolution;
mod workspace_mode;
mod scene;
mod surface_effects;
mod ipc;
mod launcher;
mod monitor;
mod motion;
mod notifications;
mod osd;
mod plugin_bridge;
mod project_switcher;
mod popup_click_catcher;
mod side_panel_left;
mod side_panel_right;
mod start_menu;
pub mod state;
mod gaming_mode;
mod theme_config;
mod tray_menu;
mod updates_list;
mod volume_popup;
mod wallpaper_ctl;
mod power;
mod power_controls;

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
        // T266: probe the blur bridge in the background and apply the
        // persisted blur once — must run after theme_config::init so the
        // effective theme (incl. surface settings) is installed first.
        surface_effects::init(cx);
        gpui_component::init(cx);
        // gpui_component::init overwrote the component Theme mode with Light
        // default — resync it to the active shell theme (T205 editor gutter).
        theme_config::sync_gpui_component_theme(cx);
        edit_mode::init(cx);
        workspace_mode::init(cx);
        scene::init(cx);
        monitor::init(cx);
        bar::init(cx);
        frame::init(cx);
        notifications::init(cx);
        osd::init(cx);
        tray_menu::init(cx);
        project_switcher::init(cx);
        volume_popup::init(cx);
        calendar_popup::init(cx);
        gaming_mode::init(cx);
        side_panel_right::init(cx);
        side_panel_left::init(cx);
        // T284: the frame re-applies panel geometry on Hide↔Wrap transitions
        // through this hook — frame.rs never imports the panels (cycle).
        frame::set_after_apply(|cx| {
            side_panel_left::apply_frame_inset(cx);
            side_panel_right::apply_frame_inset(cx);
        });
        // Register the PTY registry global *before* desktop_terminal::init so
        // the first widget can acquire its session (T257).
        cx.set_global(crate::desktop_terminal::TerminalRegistryGlobal {
            registry: std::sync::Arc::new(std::sync::Mutex::new(
                chronos_services::TerminalRegistry::new(),
            )),
            windows: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        desktop_terminal::init(cx);

        // Initialize launcher global state (desktop entries come from AppState::applications)
        launcher::init(cx);
        // T265-H: the compact Start-menu surface (shares the launcher's model).
        start_menu::init(cx);

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
