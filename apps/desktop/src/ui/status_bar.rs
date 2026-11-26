//! Native 3-Panel Status Bar Component
//! Memberikan informasi status sistem, tanggal usaha, dan pengguna aktif dengan estetika desktop klasik.

use crate::ui::theme::Theme;
use gpui::{Div, div, prelude::*, px, rgb};

pub struct StatusBar {
    pub message: String,
    pub business_date: String,
    pub user_label: String,
}

impl StatusBar {
    pub fn new(
        message: impl Into<String>,
        business_date: impl Into<String>,
        user_label: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            business_date: business_date.into(),
            user_label: user_label.into(),
        }
    }

    pub fn render(self) -> Div {
        div()
            .h(px(24.0))
            .bg(rgb(Theme::BG_STATUS_BAR))
            .border_t_1()
            .border_color(rgb(Theme::GRID_LINE))
            .flex()
            .items_center()
            .px(px(6.0))
            .gap(px(4.0))
            .text_size(px(11.0))
            .text_color(rgb(Theme::TEXT_SECONDARY))
            // Panel 1: Status pesan sistem (flex grow)
            .child(
                div()
                    .flex_1()
                    .h(px(18.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::BORDER_LIGHT))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .child(self.message),
            )
            // Panel 2: Tanggal Usaha WIB
            .child(
                div()
                    .w(px(190.0))
                    .h(px(18.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::BORDER_LIGHT))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .child(self.business_date),
            )
            // Panel 3: Pengguna Aktif & Role
            .child(
                div()
                    .w(px(220.0))
                    .h(px(18.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::BORDER_LIGHT))
                    .px(px(6.0))
                    .flex()
                    .items_center()
                    .child(self.user_label),
            )
    }
}
