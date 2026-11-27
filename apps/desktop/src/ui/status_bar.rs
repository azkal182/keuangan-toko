//! Native Desktop Status Bar Component (24px)
//! Menampilkan status operasi, halaman aktif, adapter data, dan waktu kalender usaha Asia/Jakarta.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

pub struct StatusBar {
    pub message: String,
    pub active_module: String,
    pub business_date: String,
    pub user_label: String,
    pub adapter_label: String,
}

impl StatusBar {
    pub fn new(
        message: impl Into<String>,
        active_module: impl Into<String>,
        business_date: impl Into<String>,
        user_label: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            active_module: active_module.into(),
            business_date: business_date.into(),
            user_label: user_label.into(),
            adapter_label: "—".into(),
        }
    }

    pub fn adapter(mut self, label: impl Into<String>) -> Self {
        self.adapter_label = label.into();
        self
    }

    pub fn render(self) -> Div {
        div()
            .h(px(24.0))
            .bg(rgb(Theme::BG_STATUS_BAR))
            .border_t_1()
            .border_color(rgb(0x0F172A))
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .text_size(px(10.5))
            .text_color(rgb(0xCBD5E1))
            // Left: Status message
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(8.0))
                            .text_color(rgb(0x4ADE80))
                            .child("●"),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xF8FAFC))
                            .child(self.message),
                    ),
            )
            // Right: Segmented native status cells
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .px(px(6.0))
                            .border_l_1()
                            .border_color(rgb(0x334155))
                            .text_color(rgb(0x93C5FD))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Modul: {}", self.active_module)),
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .border_l_1()
                            .border_color(rgb(0x334155))
                            .text_color(rgb(0xCBD5E1))
                            .child(format!("Adapter: {}", self.adapter_label)),
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .border_l_1()
                            .border_color(rgb(0x334155))
                            .text_color(rgb(0xCBD5E1))
                            .child(self.business_date),
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .border_l_1()
                            .border_color(rgb(0x334155))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .child(self.user_label),
                    ),
            )
    }
}
