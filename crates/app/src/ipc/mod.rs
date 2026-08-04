mod messages;
mod service;

use gpui::App;

pub use service::IpcSubscriber;
use messages::WorkspaceModeIpcCmd;

impl IpcSubscriber {
    /// Starts listening for pings, launcher-toggle, and wallpaper requests.
    /// Keeps `self` alive for the lifetime of the listener so the socket
    /// file isn't removed early.
    pub fn start(mut self, cx: &mut App) {
        let (
            mut ping_receiver,
            mut toggle_receiver,
            mut wallpaper_receiver,
            mut side_panel_toggle_receiver,
            mut side_panel_right_toggle_receiver,
            mut theme_toggle_receiver,
            mut edit_mode_toggle_receiver,
            mut workspace_mode_receiver,
            mut select_tab_receiver,
            mut preview_target_receiver,
            mut expand_left_receiver,
        ) = self.start_listener();

        cx.spawn(async move |cx| {
            let _ipc_guard = self;
            tracing::info!("IPC listener started");

            let mut last_toggle_at = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_side_panel_toggle_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_side_panel_right_toggle_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_theme_toggle_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_edit_mode_toggle_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_workspace_mode_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_select_tab_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);
            let mut last_expand_left_at =
                std::time::Instant::now() - std::time::Duration::from_secs(1);

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
                    side_panel_toggle = side_panel_toggle_receiver.recv() => {
                        if side_panel_toggle.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_side_panel_toggle_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_side_panel_toggle_at = now;
                                tracing::info!(
                                    "IPC toggle received, calling side_panel_left::toggle"
                                );
                                let _ = cx.update(|cx| {
                                    crate::side_panel_left::toggle(cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    side_panel_right_toggle = side_panel_right_toggle_receiver.recv() => {
                        if side_panel_right_toggle.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_side_panel_right_toggle_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_side_panel_right_toggle_at = now;
                                tracing::info!(
                                    "IPC toggle received, calling side_panel_right::toggle"
                                );
                                let _ = cx.update(|cx| {
                                    crate::side_panel_right::toggle(cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    theme_toggle = theme_toggle_receiver.recv() => {
                        if theme_toggle.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_theme_toggle_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_theme_toggle_at = now;
                                tracing::info!("IPC toggle-theme received");
                                let _ = cx.update(|cx| {
                                    crate::theme_config::toggle(cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    edit_mode_toggle = edit_mode_toggle_receiver.recv() => {
                        if edit_mode_toggle.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_edit_mode_toggle_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_edit_mode_toggle_at = now;
                                tracing::info!("IPC toggle-edit-mode received");
                                let _ = cx.update(|cx| {
                                    crate::edit_mode::toggle(cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    workspace_mode_cmd = workspace_mode_receiver.recv() => {
                        if let Some(cmd) = workspace_mode_cmd {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_workspace_mode_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_workspace_mode_at = now;
                                let _ = cx.update(|cx| match cmd {
                                    WorkspaceModeIpcCmd::Toggle => {
                                        crate::workspace_mode::toggle(cx)
                                    }
                                    WorkspaceModeIpcCmd::Set(mode) => {
                                        crate::workspace_mode::set(cx, mode)
                                    }
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    select_tab = select_tab_receiver.recv() => {
                        if let Some(tab) = select_tab {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_select_tab_at)
                                >= std::time::Duration::from_millis(100)
                            {
                                last_select_tab_at = now;
                                tracing::info!(tab = tab.id(), "IPC select-tab received");
                                let _ = cx.update(|cx| {
                                    crate::side_panel_right::select_tab(tab, cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    preview_target = preview_target_receiver.recv() => {
                        if let Some(path) = preview_target {
                            tracing::info!(path = %path.display(), "IPC preview-target received");
                            let _ = cx.update(|cx| {
                                crate::side_panel_right::preview_target(path, cx);
                            });
                        } else {
                            break;
                        }
                    }
                    expand_left = expand_left_receiver.recv() => {
                        if expand_left.is_some() {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_expand_left_at)
                                >= std::time::Duration::from_millis(200)
                            {
                                last_expand_left_at = now;
                                tracing::info!("IPC expand-left received");
                                let _ = cx.update(|cx| {
                                    crate::side_panel_left::expand_with_composer(cx);
                                });
                            }
                        } else {
                            break;
                        }
                    }
                    wallpaper = wallpaper_receiver.recv() => {
                        if let Some(cmd) = wallpaper {
                            match cmd {
                                crate::ipc::messages::WallpaperIpcCmd::Next => {
                                    let _ = cx.update(|cx| {
                                        tracing::info!("IPC wallpaper-next received");
                                        crate::wallpaper_ctl::next(cx);
                                    });
                                }
                                crate::ipc::messages::WallpaperIpcCmd::Set(path) => {
                                    let _ = cx.update(|cx| {
                                        tracing::info!("IPC wallpaper-set received: {}", path.display());
                                        crate::wallpaper_ctl::set(cx, &path);
                                    });
                                }
                                crate::ipc::messages::WallpaperIpcCmd::Gallery => {
                                    tracing::info!("IPC wallpaper-gallery received");
                                    let _ = cx.update(|cx| {
                                        match crate::wallpaper_ctl::open_waytrogen_gallery() {
                                            Ok(()) => {
                                                // Delayed resync: waytrogen sets via awww outside
                                                // our Mutable. Full child-wait needs Send App —
                                                // poll refresh after a short delay instead.
                                                let wallpaper =
                                                    crate::state::AppState::wallpaper(cx).clone();
                                                cx.spawn(async move |cx| {
                                                    cx.background_executor()
                                                        .timer(std::time::Duration::from_secs(3))
                                                        .await;
                                                    wallpaper.refresh();
                                                    tracing::info!(
                                                        "wallpaper: post-gallery delayed refresh"
                                                    );
                                                })
                                                .detach();
                                            }
                                            Err(e) => {
                                                tracing::warn!("IPC wallpaper-gallery: {e}");
                                            }
                                        }
                                    });
                                }
                                crate::ipc::messages::WallpaperIpcCmd::Refresh => {
                                    tracing::info!("IPC wallpaper-refresh received");
                                    let _ = cx.update(|cx| {
                                        crate::wallpaper_ctl::refresh_after_gallery(cx);
                                    });
                                }
                            }
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
