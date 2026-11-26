//! Native Windowed Modal Dialog Container
//! Menampilkan dialog modal bergaya jendela native Windows dengan titlebar tegas,
//! border kontras, area isi terstruktur, dan bar tombol aksi di bagian bawah.

use crate::ui::theme::Theme;
use gpui::{AnyElement, Div, FontWeight, div, prelude::*, px, rgb};

pub struct ModalDialog {
    pub title: &'static str,
    pub subtitle: Option<String>,
    pub width_px: f32,
    pub height_px: Option<f32>,
}

impl ModalDialog {
    pub fn new(title: &'static str, width_px: f32) -> Self {
        Self {
            title,
            subtitle: None,
            width_px,
            height_px: None,
        }
    }

    pub fn subtitle(mut self, sub: impl Into<String>) -> Self {
        self.subtitle = Some(sub.into());
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height_px = Some(h);
        self
    }

    pub fn frame(self, body: AnyElement, footer: Div) -> Div {
        div()
            .absolute()
            .inset_0()
            .bg(Theme::hsla_alpha(Theme::MODAL_BACKDROP, 0.50))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(self.width_px))
                    .when_some(self.height_px, |d, h| d.h(px(h)))
                    .bg(rgb(0xFFFFFF))
                    .border_1()
                    .border_color(rgb(Theme::MODAL_BORDER))
                    .rounded(px(4.0))
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Native Dialog Titlebar
                    .child(
                        div()
                            .h(px(34.0))
                            .bg(rgb(Theme::MODAL_HEADER_BG))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xFFFFFF))
                                            .child(self.title),
                                    )
                                    .when_some(self.subtitle, |d, sub| {
                                        d.child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(rgb(0x94A3B8))
                                                .child(format!("— {}", sub)),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(0x94A3B8))
                                    .child("[Esc] Tutup"),
                            ),
                    )
                    // Isi Form Dialog
                    .child(div().flex_1().p(px(14.0)).overflow_hidden().child(body))
                    // Footer Dialog
                    .child(footer),
            )
    }
}
