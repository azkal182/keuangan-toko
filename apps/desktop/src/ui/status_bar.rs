//! Modern Enterprise Status Bar Component
//! Memberikan informasi status sistem, mode data, dan waktu usaha dengan visual minimalis dan profesional.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

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
            .h(px(26.0))
            .bg(rgb(Theme::BG_STATUS_BAR))
            .border_t_1()
            .border_color(rgb(Theme::BORDER_DEFAULT))
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .text_size(px(11.0))
            .text_color(rgb(Theme::TEXT_MUTED))
            // Left: Glowing status indicator + Pesan sistem
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(8.0))
                            .text_color(rgb(0x16A34A))
                            .child("●"),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(Theme::TEXT_SECONDARY))
                            .child(self.message),
                    ),
            )
            // Center: Badge mode data enterprise
            .child(
                div().flex().items_center().gap(px(6.0)).child(
                    div()
                        .px(px(6.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .bg(rgb(Theme::PRIMARY_LIGHT))
                        .border_1()
                        .border_color(rgb(Theme::PRIMARY_BORDER))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(Theme::PRIMARY))
                        .child("TokoStoreAdapter: Mock Data Engine"),
                ),
            )
            // Right: Tanggal & User
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(div().child(self.business_date))
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(Theme::TEXT_PRIMARY))
                            .child(self.user_label),
                    ),
            )
    }
}
