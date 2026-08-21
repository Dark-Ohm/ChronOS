//! Calendar popup view — hosts the kit `Calendar` from `gpui-component/time`.
//!
//! The window root MUST be a `gpui_component::Root` — component widgets panic
//! on `window.root()` otherwise (T263). The `Calendar` is configured with
//! today's date on init and lets the user pick a day / flip months.
//!
//! T329: the Calendar renders with the kit's own border/radius/padding but NO
//! background fill, so the wallpaper and any open panel bled straight through
//! the date grid. It is now wrapped in the same surface-card family as the Sound
//! popup (`volume_popup/view.rs`): `theme.surface_color` plate + elevation
//! shadow + backdrop blur. The Root stays transparent (like tray_menu /
//! start_menu) so the card's rounded corners show the desktop — the plate
//! lives on the card, not the Root.

use chrono::Local;
use gpui::{AppContext, Context, Entity, IntoElement, Render, Styled, Window, div, prelude::*, px};

use gpui_component::calendar::{Calendar, CalendarEvent, CalendarState, Date};

use chronos_ui::{
    Theme, WindowRootExt, elevation_apply_light_chrome, elevation_blur_layer,
};

use crate::calendar_popup::{POPUP_WIDTH, close_this};

/// Calendar popup view — hosts the kit Calendar widget on a Sound-family card.
pub struct CalendarPopupView {
    calendar: Entity<CalendarState>,
}

impl CalendarPopupView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let today = Local::now().naive_local().date();
        let calendar = cx.new(|cx| {
            let mut state = CalendarState::new(window, cx);
            state.set_date(Date::Single(Some(today)), window, cx);
            state
        });
        Self { calendar }
    }
}

impl Render for CalendarPopupView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *Theme::global(cx);
        let elev = theme.elevation_popup();
        let blur_layer = elevation_blur_layer(&elev, theme.radius_lg);

        // Same card family as the Sound popup (T329): frosted surface_color
        // plate + elevation shadow + backdrop blur. The kit Calendar keeps its
        // own padding but its border is dropped (`border_0`) — the card's
        // `border.subtle` is the single outer edge, like volume_popup.
        let card = div()
            .window_font(&theme)
            .relative()
            .flex_col()
            .w(px(POPUP_WIDTH))
            .rounded(theme.radius_lg)
            .bg(theme.surface_color(theme.bg.primary.alpha(0.82)))
            .border_1()
            .border_color(theme.border.subtle)
            .shadow(elev.shadows.to_vec())
            .child(blur_layer)
            .overflow_hidden()
            .child(Calendar::new(&self.calendar).border_0());
        elevation_apply_light_chrome(&elev, card)
    }
}
