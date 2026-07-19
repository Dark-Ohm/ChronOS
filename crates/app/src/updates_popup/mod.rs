//! Updates popup — pending-package list + "Upgrade all" button, opened by
//! clicking the bar's updates widget.
//!
//! Mirrors `tray_menu/` for the window-lifecycle bits (layer-shell popup,
//! the `close_this` reentrancy guard around `window.remove_window()` — see
//! HANDOFF.md "СИСТЕМНЫЙ БАГ: window.remove_window()") but is simpler:
//! there is only ever one popup (no per-service keying), and — per ZED.md's
//! explicit warning about the `follow_mouse=1` focus-loss trap (HANDOFF.md /
//! MEMORY.md 2026-07-18: "Cline №9 debounce отклонён") — it does **not**
//! close on keyboard-focus loss at all. Dismiss is always a conscious
//! action: the bar icon toggle, the in-popup "✕" button, or "Upgrade all".

pub mod view;

use gpui::{
    App, Bounds, Context, DisplayId, Entity, Global, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};

use chronos_services::{AurCommand, Service, UpdatesState};

use crate::state::{self, AppState};
use crate::updates_popup::view::UpdatesPopupView;

/// Popup width (px).
const POPUP_WIDTH: f32 = 360.;
/// Top + right margin (px) so the card sits just below the bar's top edge —
/// same margin `tray_menu` uses (bar height assumption).
const POPUP_MARGIN_TOP: f32 = 36.;
const POPUP_MARGIN_RIGHT: f32 = 8.;
/// Safety-margin budgets for header/divider/footer, in px. These are upper
/// bounds, not measured exact sizes — a first attempt at this popup sized
/// the window from `count * ROW_H`-style pixel math with an unmeasured
/// per-row height (32px) and no hard clip on the list; live testing
/// 2026-07-19 showed the real rendered row height is taller than that
/// guess, so a 24-row list consumed the entire window and pushed the
/// "Upgrade all" button below the window's bottom edge — invisible and
/// unclickable, not just visually truncated. Pixel-perfect math on
/// unmeasured GPUI text metrics is not reliable enough to build the only
/// path to a privileged action on, so the fix here does not depend on it:
/// the list gets a hard `max_h()` + `overflow_hidden()` clip (`LIST_MAX_H`)
/// and the footer is laid out *after* that clipped box, guaranteeing it
/// fits inside `MAX_POPUP_H` regardless of how tall rows actually render.
pub(crate) const HEADER_BUDGET_H: f32 = 48.;
const DIVIDER_H: f32 = 1.;
pub(crate) const FOOTER_BUDGET_H: f32 = 64.;
/// Per-row geometry (px) — used only to pick how many rows to *offer* into
/// the clipped list, not to size the window (the `max_h` clip is the real
/// guarantee, this just avoids handing it more rows than can plausibly fit
/// so the list doesn't cut off mid-row).
pub(crate) const ROW_H: f32 = 40.;
/// Height of the "up to date" placeholder row.
const EMPTY_ROW_H: f32 = 40.;
/// Hard pixel clip on the row-list container (`view.rs`: `.max_h(px(LIST_MAX_H)).overflow_hidden()`).
pub(crate) const LIST_MAX_H: f32 = 300.;
/// Total window height cap. Deliberately `HEADER_BUDGET_H + DIVIDER_H +
/// LIST_MAX_H + FOOTER_BUDGET_H` plus a small margin — see the comment on
/// `HEADER_BUDGET_H` for why this isn't computed from real row counts.
pub(crate) const MAX_POPUP_H: f32 = HEADER_BUDGET_H + DIVIDER_H + LIST_MAX_H + FOOTER_BUDGET_H;

/// How many update rows to hand into the clipped list before switching to a
/// "+N more" note. Not the layout guarantee itself (`LIST_MAX_H` clip is) —
/// just keeps the note from being buried under rows that would get clipped
/// mid-row anyway.
pub(crate) fn max_visible_rows() -> usize {
    ((LIST_MAX_H / ROW_H).floor() as usize).max(1)
}

fn estimate_popup_height(count: usize) -> f32 {
    if count == 0 {
        HEADER_BUDGET_H + DIVIDER_H + EMPTY_ROW_H
    } else {
        MAX_POPUP_H
    }
}

/// Global state for the updates popup.
#[derive(Default)]
pub struct UpdatesPopupState {
    /// Window handle while the popup is open; `None` when closed.
    handle: Option<WindowHandle<UpdatesPopupView>>,
    /// Watcher entity driving repaints/resizes on `UpdatesState` changes.
    watcher: Option<Entity<UpdatesPopupWatcher>>,
}

impl Global for UpdatesPopupState {}

/// Tiny entity that hosts the `state::watch()` subscription — same role as
/// `TrayMenuWatcher`; it has no state of its own.
pub struct UpdatesPopupWatcher {}

