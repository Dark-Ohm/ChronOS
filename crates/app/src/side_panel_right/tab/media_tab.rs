//! Media tab — thin wrapper over the existing `render_mpris_card` (T305).
//!
//! Owns exactly one service subscription (`MprisState`, same pattern as the
//! System tab's mpris watch) and renders the shared card unchanged. No
//! backend logic here — `services::mpris` is not touched; this tab is the
//! panel's "now playing" surface (T320 moved it onto the rail).

use chronos_services::{MprisState, Service};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};


use crate::side_panel_right::mpris_card::render_mpris_card;
use crate::state::{self, AppState};

pub struct MediaTab {
    mpris: MprisState,
}

impl MediaTab {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mpris_signal = AppState::mpris(cx).subscribe();
        state::watch(cx, mpris_signal, |this: &mut Self, data: MprisState, cx| {
            this.mpris = data;
            cx.notify();
        });
        Self {
            mpris: AppState::mpris(cx).get(),
        }
    }
}

impl Render for MediaTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(12.))
            .child(render_mpris_card(&self.mpris, cx))
    }
}
