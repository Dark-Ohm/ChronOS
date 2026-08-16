//! Calendar popup view — hosts the kit `Calendar` from `gpui-component/time`.
//!
//! The window root MUST be a `gpui_component::Root` — component widgets panic
//! on `window.root()` otherwise (T263). The `Calendar` is configured with
//! today's date on init and lets the user pick a day / flip months.

use chrono::Local;
use gpui::{AppContext, Context, Entity, Render, Window};

use gpui_component::calendar::{Calendar, CalendarEvent, CalendarState, Date};

use crate::calendar_popup::close_this;

/// Calendar popup view — just hosts the kit Calendar widget.
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        Calendar::new(&self.calendar)
    }
}