fn pick_display(cx: &App) -> Option<DisplayId> {
    cx.primary_display()
        .map(|d| d.id())
        .or_else(|| cx.displays().into_iter().next().map(|d| d.id()))
}

/// Layer-shell window options for the popup: TOP | RIGHT, overlay, never
/// exclusive, no keyboard interactivity (mouse-driven, like `tray_menu`).
fn window_options(display_id: Option<DisplayId>, height: f32) -> WindowOptions {
    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(height)),
        })),
        app_id: Some("chronos-updates-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "updates-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((px(POPUP_MARGIN_TOP), px(POPUP_MARGIN_RIGHT), px(0.), px(0.))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the popup (idempotent — no-op if already open). Also fires a
/// `Refresh` so the list is current even if the last poll tick is stale
/// (mirrors `tray_menu::open`'s re-fetch-on-open).
pub fn open(cx: &mut App) {
    AppState::aur(cx).dispatch(AurCommand::Refresh);

    if cx.global::<UpdatesPopupState>().handle.is_some() {
        return;
    }

    let display_id = pick_display(cx);
    let count = AppState::aur(cx).get().count();
    let height = estimate_popup_height(count);
    match cx.open_window(window_options(display_id, height), |_, app_cx| {
        app_cx.new(|view_cx| UpdatesPopupView::new(view_cx))
    }) {
        Ok(new_handle) => {
            cx.global_mut::<UpdatesPopupState>().handle = Some(new_handle);
        }
        Err(err) => tracing::warn!("updates_popup: failed to open popup: {err}"),
    }
}

/// Close the popup (clears state + destroys the window). Safe to call from
/// contexts that do NOT already hold `&mut Window` for this popup (bar
/// widget click, external toggle) — uses `handle.update`.
pub fn close(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<UpdatesPopupState>().handle.take() {
        let _ = handle.update(cx, |_, window: &mut Window, _| window.remove_window());
    }
}

/// Close the popup from inside a callback that already holds `&mut Window`
/// for this popup's window-id (the in-popup "✕" / "Upgrade all" buttons). A
/// blind `close(cx)` would re-enter `handle.update` on the same id, which
/// silently fails while the callback is running (the window slot is empty
/// during dispatch) and leaves a ghost popup — see HANDOFF.md "СИСТЕМНЫЙ
/// БАГ: window.remove_window()". Clear the tracked handle and call
/// `remove_window()` on the live reference directly instead — the pattern
/// `launcher`/`tray_menu` already use.
pub(crate) fn close_this(window: &mut Window, cx: &mut App) {
    let this = window.window_handle();
    let tracked = cx
        .global::<UpdatesPopupState>()
        .handle
        .as_ref()
        .map(|h| **h == this)
        .unwrap_or(false);
    if tracked {
        cx.global_mut::<UpdatesPopupState>().handle.take(); // clear BEFORE remove
    }
    window.remove_window(); // direct, no reentrant handle.update
}

/// Toggle: click on the bar icon closes an open popup, opens a closed one.
/// Called from the bar widget's `on_click`, which holds `&mut Window` for
/// the BAR's window, not the popup's — so closing an already-open popup here
/// correctly goes through `close(cx)` (`handle.update`), not `close_this`.
pub fn toggle(_window: &mut Window, cx: &mut App) {
    let is_open = cx.global::<UpdatesPopupState>().handle.is_some();
    if is_open {
        close(cx);
    } else {
        open(cx);
    }
}

/// Dispatch "Upgrade all" and close the popup. Called from inside the
/// popup's own `on_click`, which already holds `&mut Window` for the
/// popup's window — closing MUST go through `close_this`, not `close(cx)`
/// (same reentrancy hazard as `tray_menu::click_item`).
pub(crate) fn upgrade_all(window: &mut Window, cx: &mut App) {
    AppState::aur(cx).dispatch(AurCommand::UpgradeAll);
    tracing::info!("updates_popup: dispatched UpgradeAll");
    close_this(window, cx);
}

/// Wire the updates popup to the live aur service. Called once from
/// `main.rs`.
pub fn init(cx: &mut App) {
    cx.set_global(UpdatesPopupState::default());

    let signal = AppState::aur(cx).subscribe();

    let watcher = cx.new(|cx| {
        state::watch(
            cx,
            signal,
            |_this: &mut UpdatesPopupWatcher,
             updates_state: UpdatesState,
             cx: &mut Context<UpdatesPopupWatcher>| {
                let handle = cx.global::<UpdatesPopupState>().handle.clone();
                if let Some(handle) = handle {
                    let height = estimate_popup_height(updates_state.count());
                    let _ = handle.update(cx, |_, window: &mut Window, _| {
                        window.resize(Size::new(px(POPUP_WIDTH), px(height)));
                    });
                    let _ = handle.update(cx, |_, _window, view_cx| view_cx.notify());
                }
            },
        );
        UpdatesPopupWatcher {}
    });

    cx.global_mut::<UpdatesPopupState>().watcher = Some(watcher);
    tracing::info!("updates_popup: subscribed to aur service");
}
